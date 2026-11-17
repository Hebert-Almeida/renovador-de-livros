//! Ciclo de renovação (W2-01): política → gate → login → lista → sincronizar →
//! planejar → renovar e confirmar → gravar → logout → relatório.
//!
//! Erros antes de abrir a sessão no gate (conta inativa, política remota, cota,
//! breaker…) voltam como `Err`: nenhuma requisição foi feita. Depois disso, o
//! ciclo sempre devolve um [`CycleReport`]; problemas no meio do caminho
//! (senha errada, CAPTCHA, 429…) ficam em `report.error`, com o que já foi
//! gravado preservado.

mod db;
pub mod notes;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;
use zeroize::Zeroizing;

use crate::clock::Clock;
use crate::guard::{GuardError, RemotePolicy, RequestGate, SessionTicket, policy_fetch_due};
pub use crate::model::CycleReport;
use crate::model::*;
use crate::planner::{self, PlanInput};
use crate::sophia::{LoggedSession, LoginResult, RemoteLoan, RenewOutcome, SophiaClient};
use crate::store::Store;
use crate::{Error, Result};

/// Fonte da política remota (`policy.json` + assinatura). Deve devolver a
/// política **já verificada** (ver `RemotePolicy::verify_and_parse`).
#[async_trait]
pub trait PolicySource: Send + Sync {
    async fn fetch(&self) -> Result<RemotePolicy>;
}

pub struct Engine {
    pub gate: Arc<dyn RequestGate>,
    pub client: Arc<dyn SophiaClient>,
    pub clock: Arc<dyn Clock>,
    /// Consultada no máximo 1×/dia; sem fonte, vale a última política em cache.
    pub policy_source: Option<Arc<dyn PolicySource>>,
}

pub type ProgressFn<'a> = &'a (dyn Fn(CycleProgress) + Send + Sync);

type EmitFn<'a> = &'a (dyn Fn(CycleStep, &str, Option<u32>, Option<u32>) + Send + Sync);

/// Notificação que o app deve exibir após o ciclo (seção 5.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Notification {
    Renewed {
        loan_id: Id,
        title: String,
        new_due_date: Option<NaiveDate>,
    },
    /// Falha sem nova tentativa (reserva, limite, bloqueio…).
    Failed {
        loan_id: Id,
        title: String,
        result: RenewalResult,
        due_date: NaiveDate,
        message: Option<String>,
    },
    /// Vence hoje e não há mais janela para renovar (ex.: app aberto após 21h).
    DueToday {
        loan_id: Id,
        title: String,
        due_date: NaiveDate,
    },
    /// Lembrete agendado que venceu.
    Reminder {
        loan_id: Id,
        title: String,
        due_date: NaiveDate,
    },
    InvalidPassword {
        account_id: Id,
    },
    AutomationPaused {
        account_id: Id,
        reason: String,
    },
}

/// Uma tentativa de renovação; `retry = true` quando a falha foi de rede e
/// uma nova tentativa foi agendada (`planner::retry_after_failure`).
#[derive(Debug, Clone, Serialize)]
pub struct RenewAttempt {
    pub event: RenewalEvent,
    pub retry: bool,
    pub retry_at: Option<DateTime<Utc>>,
}

/// Resultado completo do ciclo: o relatório (contrato IPC) e o que o app
/// precisa fazer em seguida (notificar).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CycleOutcome {
    pub report: CycleReport,
    pub attempts: Vec<RenewAttempt>,
    pub notifications: Vec<Notification>,
}

impl CycleOutcome {
    fn empty(account_id: Id) -> Self {
        Self {
            report: CycleReport {
                account_id,
                ..CycleReport::default()
            },
            ..Self::default()
        }
    }

    /// `true` se algum livro vence hoje sem chance de renovação automática.
    pub fn needs_due_today_notification(&self) -> bool {
        self.notifications
            .iter()
            .any(|n| matches!(n, Notification::DueToday { .. }))
    }
}

/// O cliente devolve 429/503 como `Error::Network` com esta mensagem.
fn is_rate_limited(e: &Error) -> bool {
    match e {
        Error::Network(m) => m.starts_with("o servidor pediu para aguardar"),
        Error::Guard(GuardError::RetryAfter { .. }) => true,
        _ => false,
    }
}

