//! Implementação padrão de [`RequestGate`] (W1-01).
//!
//! Convenção de persistência: o início de cada sessão é gravado em
//! `request_log` como um registro com `kind = "session_start:<trigger>"` (mesma convenção de `store::session_start_kind`)
//! (ver [`session_kind`]). É assim que `count_sessions`,
//! `last_session_start` e `distinct_logins_since` sabem o que contar, inclusive
//! depois de reiniciar o app.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveTime, Utc};
use chrono_tz::America::Sao_Paulo;
use chrono_tz::Tz;
use tokio::sync::Semaphore;

use super::limits;
use super::{
    BreakerState, GuardError, GuardPersistence, Permit, RemotePolicy, RequestGate, RequestOutcome,
    RequestRecord, SessionTicket,
};
use crate::clock::Clock;
use crate::model::{Id, QuotaStatus, Trigger};

const SESSION_KIND_PREFIX: &str = "session_start:";
/// `kind` das requisições de login (contadas em `distinct_logins_since`).
pub const LOGIN_KIND: &str = "login";
/// Motivo gravado no breaker quando o servidor pediu para aguardar (429/503).
const REASON_RETRY_AFTER: &str = "retry_after";
const REASON_FAILURES: &str = "consecutive_failures";
const REASON_FAILURES_LONG: &str = "consecutive_failures_long";
const REASON_HUMAN: &str = "human_verification";
const REASON_INVALID_CREDENTIALS: &str = "invalid_credentials";
/// Pausa "até ação do usuário": longa o bastante para nunca expirar sozinha.
const UNTIL_USER_ACTION_DAYS: i64 = 365 * 100;

/// `kind` do registro que marca o início de uma sessão.
pub fn session_kind(trigger: Trigger) -> String {
    format!("{SESSION_KIND_PREFIX}{}", trigger.as_str())
}

/// Inverso de [`session_kind`]: `Some(trigger)` se o `kind` marca início de sessão.
pub fn session_trigger(kind: &str) -> Option<Trigger> {
    kind.strip_prefix(SESSION_KIND_PREFIX)?.parse().ok()
}

/// Escopo do breaker de uma conta.
pub fn account_scope(account_id: Id) -> String {
    format!("account:{account_id}")
}

/// Escopo do breaker de um host.
pub fn host_scope(host: &str) -> String {
    format!("host:{}", host.to_ascii_lowercase())
}

/// Valida o endereço da biblioteca (só HTTPS, com host) e devolve o host
/// normalizado (minúsculo).
pub fn validate_library_url(base: &str) -> Result<String, GuardError> {
    let not_allowed = || GuardError::HostNotAllowed {
        host: base.to_owned(),
    };
    let url = url::Url::parse(base.trim()).map_err(|_| not_allowed())?;
    if limits::REQUIRE_HTTPS && url.scheme() != "https" {
        return Err(not_allowed());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(not_allowed());
    }
    url.host_str()
        .filter(|h| !h.is_empty())
        .map(|h| h.to_ascii_lowercase())
        .ok_or_else(not_allowed)
}

/// O valor do cabeçalho `x-powered-by` identifica um servidor Sophia?
pub fn is_sophia_powered_by(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        v.split(',')
            .any(|p| p.trim().eq_ignore_ascii_case(limits::SOPHIA_POWERED_BY))
    })
}

/// Pode cadastrar mais uma conta, dado o número de contas existentes?
pub fn check_account_count(existing: u32) -> Result<(), GuardError> {
    if existing >= limits::MAX_ACCOUNTS {
        Err(GuardError::AccountLimit)
    } else {
        Ok(())
    }
}

/// Espera assíncrona injetável (testes avançam o `FakeClock`).
#[async_trait]
pub trait Sleeper: Send + Sync {
    async fn sleep(&self, d: StdDuration);
}

/// [`Sleeper`] real, via `tokio::time::sleep`.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, d: StdDuration) {
        tokio::time::sleep(d).await;
    }
}

#[derive(Debug, Default)]
struct SessionState {
    requests: u32,
    /// Erro que encerrou a sessão (CAPTCHA, 429…): nenhuma nova requisição.
    aborted: Option<GuardError>,
}