/// Erro que encerra a sessão (nenhuma requisição a mais).
fn is_fatal(e: &Error) -> bool {
    is_rate_limited(e) || !matches!(e, Error::Network(_))
}

fn classify_error(e: &Error) -> Option<RenewalResult> {
    if is_rate_limited(e) {
        return Some(RenewalResult::RateLimited);
    }
    match e {
        Error::Network(_) => Some(RenewalResult::NetworkError),
        Error::Parse(_) | Error::NotImplemented(_) | Error::InvalidLibrary(_) => {
            Some(RenewalResult::ParseError)
        }
        Error::HumanVerificationRequired => Some(RenewalResult::Captcha),
        // Gate recusou (cota, breaker): nada foi enviado, nada a registrar.
        _ => None,
    }
}

fn outcome_result(o: &RenewOutcome) -> (RenewalResult, Option<String>) {
    match o {
        RenewOutcome::Success { .. } => (RenewalResult::Success, None),
        RenewOutcome::Reserved { message } => (RenewalResult::Reserved, Some(message.clone())),
        RenewOutcome::LimitReached { message } => {
            (RenewalResult::LimitReached, Some(message.clone()))
        }
        RenewOutcome::Blocked { message } => (RenewalResult::Blocked, Some(message.clone())),
        RenewOutcome::Overdue { message } => (RenewalResult::Overdue, Some(message.clone())),
    }
}

/// Estado mutável de um ciclo em andamento.
struct Run<'a> {
    store: &'a Store,
    account: Account,
    trigger: Trigger,
    now: DateTime<Utc>,
    seed: u64,
    out: CycleOutcome,
}

impl Engine {
    pub fn new(
        gate: Arc<dyn RequestGate>,
        client: Arc<dyn SophiaClient>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            gate,
            client,
            clock,
            policy_source: None,
        }
    }

    pub fn with_policy_source(mut self, source: Arc<dyn PolicySource>) -> Self {
        self.policy_source = Some(source);
        self
    }

    /// Executa um ciclo e devolve só o relatório (contrato do IPC).
    pub async fn run_cycle(
        &self,
        store: &Store,
        account_id: Id,
        kind: CycleKind,
        trigger: Trigger,
        progress: ProgressFn<'_>,
    ) -> Result<CycleReport> {
        self.run_cycle_outcome(store, account_id, kind, trigger, None, progress)
            .await
            .map(|o| o.report)
    }

    /// Executa um ciclo completo. `password` é usado no modo "sem salvar senha";
    /// sem ele, vale a senha salva.
    pub async fn run_cycle_outcome(
        &self,
        store: &Store,
        account_id: Id,
        kind: CycleKind,
        trigger: Trigger,
        password: Option<&str>,
        progress: ProgressFn<'_>,
    ) -> Result<CycleOutcome> {
        let account = store.account_get(account_id)?;
        let emit = |step: CycleStep, message: &str, current: Option<u32>, total: Option<u32>| {
            progress(CycleProgress {
                account_id,
                step,
                message: message.to_owned(),
                current,
                total,
            })
        };
        emit(CycleStep::Waiting, "verificando limites", None, None);

        if account.status != AccountStatus::Active {
            return Err(Error::Invalid(format!(
                "conta não está ativa ({})",
                account.status.as_str()
            )));
        }
        let password: Zeroizing<String> = match password {
            Some(p) => Zeroizing::new(p.to_owned()),
            None => store
                .account_password(account_id)?
                .ok_or_else(|| Error::Invalid("a conta não tem senha salva".into()))?,
        };

        // 1. Política remota e gate.
        self.check_policy(store, &account, trigger).await?;
        let seed = db::install_seed(store)?;
        let ticket = self
            .gate
            .begin_session(account.id, &account.library_base_url, trigger)
            .await?;

        let mut run = Run {
            store,
            now: self.clock.now(),
            seed,
            out: CycleOutcome::empty(account_id),
            account,
            trigger,
        };

        if let Err(e) = self
            .session(&mut run, &ticket, kind, &password, &emit)
            .await
        {
            match &e {
                Error::Store(_) => {
                    self.gate.end_session(ticket);
                    return Err(e);
                }
                Error::InvalidCredentials => {
                    run.out
                        .notifications
                        .push(Notification::InvalidPassword { account_id });
                }
                Error::HumanVerificationRequired | Error::Guard(GuardError::BreakerOpen { .. }) => {
                    run.out.notifications.push(Notification::AutomationPaused {
                        account_id,
                        reason: e.to_string(),
                    });
                }
                _ => {}
            }
            tracing::warn!("ciclo da conta {account_id} interrompido: {e}");
            run.out.report.error = Some(e.to_string());
        }
        run.out.report.requests_made = db::requests_in_session(store, &ticket.session_id)?;
        self.gate.end_session(ticket);
        emit(CycleStep::Done, "ciclo concluído", None, None);
        Ok(run.out)
    }

    /// Passo 1: consulta a política remota (no máx. 1×/dia) e aplica o que
    /// cabe ao motor; o gate aplica o restante.
    async fn check_policy(&self, store: &Store, account: &Account, trigger: Trigger) -> Result<()> {
        let now = self.clock.now();
        let (mut policy, fetched_at) = db::cached_policy(store)?;
        if let Some(src) = &self.policy_source
            && policy_fetch_due(fetched_at, now)
        {
            match src.fetch().await {
                Ok(p) => {
                    db::save_policy(store, &p, now)?;
                    policy = p;
                }
                // Sem política nova, vale a última conhecida (que só aperta).
                Err(e) => tracing::warn!("política remota indisponível: {e}"),
            }
        }
        let limits = policy.effective();
        if matches!(trigger, Trigger::Auto | Trigger::CatchUp) && !limits.automation_enabled {
            return Err(Error::Guard(GuardError::RemoteDisabled));
        }
        let host = url::Url::parse(&account.library_base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
            .unwrap_or_default();
        if limits.blocked_hosts.contains(&host) {
            return Err(Error::Guard(GuardError::HostNotAllowed { host }));
        }
        Ok(())
    }

    /// Passos 2 a 7, dentro da sessão aberta no gate.
    async fn session(
        &self,
        run: &mut Run<'_>,
        ticket: &SessionTicket,
        kind: CycleKind,
        password: &str,
        emit: EmitFn<'_>,
    ) -> Result<()> {
        let acc = run.account.clone();

        // 2. Login e lista.
        emit(CycleStep::Login, "entrando na biblioteca", None, None);
        let session = match self
            .client
            .login(ticket, &acc.library_base_url, &acc.login, password)
            .await?
        {
            LoginResult::Ok(s) => s,
            LoginResult::InvalidCredentials { message } => {
                tracing::info!("login recusado: {message}");
                run.store.account_update(
                    acc.id,
                    &AccountPatch {
                        status: Some(AccountStatus::InvalidPassword),
                        ..AccountPatch::default()
                    },
                )?;
                // Conta pausada até editar a senha: cancela a agenda do sistema.
                run.account.status = AccountStatus::InvalidPassword;
                self.replan(run)?;
                return Err(Error::InvalidCredentials);
            }
        };
        emit(CycleStep::List, "lendo empréstimos", None, None);
        let remote = self.client.list_loans(ticket, &session).await?;

        // 3. Sincronizar o banco.
        let views = self.sync(run, &remote)?;
        run.out.report.loans_seen = remote.len() as u32;

        // 4. O planner decide o que renovar.
        let mut fatal = None;
        if kind == CycleKind::RenewDue {
            let due: Vec<LoanView> = planner::due_for_renewal(run.now, &acc, &views)
                .into_iter()
                .cloned()
                .collect();
            let remote_by_key: HashMap<&str, &RemoteLoan> =
                remote.iter().map(|r| (r.item_key.as_str(), r)).collect();
            let total = due.len() as u32;

            // 5. Renovar cada livro e confirmar com novo GET da lista.
            for (i, lv) in due.iter().enumerate() {
                emit(
                    CycleStep::Renew,
                    &format!("renovando {}", lv.book.title),
                    Some(i as u32 + 1),
                    Some(total),
                );
                let Some(r) = remote_by_key.get(lv.book.item_key.as_str()) else {
                    continue;
                };
                if let Err(e) = self.renew_one(run, ticket, &session, lv, r, emit).await {
                    fatal = Some(e);
                    break;
                }
            }
        }

        // 6. Gravar agendamentos (os eventos já foram gravados livro a livro).
        emit(CycleStep::Save, "salvando", None, None);
        self.finish_schedules(run)?;
        self.replan(run)?;

        // 7. Logout (falha aqui não invalida o ciclo).
        if fatal.is_none()
            && let Err(e) = self.client.logout(ticket, &session).await
        {
            tracing::warn!("logout falhou: {e}");
        }
        fatal.map_or(Ok(()), Err)
    }

    /// Upsert de livros e empréstimos; empréstimos locais que sumiram da lista
    /// são marcados como devolvidos.
    fn sync(&self, run: &mut Run<'_>, remote: &[RemoteLoan]) -> Result<Vec<LoanView>> {
        let store = run.store;
        let acc_id = run.account.id;
        let today = planner::local_date(run.now);
        let filter = LoanFilter {
            account_id: Some(acc_id),
            ..LoanFilter::default()
        };
        let local = store.loan_list(&filter)?;
        let open: HashMap<&str, &LoanView> = local
            .iter()
            .filter(|lv| lv.loan.status != LoanStatus::Returned)
            .map(|lv| (lv.book.item_key.as_str(), lv))
            .collect();

        let mut seen = HashSet::new();
        for r in remote {
            seen.insert(r.item_key.as_str());
            let book = store.book_upsert(acc_id, &r.item_key, &r.title)?;
            let prev = open.get(r.item_key.as_str()).map(|lv| &lv.loan);
            let status = if r.due_date < today {
                LoanStatus::Overdue
            } else {
                LoanStatus::Active
            };
            store.loan_upsert(&Loan {
                id: prev.map_or(0, |l| l.id),
                book_id: book.id,
                loaned_at: prev.and_then(|l| l.loaned_at),
                due_date: r.due_date,
                renewals_count: r
                    .renewals_count
                    .or(prev.map(|l| l.renewals_count))
                    .unwrap_or(0),
                renewals_limit: prev.and_then(|l| l.renewals_limit),
                status,
                auto_renew: prev.is_none_or(|l| l.auto_renew),
                returned_at: None,
                updated_at: run.now,
            })?;
        }
        for (key, lv) in &open {
            if !seen.contains(key) {
                store.loan_mark_returned(lv.loan.id)?;
                run.out.report.returned_detected += 1;
            }
        }
        db::set_last_sync(store, acc_id, run.now)?;
        run.account.last_sync_at = Some(run.now);
        store.loan_list(&filter)
    }

    /// Renova um livro, confirma relendo a lista e grava o `renewal_event`.
    /// `Err` = erro que encerra a sessão (já registrado no evento, se cabível).
    async fn renew_one(
        &self,
        run: &mut Run<'_>,
        ticket: &SessionTicket,
        session: &LoggedSession,
        lv: &LoanView,
        remote: &RemoteLoan,
        emit: EmitFn<'_>,
    ) -> Result<()> {
        let old_due = lv.loan.due_date;
        let mut new_due = None;
        let mut fatal = None;
        let (result, message) = if remote.renew_url.is_none() {
            (
                RenewalResult::Blocked,
                Some("renovação indisponível no site".to_owned()),
            )
        } else {
            match self.client.renew(ticket, session, remote).await {
                Ok(RenewOutcome::Success { new_due_date }) => {
                    emit(CycleStep::Confirm, "confirmando nova data", None, None);
                    match self.client.list_loans(ticket, session).await {
                        Ok(list) => match list.iter().find(|r| r.item_key == remote.item_key) {
                            Some(r) if r.due_date > old_due => {
                                new_due = Some(r.due_date);
                                self.update_loan(run, lv, r)?;
                                (RenewalResult::Success, None)
                            }
                            _ => (
                                RenewalResult::ParseError,
                                Some("renovação não confirmada na lista".to_owned()),
                            ),
                        },
                        Err(e) => {
                            // O site aceitou; só a releitura falhou.
                            new_due = new_due_date;
                            let msg = format!("renovação não confirmada: {e}");
                            if is_fatal(&e) {
                                fatal = Some(e);
                            }
                            (RenewalResult::Success, Some(msg))
                        }
                    }
                }
                Ok(other) => outcome_result(&other),
                Err(e) => {
                    let Some(result) = classify_error(&e) else {
                        return Err(e);
                    };
                    let msg = e.to_string();
                    if is_fatal(&e) {
                        fatal = Some(e);
                    }
                    (result, Some(msg))
                }
            }
        };

        let event = run.store.event_insert(&RenewalEvent {
            id: 0,
            loan_id: lv.loan.id,
            attempted_at: run.now,
            trigger: run.trigger,
            result,
            old_due_date: Some(old_due),
            new_due_date: new_due,
            message: message.clone(),
        })?;

        let retry_at =
            planner::retry_after_failure(result, old_due, run.now, run.seed, run.account.id);
        if let Some(at) = retry_at {
            run.store.schedule_insert(&Schedule {
                id: 0,
                account_id: run.account.id,
                loan_id: Some(lv.loan.id),
                kind: ScheduleKind::Renew,
                run_at: at,
                status: ScheduleStatus::Pending,
                origin: ScheduleOrigin::System,
                updated_at: run.now,
            })?;
        }

        let failed = Notification::Failed {
            loan_id: lv.loan.id,
            title: lv.book.title.clone(),
            result,
            due_date: old_due,
            message,
        };
        match result {
            RenewalResult::Success => {
                db::set_last_renewal(run.store, run.account.id, run.now)?;
                run.out.notifications.push(Notification::Renewed {
                    loan_id: lv.loan.id,
                    title: lv.book.title.clone(),
                    new_due_date: new_due,
                });
                run.out.report.renewed.push(event.clone());
            }
            RenewalResult::Reserved
            | RenewalResult::LimitReached
            | RenewalResult::Blocked
            | RenewalResult::Overdue => {
                // "Não tenta de novo": desliga a renovação automática do
                // empréstimo, o que faz o planner gerar lembretes D−2/D0.
                run.store.loan_set_auto_renew(lv.loan.id, false)?;
                run.out.notifications.push(failed);
                run.out.report.failed.push(event.clone());
            }
            _ => {
                if retry_at.is_none() {
                    run.out.notifications.push(failed);
                }
                run.out.report.failed.push(event.clone());
            }
        }
        run.out.attempts.push(RenewAttempt {
            event,
            retry: retry_at.is_some(),
            retry_at,
        });
        fatal.map_or(Ok(()), Err)
    }

    fn update_loan(&self, run: &Run<'_>, lv: &LoanView, r: &RemoteLoan) -> Result<()> {
        let mut loan = lv.loan.clone();
        loan.due_date = r.due_date;
        loan.renewals_count = r.renewals_count.unwrap_or(loan.renewals_count + 1);
        loan.status = LoanStatus::Active;
        run.store.loan_upsert(&loan).map(|_| ())
    }

    /// Fecha os agendamentos vencidos que este ciclo executou: sincronização,
    /// renovações tentadas e lembretes (que viram notificação).
    fn finish_schedules(&self, run: &mut Run<'_>) -> Result<()> {
        let store = run.store;
        let pending = store.schedule_list(&ScheduleFilter {
            account_id: Some(run.account.id),
            status: Some(ScheduleStatus::Pending),
            ..ScheduleFilter::default()
        })?;
        let attempts: HashMap<Id, RenewalResult> = run
            .out
            .attempts
            .iter()
            .map(|a| (a.event.loan_id, a.event.result))
            .collect();
        let synced = run.account.last_sync_at == Some(run.now);
        let loans: HashMap<Id, LoanView> = store
            .loan_list(&LoanFilter {
                account_id: Some(run.account.id),
                ..LoanFilter::default()
            })?
            .into_iter()
            .map(|lv| (lv.loan.id, lv))
            .collect();
        let mut reminders = Vec::new();
        for s in pending.iter().filter(|s| s.run_at <= run.now) {
            let status = match (s.kind, s.loan_id) {
                (ScheduleKind::Sync, _) if synced => ScheduleStatus::Done,
                (ScheduleKind::Renew, Some(loan_id)) => match attempts.get(&loan_id) {
                    Some(RenewalResult::Success) => ScheduleStatus::Done,
                    Some(_) => ScheduleStatus::Failed,
                    None => continue,
                },
                (ScheduleKind::Remind, Some(loan_id)) => {
                    if let Some(lv) = loans.get(&loan_id)
                        && lv.loan.status != LoanStatus::Returned
                    {
                        reminders.push(Notification::Reminder {
                            loan_id,
                            title: lv.book.title.clone(),
                            due_date: lv.loan.due_date,
                        });
                    }
                    ScheduleStatus::Done
                }
                _ => continue,
            };
            store.schedule_set_status(s.id, status, s.origin)?;
        }
        run.out.notifications.extend(reminders);
        Ok(())
    }

    /// Recalcula a agenda da conta com o planner e grava as mudanças.
    fn replan(&self, run: &mut Run<'_>) -> Result<()> {
        let store = run.store;
        let acc_id = run.account.id;
        let loans = store.loan_list(&LoanFilter {
            account_id: Some(acc_id),
            ..LoanFilter::default()
        })?;
        let existing = store.schedule_list(&ScheduleFilter {
            account_id: Some(acc_id),
            ..ScheduleFilter::default()
        })?;
        let plan = planner::plan(&PlanInput {
            now: run.now,
            account: &run.account,
            loans: &loans,
            existing: &existing,
            install_seed: run.seed,
        })?;
        for id in plan.cancel {
            store.schedule_set_status(id, ScheduleStatus::Cancelled, ScheduleOrigin::System)?;
        }
        let today = planner::local_date(run.now);
        let by_id: HashMap<Id, &LoanView> = loans.iter().map(|lv| (lv.loan.id, lv)).collect();
        for s in &plan.create {
            store.schedule_insert(s)?;
            // Vence hoje e o lembrete só sairia amanhã: avisa agora.
            if s.kind == ScheduleKind::Remind
                && let Some(lv) = s.loan_id.and_then(|id| by_id.get(&id))
                && lv.loan.due_date == today
                && planner::local_date(s.run_at) > today
            {
                run.out.notifications.push(Notification::DueToday {
                    loan_id: lv.loan.id,
                    title: lv.book.title.clone(),
                    due_date: lv.loan.due_date,
                });
            }
        }
        let next = store
            .schedule_list(&ScheduleFilter {
                account_id: Some(acc_id),
                kind: Some(ScheduleKind::Renew),
                status: Some(ScheduleStatus::Pending),
            })?
            .into_iter()
            .map(|s| s.run_at)
            .min();
        db::set_next_renewal(store, acc_id, next)
    }

    /// Execução em segundo plano: para cada conta ativa com agendamentos
    /// vencidos, roda o ciclo adequado. Sem tarefa vencida, não usa a rede.
    pub async fn run_due(&self, store: &Store) -> Result<Vec<CycleOutcome>> {
        let now = self.clock.now();
        let mut outcomes = Vec::new();
        for acc in store.account_list()? {
            if acc.status != AccountStatus::Active || !acc.has_password {
                continue;
            }
            let due: Vec<Schedule> = store
                .schedule_list(&ScheduleFilter {
                    account_id: Some(acc.id),
                    status: Some(ScheduleStatus::Pending),
                    ..ScheduleFilter::default()
                })?
                .into_iter()
                .filter(|s| s.run_at <= now)
                .collect();
            if due.is_empty() {
                continue;
            }
            if due.iter().all(|s| s.kind == ScheduleKind::Remind) {
                // Só lembretes: notifica sem abrir sessão.
                let mut run = Run {
                    store,
                    now,
                    seed: db::install_seed(store)?,
                    out: CycleOutcome::empty(acc.id),
                    account: acc,
                    trigger: Trigger::Auto,
                };
                self.finish_schedules(&mut run)?;
                outcomes.push(run.out);
                continue;
            }
            let kind = if due.iter().any(|s| s.kind == ScheduleKind::Renew) {
                CycleKind::RenewDue
            } else {
                CycleKind::Sync
            };
            let oldest = due.iter().map(|s| s.run_at).min().unwrap_or(now);
            let trigger = if now - oldest > Duration::hours(1) {
                Trigger::CatchUp
            } else {
                Trigger::Auto
            };
            match self
                .run_cycle_outcome(store, acc.id, kind, trigger, None, &|_| {})
                .await
            {
                Ok(o) => outcomes.push(o),
                Err(e @ Error::Store(_)) => return Err(e),
                // Gate recusou (janela, cota, breaker): tenta na próxima vez.
                Err(e) => {
                    let mut o = CycleOutcome::empty(acc.id);
                    o.report.error = Some(e.to_string());
                    outcomes.push(o);
                }
            }
        }
        Ok(outcomes)
    }
}