#[derive(Debug, Default)]
struct GateState {
    /// Instante a partir do qual a próxima requisição pode sair.
    next_request_at: Option<DateTime<Utc>>,
    sessions: HashMap<String, SessionState>,
}

/// Implementação padrão (W1-01).
pub struct DefaultGate {
    pub persistence: Arc<dyn GuardPersistence>,
    pub clock: Arc<dyn Clock>,
    pub policy: RemotePolicy,
    sleeper: Arc<dyn Sleeper>,
    semaphore: Arc<Semaphore>,
    state: Mutex<GateState>,
}

impl DefaultGate {
    pub fn new(persistence: Arc<dyn GuardPersistence>, clock: Arc<dyn Clock>) -> Self {
        Self {
            persistence,
            clock,
            policy: RemotePolicy::default(),
            sleeper: Arc::new(TokioSleeper),
            semaphore: Arc::new(Semaphore::new(limits::MAX_CONCURRENT_REQUESTS)),
            state: Mutex::new(GateState::default()),
        }
    }

    /// Troca a forma de esperar (testes usam um sleeper que avança o relógio).
    pub fn with_sleeper(mut self, sleeper: Arc<dyn Sleeper>) -> Self {
        self.sleeper = sleeper;
        self
    }

    /// Aplica uma política remota já verificada. Ela é combinada com a atual
    /// pelo merge "só aperta"; para substituir, atribua `policy` diretamente.
    pub fn with_policy(mut self, remote: &RemotePolicy) -> Self {
        self.policy = self.policy.merge(remote);
        self
    }

    /// Retoma uma conta pausada por senha inválida ou verificação humana
    /// (chamar quando o usuário editar a senha ou confirmar a ação manual).
    pub fn resume_account(&self, account_id: Id) -> crate::Result<()> {
        self.persistence
            .set_breaker(&account_scope(account_id), &BreakerState::default())
    }

    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    fn check_breaker(&self, scope: &str, now: DateTime<Utc>) -> Result<(), GuardError> {
        let st = self.persistence.breaker(scope).map_err(fail_closed)?;
        match st.open_until {
            Some(until) if until > now => {
                if st.reason.as_deref() == Some(REASON_RETRY_AFTER) {
                    Err(GuardError::RetryAfter { until })
                } else {
                    Err(GuardError::BreakerOpen { until })
                }
            }
            _ => Ok(()),
        }
    }

    fn check_breakers(&self, account_id: Id, host: &str) -> Result<(), GuardError> {
        let now = self.now();
        self.check_breaker(&account_scope(account_id), now)?;
        self.check_breaker(&host_scope(host), now)
    }

    fn update_breaker(&self, scope: &str, f: impl FnOnce(&mut BreakerState)) {
        let mut st = match self.persistence.breaker(scope) {
            Ok(st) => st,
            Err(e) => {
                tracing::error!("guard: falha ao ler breaker {scope}: {e}");
                BreakerState::default()
            }
        };
        f(&mut st);
        if let Err(e) = self.persistence.set_breaker(scope, &st) {
            tracing::error!("guard: falha ao gravar breaker {scope}: {e}");
        }
    }

    fn abort_session(&self, session_id: &str, err: GuardError) {
        let mut state = self.state.lock().unwrap();
        if let Some(s) = state.sessions.get_mut(session_id) {
            s.aborted = Some(err);
        }
    }
}

/// Erro de persistência no gate: na dúvida, bloqueia.
fn fail_closed(e: crate::Error) -> GuardError {
    tracing::error!("guard: erro de persistência, bloqueando: {e}");
    GuardError::QuotaExhausted { retry_at: None }
}

fn to_chrono(d: StdDuration) -> Duration {
    Duration::from_std(d).unwrap_or(Duration::MAX)
}

fn local_midnight_utc(date: chrono::NaiveDate, tz: Tz) -> DateTime<Utc> {
    date.and_time(NaiveTime::MIN)
        .and_local_timezone(tz)
        .earliest()
        // Meia-noite inexistente (horário de verão): 01h é o início do dia.
        .or_else(|| {
            date.and_hms_opt(1, 0, 0)
                .and_then(|t| t.and_local_timezone(tz).earliest())
        })
        .expect("início do dia local")
        .with_timezone(&Utc)
}

/// Início do dia civil (America/Sao_Paulo) que contém `now`.
fn day_start(now: DateTime<Utc>) -> DateTime<Utc> {
    local_midnight_utc(now.with_timezone(&Sao_Paulo).date_naive(), Sao_Paulo)
}

/// Início do próximo dia civil (America/Sao_Paulo).
fn next_day_start(now: DateTime<Utc>) -> DateTime<Utc> {
    let date = now.with_timezone(&Sao_Paulo).date_naive();
    local_midnight_utc(date.succ_opt().expect("data válida"), Sao_Paulo)
}

/// `Some(próxima abertura)` se `now` está fora da janela automática.
fn outside_window(now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let local = now.with_timezone(&Sao_Paulo);
    let hour = chrono::Timelike::hour(&local);
    if (limits::AUTO_WINDOW_START_HOUR..limits::AUTO_WINDOW_END_HOUR).contains(&hour) {
        return None;
    }
    let date = if hour < limits::AUTO_WINDOW_START_HOUR {
        local.date_naive()
    } else {
        local.date_naive().succ_opt().expect("data válida")
    };
    let at = date
        .and_hms_opt(limits::AUTO_WINDOW_START_HOUR, 0, 0)
        .and_then(|t| t.and_local_timezone(Sao_Paulo).earliest())
        .expect("abertura da janela")
        .with_timezone(&Utc);
    Some(at)
}

fn new_session_id() -> String {
    let mut b = [0u8; 16];
    rand::fill(&mut b[..]);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn jitter() -> StdDuration {
    let max = limits::MAX_REQUEST_JITTER.as_millis() as u64;
    StdDuration::from_millis(rand::random_range(0..=max))
}

fn is_auto(trigger: Trigger) -> bool {
    matches!(trigger, Trigger::Auto | Trigger::CatchUp)
}

#[async_trait]
impl RequestGate for DefaultGate {
    async fn begin_session(
        &self,
        account_id: Id,
        host: &str,
        trigger: Trigger,
    ) -> Result<SessionTicket, GuardError> {
        let limits = self.policy.effective();
        if is_auto(trigger) && !limits.automation_enabled {
            return Err(GuardError::RemoteDisabled);
        }
        let host = validate_library_url(host)?;
        if limits.blocked_hosts.contains(&host) {
            return Err(GuardError::HostNotAllowed { host });
        }

        // Verificação + registro atômicos dentro desta instância.
        let mut state = self.state.lock().unwrap();
        let now = self.now();
        self.check_breakers(account_id, &host)?;
        if is_auto(trigger)
            && let Some(next_window) = outside_window(now)
        {
            return Err(GuardError::OutsideWindow { next_window });
        }

        let p = &self.persistence;
        let since = day_start(now);
        let tomorrow = next_day_start(now);
        let exhausted = GuardError::QuotaExhausted {
            retry_at: Some(tomorrow),
        };

        let install = p.count_sessions(None, None, since).map_err(fail_closed)?;
        if install >= limits.max_sessions_per_install_day {
            return Err(exhausted);
        }

        let account_today = p
            .count_sessions(Some(account_id), None, since)
            .map_err(fail_closed)?;
        if account_today == 0
            && p.distinct_logins_since(since).map_err(fail_closed)?
                >= limits::MAX_DISTINCT_LOGINS_PER_DAY
        {
            return Err(GuardError::AccountLimit);
        }

        if is_auto(trigger) {
            let auto = p
                .count_sessions(Some(account_id), Some(Trigger::Auto), since)
                .map_err(fail_closed)?
                + p.count_sessions(Some(account_id), Some(Trigger::CatchUp), since)
                    .map_err(fail_closed)?;
            if auto >= limits.max_auto_sessions_per_account_day {
                return Err(exhausted);
            }
        } else {
            let manual = p
                .count_sessions(Some(account_id), Some(Trigger::Manual), since)
                .map_err(fail_closed)?;
            if manual >= limits.max_manual_sessions_per_account_day {
                return Err(exhausted);
            }
            if let Some(last) = p
                .last_session_start(account_id, Trigger::Manual)
                .map_err(fail_closed)?
            {
                let retry_at = last + to_chrono(limits::MIN_MANUAL_SESSION_GAP);
                if retry_at > now {
                    return Err(GuardError::TooSoon { retry_at });
                }
            }
        }

        let session_id = new_session_id();
        p.record_request(&RequestRecord {
            account_id: Some(account_id),
            host: host.clone(),
            at: now,
            kind: session_kind(trigger),
            session_id: session_id.clone(),
            http_status: None,
            outcome: RequestOutcome::Ok,
        })
        .map_err(fail_closed)?;
        state
            .sessions
            .insert(session_id.clone(), SessionState::default());

        Ok(SessionTicket {
            session_id,
            account_id,
            host,
            trigger,
        })
    }

    async fn acquire(&self, ticket: &SessionTicket, kind: &str) -> Result<Permit, GuardError> {
        let limits = self.policy.effective();
        let check = |state: &GateState| -> Result<u32, GuardError> {
            let s = state
                .sessions
                .get(&ticket.session_id)
                .ok_or(GuardError::SessionBudgetExhausted)?;
            if let Some(err) = &s.aborted {
                return Err(err.clone());
            }
            if s.requests >= limits.max_requests_per_session {
                return Err(GuardError::SessionBudgetExhausted);
            }
            Ok(s.requests)
        };
        check(&self.state.lock().unwrap())?;
        self.check_breakers(ticket.account_id, &ticket.host)?;

        let slot = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semáforo do gate nunca é fechado");

        // Espaçamento mínimo + jitter desde o fim da requisição anterior.
        let target = self.state.lock().unwrap().next_request_at;
        if let Some(target) = target {
            loop {
                let now = self.now();
                if now >= target {
                    break;
                }
                let wait = (target - now).to_std().unwrap_or_default();
                self.sleeper.sleep(wait).await;
            }
        }

        // O estado pode ter mudado durante a espera (ex.: breaker aberto).
        let index = {
            let mut state = self.state.lock().unwrap();
            let index = check(&state)?;
            self.check_breakers(ticket.account_id, &ticket.host)?;
            let now = self.now();
            state.next_request_at = Some(now + to_chrono(limits.min_request_interval + jitter()));
            state
                .sessions
                .get_mut(&ticket.session_id)
                .expect("sessão verificada")
                .requests += 1;
            index
        };

        Ok(Permit {
            session_id: ticket.session_id.clone(),
            request_index: index,
            account_id: ticket.account_id,
            host: ticket.host.clone(),
            kind: kind.to_owned(),
            _slot: Some(slot),
        })
    }

    fn report(
        &self,
        permit: Permit,
        http_status: Option<u16>,
        outcome: RequestOutcome,
        retry_after: Option<StdDuration>,
    ) {
        let now = self.now();
        let limits = self.policy.effective();
        {
            // O intervalo conta a partir do fim da requisição.
            let mut state = self.state.lock().unwrap();
            let next = now + to_chrono(limits.min_request_interval + jitter());
            state.next_request_at = Some(state.next_request_at.map_or(next, |n| n.max(next)));
        }
        if let Err(e) = self.persistence.record_request(&RequestRecord {
            account_id: Some(permit.account_id),
            host: permit.host.clone(),
            at: now,
            kind: permit.kind.clone(),
            session_id: permit.session_id.clone(),
            http_status,
            outcome,
        }) {
            tracing::error!("guard: falha ao gravar request_log: {e}");
        }

        let account = account_scope(permit.account_id);
        let host = host_scope(&permit.host);
        match outcome {
            // Login bem-sucedido não prova que o HTML de dados ainda é
            // reconhecível: só uma resposta OK de outro tipo zera a contagem.
            RequestOutcome::Ok if permit.kind == LOGIN_KIND => {}
            RequestOutcome::Ok => {
                for scope in [&account, &host] {
                    self.update_breaker(scope, |st| {
                        if st.consecutive_failures > 0 {
                            st.consecutive_failures = 0;
                        }
                    });
                }
            }
            RequestOutcome::NetworkError
            | RequestOutcome::ServerError
            | RequestOutcome::UnexpectedHtml => {
                let mut opened = None;
                for scope in [&account, &host] {
                    self.update_breaker(scope, |st| {
                        st.consecutive_failures += 1;
                        let pause = if st.consecutive_failures >= limits::BREAKER_LONG_THRESHOLD {
                            Some((limits::BREAKER_LONG_PAUSE, REASON_FAILURES_LONG))
                        } else if st.consecutive_failures == limits::BREAKER_SHORT_THRESHOLD {
                            Some((limits::BREAKER_SHORT_PAUSE, REASON_FAILURES))
                        } else {
                            None
                        };
                        if let Some((d, reason)) = pause {
                            let until = now + to_chrono(d);
                            if st.open_until.is_none_or(|u| u < until) {
                                st.open_until = Some(until);
                                st.reason = Some(reason.to_owned());
                            }
                            opened = Some(until);
                        }
                    });
                }
                if let Some(until) = opened {
                    self.abort_session(&permit.session_id, GuardError::BreakerOpen { until });
                }
            }
            RequestOutcome::RateLimited => {
                let wait = retry_after
                    .unwrap_or(limits::MIN_RETRY_AFTER)
                    .max(limits::MIN_RETRY_AFTER);
                let until = now + to_chrono(wait);
                self.update_breaker(&host, |st| {
                    if st.open_until.is_none_or(|u| u < until) {
                        st.open_until = Some(until);
                        st.reason = Some(REASON_RETRY_AFTER.to_owned());
                    }
                });
                self.abort_session(&permit.session_id, GuardError::RetryAfter { until });
            }
            RequestOutcome::HumanVerification | RequestOutcome::InvalidCredentials => {
                let reason = if outcome == RequestOutcome::HumanVerification {
                    REASON_HUMAN
                } else {
                    REASON_INVALID_CREDENTIALS
                };
                let until = now + Duration::days(UNTIL_USER_ACTION_DAYS);
                self.update_breaker(&account, |st| {
                    st.open_until = Some(until);
                    st.reason = Some(reason.to_owned());
                });
                self.abort_session(&permit.session_id, GuardError::BreakerOpen { until });
            }
        }
        // `permit` sai de escopo aqui e devolve o semáforo.
        drop(permit);
    }

    fn end_session(&self, ticket: SessionTicket) {
        self.state
            .lock()
            .unwrap()
            .sessions
            .remove(&ticket.session_id);
    }

    fn quota(&self, account_id: Id) -> crate::Result<QuotaStatus> {
        let limits = self.policy.effective();
        let p = &self.persistence;
        let now = self.now();
        let since = day_start(now);
        let manual = p.count_sessions(Some(account_id), Some(Trigger::Manual), since)?;
        let auto = p.count_sessions(Some(account_id), Some(Trigger::Auto), since)?
            + p.count_sessions(Some(account_id), Some(Trigger::CatchUp), since)?;
        let install = p.count_sessions(None, None, since)?;
        let manual_left = limits
            .max_manual_sessions_per_account_day
            .saturating_sub(manual);
        let install_left = limits.max_sessions_per_install_day.saturating_sub(install);

        let breaker = p.breaker(&account_scope(account_id))?;
        let breaker_open_until = breaker.open_until.filter(|u| *u > now);

        let mut next_allowed_at = if manual_left == 0 || install_left == 0 {
            Some(next_day_start(now))
        } else {
            p.last_session_start(account_id, Trigger::Manual)?
                .map(|last| last + to_chrono(limits::MIN_MANUAL_SESSION_GAP))
                .filter(|t| *t > now)
        };
        if let Some(b) = breaker_open_until {
            next_allowed_at = Some(next_allowed_at.map_or(b, |n| n.max(b)));
        }

        Ok(QuotaStatus {
            account_id,
            manual_sessions_left: manual_left.min(install_left),
            auto_sessions_left: limits
                .max_auto_sessions_per_account_day
                .saturating_sub(auto)
                .min(install_left),
            install_sessions_left: install_left,
            next_allowed_at,
            breaker_open_until,
            reason: breaker_open_until.and(breaker.reason),
        })
    }
}
