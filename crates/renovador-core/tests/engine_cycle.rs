//! Testes do ciclo completo (W2-01) com wiremock, `DefaultGate` real sobre o
//! `Store`, `FakeClock` e parsers falsos (o parser real é da W1-06).
//! Nunca acessa o site real do Sophia.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use renovador_core::clock::{Clock, FakeClock};
use renovador_core::engine::{CycleOutcome, Engine, Notification, PolicySource};
use renovador_core::guard::{
    DefaultGate, GuardError, GuardPersistence, Permit, RemotePolicy, RequestGate, RequestOutcome,
    SessionTicket, Sleeper, account_scope, host_scope,
};
use renovador_core::model::*;
use renovador_core::sophia::client::{ClientConfig, HttpSophiaClient, Parsers, ReqwestTransport};
use renovador_core::sophia::{HttpResponse, LoginResponse, RemoteLoan, RenewOutcome, Transport};
use renovador_core::store::Store;
use renovador_core::{Error, keys};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------- parsers falsos sobre HTML sintético ----------

fn attr<'a>(chunk: &'a str, name: &str) -> Option<&'a str> {
    let start = chunk.find(&format!("{name}=\""))? + name.len() + 2;
    let len = chunk[start..].find('"')?;
    Some(&chunk[start..start + len])
}

fn fake_login(body: &str) -> renovador_core::Result<LoginResponse> {
    serde_json::from_str(body).map_err(|e| Error::Parse(e.to_string()))
}

/// `<ul id="emprestimos"><li key=".." title=".." due="YYYY-MM-DD" n="0" renew=".."></li></ul>`
fn fake_list(body: &str) -> renovador_core::Result<Vec<RemoteLoan>> {
    if !body.contains("<ul id=\"emprestimos\">") {
        return Err(Error::Parse("estrutura inesperada".into()));
    }
    body.split("<li ")
        .skip(1)
        .map(|c| {
            let due = attr(c, "due").ok_or_else(|| Error::Parse("due".into()))?;
            Ok(RemoteLoan {
                item_key: attr(c, "key").unwrap_or_default().into(),
                title: attr(c, "title").unwrap_or_default().into(),
                due_date: due.parse().map_err(|_| Error::Parse("data".into()))?,
                renewals_count: attr(c, "n").and_then(|n| n.parse().ok()),
                renew_url: attr(c, "renew").map(str::to_owned),
            })
        })
        .collect()
}

fn fake_renewal(body: &str) -> renovador_core::Result<RenewOutcome> {
    if body.contains("RENOVADO") {
        Ok(RenewOutcome::Success { new_due_date: None })
    } else if body.contains("RESERVA") {
        Ok(RenewOutcome::Reserved {
            message: "reservado".into(),
        })
    } else if body.contains("LIMITE") {
        Ok(RenewOutcome::LimitReached {
            message: "limite".into(),
        })
    } else {
        Err(Error::Parse("renovação".into()))
    }
}

fn list_html(items: &[(&str, &str, &str)]) -> String {
    let lis: String = items
        .iter()
        .map(|(key, title, due)| {
            format!(
                "<li key=\"{key}\" title=\"{title}\" due=\"{due}\" n=\"0\" \
                 renew=\"renovar.asp?id={key}\"></li>"
            )
        })
        .collect();
    format!("<html><ul id=\"emprestimos\">{lis}</ul></html>")
}

// ---------- gate real adaptado ao wiremock ----------

/// `DefaultGate` exige HTTPS e normaliza o host sem porta; o wiremock usa
/// `http://127.0.0.1:<porta>`. Este adaptador só traduz o endereço: cotas,
/// espaçamento e breaker continuam sendo os do gate real.
struct PortGate {
    inner: DefaultGate,
}

impl PortGate {
    fn to_inner(t: &SessionTicket) -> SessionTicket {
        SessionTicket {
            host: t.host.split(':').next().unwrap_or_default().to_owned(),
            ..t.clone()
        }
    }
}

#[async_trait]
impl RequestGate for PortGate {
    async fn begin_session(
        &self,
        account_id: Id,
        base: &str,
        trigger: Trigger,
    ) -> Result<SessionTicket, GuardError> {
        let url = url::Url::parse(base).unwrap();
        let https = format!("https://{}{}", url.host_str().unwrap(), url.path());
        let mut t = self
            .inner
            .begin_session(account_id, &https, trigger)
            .await?;
        if let Some(port) = url.port() {
            t.host = format!("{}:{port}", t.host);
        }
        Ok(t)
    }
    async fn acquire(&self, ticket: &SessionTicket, kind: &str) -> Result<Permit, GuardError> {
        self.inner.acquire(&Self::to_inner(ticket), kind).await
    }
    fn report(
        &self,
        permit: Permit,
        status: Option<u16>,
        outcome: RequestOutcome,
        retry_after: Option<StdDuration>,
    ) {
        self.inner.report(permit, status, outcome, retry_after)
    }
    fn end_session(&self, ticket: SessionTicket) {
        self.inner.end_session(Self::to_inner(&ticket))
    }
    fn quota(&self, account_id: Id) -> renovador_core::Result<QuotaStatus> {
        self.inner.quota(account_id)
    }
}

/// Sleeper que só avança o relógio falso e anota as esperas.
struct FakeSleeper {
    clock: Arc<FakeClock>,
    waits: Mutex<Vec<StdDuration>>,
}

#[async_trait]
impl Sleeper for FakeSleeper {
    async fn sleep(&self, d: StdDuration) {
        self.waits.lock().unwrap().push(d);
        self.clock.advance(Duration::from_std(d).unwrap());
    }
}

/// Transporte que pode simular queda de rede na renovação.
struct FlakyTransport {
    inner: ReqwestTransport,
    fail_renew: AtomicBool,
}

#[async_trait]
impl Transport for FlakyTransport {
    async fn get(&self, url: &str) -> renovador_core::Result<HttpResponse> {
        if self.fail_renew.load(Ordering::SeqCst) && url.contains("renovar.asp") {
            return Err(Error::Network("conexão recusada".into()));
        }
        self.inner.get(url).await
    }
    async fn post_form(
        &self,
        url: &str,
        form: &[(&str, &str)],
    ) -> renovador_core::Result<HttpResponse> {
        self.inner.post_form(url, form).await
    }
}

struct StaticPolicy(RemotePolicy);

#[async_trait]
impl PolicySource for StaticPolicy {
    async fn fetch(&self) -> renovador_core::Result<RemotePolicy> {
        Ok(self.0.clone())
    }
}

// ---------- ambiente ----------

/// 10h em São Paulo (13h UTC) de 16/11/2026, dentro da janela.
fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 16, 13, 0, 0).unwrap()
}

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
}

struct Env {
    _dir: tempfile::TempDir,
    store: Arc<Store>,
    clock: Arc<FakeClock>,
    sleeper: Arc<FakeSleeper>,
    transport: Arc<FlakyTransport>,
    server: MockServer,
    account: Account,
}

impl Env {
    async fn new() -> Self {
        Self::at(start()).await
    }

    async fn at(now: DateTime<Utc>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&dir.path().join("t.db"), &keys::generate_key()).unwrap());
        let server = MockServer::start().await;
        let account = store
            .account_create(&NewAccount {
                library_base_url: format!("{}/5362", server.uri()),
                library_name: "Teste".into(),
                login: "aluno".into(),
                password: Some("senha".into()),
                mode: AccountMode::Auto,
                renew_days_before: 1,
                remind_days_before: 2,
            })
            .unwrap();
        let clock = Arc::new(FakeClock::new(now));
        Self {
            _dir: dir,
            store,
            sleeper: Arc::new(FakeSleeper {
                clock: clock.clone(),
                waits: Mutex::new(Vec::new()),
            }),
            clock,
            transport: Arc::new(FlakyTransport {
                inner: ReqwestTransport::new().unwrap(),
                fail_renew: AtomicBool::new(false),
            }),
            server,
            account,
        }
    }

    /// Engine novo sobre o mesmo banco (equivale a reiniciar o app).
    fn engine(&self) -> Engine {
        let gate: Arc<dyn RequestGate> = Arc::new(PortGate {
            inner: DefaultGate::new(self.store.clone(), self.clock.clone())
                .with_sleeper(self.sleeper.clone()),
        });
        let client = HttpSophiaClient::new(
            self.transport.clone(),
            gate.clone(),
            ClientConfig {
                require_https: false,
                logout_path: None,
                parsers: Parsers {
                    login: fake_login,
                    loan_list: fake_list,
                    renewal: fake_renewal,
                },
            },
        );
        Engine::new(gate, Arc::new(client), self.clock.clone())
    }

    async fn run(&self, kind: CycleKind, trigger: Trigger) -> renovador_core::Result<CycleOutcome> {
        self.engine()
            .run_cycle_outcome(&self.store, self.account.id, kind, trigger, None, &|_| {})
            .await
    }

    async fn renew(&self) -> CycleOutcome {
        self.run(CycleKind::RenewDue, Trigger::Manual)
            .await
            .unwrap()
    }

    async fn requests(&self) -> usize {
        self.server.received_requests().await.unwrap().len()
    }

    fn host(&self) -> String {
        "127.0.0.1".into()
    }

    fn loans(&self) -> Vec<LoanView> {
        self.store
            .loan_list(&LoanFilter {
                account_id: Some(self.account.id),
                ..LoanFilter::default()
            })
            .unwrap()
    }

    fn loan(&self, key: &str) -> LoanView {
        self.loans()
            .into_iter()
            .find(|lv| lv.book.item_key == key)
            .unwrap()
    }

    fn events(&self) -> Vec<RenewalEvent> {
        self.store
            .history_list(
                &HistoryFilter::default(),
                Page {
                    offset: 0,
                    limit: 100,
                },
            )
            .unwrap()
            .items
    }

    fn schedules(&self, kind: ScheduleKind, status: ScheduleStatus) -> Vec<Schedule> {
        self.store
            .schedule_list(&ScheduleFilter {
                account_id: Some(self.account.id),
                kind: Some(kind),
                status: Some(status),
            })
            .unwrap()
    }

    /// Requisições HTTP gravadas no `request_log` (sem marcadores de sessão),
    /// em ordem cronológica.
    fn http_log(&self) -> Vec<RequestLogEntry> {
        let mut v: Vec<_> = self
            .store
            .requests_recent(1000)
            .unwrap()
            .into_iter()
            .filter(|r| !r.kind.starts_with("session_start:"))
            .collect();
        v.reverse();
        v
    }

    fn assert_spacing(&self) {
        let log = self.http_log();
        for w in log.windows(2) {
            assert!(
                w[1].at - w[0].at >= Duration::seconds(2),
                "requisições com menos de 2 s: {:?} → {:?}",
                w[0].at,
                w[1].at
            );
        }
        for d in self.sleeper.waits.lock().unwrap().iter() {
            assert!(
                *d <= StdDuration::from_secs(3),
                "espera longa demais: {d:?}"
            );
        }
    }

    // --- mocks ---

    async fn mount_login_page(&self) {
        Mock::given(method("GET"))
            .and(path("/5362/mobile/login.asp"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-powered-by", "Sophia")
                    .insert_header("set-cookie", "TESTE=1; Path=/")
                    .set_body_string("<form id=\"login\"></form>"),
            )
            .mount(&self.server)
            .await;
    }

    async fn mount_login_ok(&self) {
        self.mount_login_page().await;
        Mock::given(method("POST"))
            .and(path("/5362/mobile/ajx_efetua_login.asp"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"login_valido":"true","mensagem":"","url":"lista.asp?x=1","abrir_popup":false}"#,
            ))
            .mount(&self.server)
            .await;
    }

    /// Primeira leitura da lista devolve `first`; as seguintes, `then`.
    async fn mount_list(&self, first: String, then: Option<String>) {
        self.mount_list_n(first, 1, then).await;
    }

    /// As `n` primeiras leituras devolvem `first`; as seguintes, `then`.
    async fn mount_list_n(&self, first: String, n: u64, then: Option<String>) {
        let then_body = then.unwrap_or_else(|| first.clone());
        Mock::given(method("GET"))
            .and(path("/5362/mobile/lista.asp"))
            .respond_with(ResponseTemplate::new(200).set_body_string(first))
            .up_to_n_times(n)
            .with_priority(1)
            .mount(&self.server)
            .await;
        Mock::given(method("GET"))
            .and(path("/5362/mobile/lista.asp"))
            .respond_with(ResponseTemplate::new(200).set_body_string(then_body))
            .mount(&self.server)
            .await;
    }

    async fn mount_renew(&self, resp: ResponseTemplate) {
        Mock::given(method("GET"))
            .and(path("/5362/mobile/renovar.asp"))
            .respond_with(resp)
            .mount(&self.server)
            .await;
    }

    /// Login ok + um livro (vence amanhã, renova hoje) + resposta de renovação.
    async fn standard(&self, renew: ResponseTemplate, after: &str) {
        self.mount_login_ok().await;
        self.mount_list(
            list_html(&[("k1", "Livro A", "2026-11-17")]),
            Some(list_html(&[("k1", "Livro A", after)])),
        )
        .await;
        self.mount_renew(renew).await;
    }

    fn breaker(&self, scope: &str) -> renovador_core::guard::BreakerState {
        self.store.breaker(scope).unwrap()
    }
}

fn body(s: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_string(s.to_owned())
}

// ---------- cenários da seção 10 ----------

#[tokio::test]
async fn success_renews_confirms_and_schedules_next() {
    let env = Env::new().await;
    env.standard(body("RENOVADO"), "2026-11-24").await;

    let out = env.renew().await;
    let r = &out.report;
    assert_eq!(r.error, None);
    assert_eq!(r.loans_seen, 1);
    assert_eq!(r.renewed.len(), 1);
    assert!(r.failed.is_empty());
    // login_page, login, lista, renovação, lista de confirmação.
    assert_eq!(r.requests_made, 5);
    assert_eq!(env.requests().await, 5);
    env.assert_spacing();

    let lv = env.loan("k1");
    assert_eq!(lv.loan.due_date, day(24));
    assert_eq!(lv.loan.status, LoanStatus::Active);
    let ev = env.events();
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].result, RenewalResult::Success);
    assert_eq!(ev[0].trigger, Trigger::Manual);
    assert_eq!(ev[0].old_due_date, Some(day(17)));
    assert_eq!(ev[0].new_due_date, Some(day(24)));

    // Próxima renovação agendada para D−1 do novo vencimento.
    let renews = env.schedules(ScheduleKind::Renew, ScheduleStatus::Pending);
    assert_eq!(renews.len(), 1);
    assert_eq!(
        renovador_core::planner::local_date(renews[0].run_at),
        day(23)
    );
    assert_eq!(
        env.schedules(ScheduleKind::Sync, ScheduleStatus::Pending)
            .len(),
        1
    );
    let acc = env.store.account_get(env.account.id).unwrap();
    assert!(acc.last_sync_at.is_some());
    assert!(acc.last_renewal_at.is_some());
    assert_eq!(acc.next_renewal_at, Some(renews[0].run_at));
    assert!(out.notifications.iter().any(|n| matches!(
        n,
        Notification::Renewed { new_due_date: Some(d), .. } if *d == day(24)
    )));
    assert_eq!(
        env.breaker(&account_scope(env.account.id))
            .consecutive_failures,
        0
    );
}

#[tokio::test]
async fn wrong_password_pauses_account() {
    let env = Env::new().await;
    env.mount_login_page().await;
    Mock::given(method("POST"))
        .and(path("/5362/mobile/ajx_efetua_login.asp"))
        .respond_with(body(
            r#"{"login_valido":"false","mensagem":"Senha inválida","url":"","abrir_popup":false}"#,
        ))
        .mount(&env.server)
        .await;

    let out = env.renew().await;
    assert!(out.report.error.is_some());
    assert_eq!(out.report.requests_made, 2);
    assert_eq!(env.requests().await, 2);
    assert!(out.notifications.contains(&Notification::InvalidPassword {
        account_id: env.account.id
    }));
    let acc = env.store.account_get(env.account.id).unwrap();
    assert_eq!(acc.status, AccountStatus::InvalidPassword);
    let br = env.breaker(&account_scope(env.account.id));
    assert!(
        br.open_until
            .is_some_and(|u| u > env.clock.now() + Duration::days(30))
    );

    // Nova tentativa não usa a rede.
    assert!(env.renew_err().await);
    assert_eq!(env.requests().await, 2);
}

impl Env {
    async fn renew_err(&self) -> bool {
        self.clock.advance(Duration::minutes(31));
        self.run(CycleKind::RenewDue, Trigger::Manual)
            .await
            .is_err()
    }
}

async fn refused(result_body: &str, expected: RenewalResult) {
    let env = Env::new().await;
    env.standard(body(result_body), "2026-11-17").await;
    let out = env.renew().await;
    assert_eq!(out.report.error, None);
    // Sem confirmação: login_page, login, lista, renovação.
    assert_eq!(out.report.requests_made, 4);
    assert_eq!(env.requests().await, 4);
    env.assert_spacing();
    assert_eq!(out.report.failed.len(), 1);
    assert_eq!(out.report.failed[0].result, expected);
    assert!(!out.attempts[0].retry);
    let lv = env.loan("k1");
    assert_eq!(lv.loan.due_date, day(17));
    assert!(!lv.loan.auto_renew, "não tenta de novo");
    assert!(
        env.schedules(ScheduleKind::Renew, ScheduleStatus::Pending)
            .is_empty()
    );
    // Lembrete no dia do vencimento.
    let reminders = env.schedules(ScheduleKind::Remind, ScheduleStatus::Pending);
    assert!(
        reminders
            .iter()
            .any(|s| renovador_core::planner::local_date(s.run_at) == day(17))
    );
    assert!(out.notifications.iter().any(|n| matches!(
        n,
        Notification::Failed { result, due_date, .. } if *result == expected && *due_date == day(17)
    )));
}

#[tokio::test]
async fn reserved_is_not_retried() {
    refused("RESERVA", RenewalResult::Reserved).await;
}

#[tokio::test]
async fn limit_reached_is_not_retried() {
    refused("LIMITE", RenewalResult::LimitReached).await;
}

#[tokio::test]
async fn rate_limited_429_respects_retry_after() {
    let env = Env::new().await;
    env.standard(
        ResponseTemplate::new(429).insert_header("retry-after", "7200"),
        "2026-11-17",
    )
    .await;
    let out = env.renew().await;
    assert!(out.report.error.is_some());
    assert_eq!(out.report.requests_made, 4);
    assert_eq!(env.requests().await, 4);
    assert_eq!(out.report.failed[0].result, RenewalResult::RateLimited);
    let br = env.breaker(&host_scope(&env.host()));
    assert!(br.open_until.unwrap() >= env.clock.now() + Duration::minutes(119));

    // Antes do Retry-After, nenhuma sessão nova.
    assert!(env.renew_err().await);
    assert_eq!(env.requests().await, 4);
}

#[tokio::test]
async fn server_503_opens_host_breaker_for_at_least_one_hour() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/lista.asp"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&env.server)
        .await;
    let out = env.renew().await;
    assert!(out.report.error.is_some());
    assert_eq!(out.report.requests_made, 3);
    assert_eq!(env.requests().await, 3);
    assert!(env.loans().is_empty());
    let br = env.breaker(&host_scope(&env.host()));
    assert!(br.open_until.unwrap() >= env.clock.now() + Duration::minutes(59));
    assert!(env.renew_err().await);
    assert_eq!(env.requests().await, 3);
}

#[tokio::test]
async fn captcha_stops_immediately() {
    let env = Env::new().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/login.asp"))
        .respond_with(body("<div class=\"g-recaptcha\"></div>"))
        .mount(&env.server)
        .await;
    let out = env.renew().await;
    assert!(out.report.error.is_some());
    assert_eq!(out.report.requests_made, 1);
    assert_eq!(env.requests().await, 1);
    assert!(
        out.notifications
            .iter()
            .any(|n| matches!(n, Notification::AutomationPaused { .. }))
    );
    let br = env.breaker(&account_scope(env.account.id));
    assert_eq!(br.reason.as_deref(), Some("human_verification"));
    assert!(env.renew_err().await);
    assert_eq!(env.requests().await, 1);
}

#[tokio::test]
async fn captcha_on_renewal_records_event_and_stops() {
    let env = Env::new().await;
    env.standard(body("<div class=\"h-captcha\"></div>"), "2026-11-17")
        .await;
    let out = env.renew().await;
    assert_eq!(out.report.requests_made, 4);
    assert_eq!(out.report.failed[0].result, RenewalResult::Captcha);
    assert_eq!(env.events()[0].result, RenewalResult::Captcha);
}

#[tokio::test]
async fn cloudflare_challenge_stops_immediately() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/lista.asp"))
        .respond_with(ResponseTemplate::new(403).set_body_string("<title>Just a moment...</title>"))
        .mount(&env.server)
        .await;
    let out = env.renew().await;
    assert!(out.report.error.is_some());
    assert_eq!(out.report.requests_made, 3);
    assert_eq!(env.requests().await, 3);
    assert!(env.loans().is_empty());
    assert!(env.renew_err().await);
    assert_eq!(env.requests().await, 3);
}

#[tokio::test]
async fn changed_html_aborts_and_is_counted() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/lista.asp"))
        .respond_with(body("<html><div>layout novo</div></html>"))
        .mount(&env.server)
        .await;
    let out = env.renew().await;
    assert!(matches!(out.report.error.as_deref(), Some(e) if e.contains("inesperada")));
    assert_eq!(out.report.requests_made, 3);
    assert_eq!(env.requests().await, 3);
    assert_eq!(
        env.breaker(&account_scope(env.account.id))
            .consecutive_failures,
        1
    );
    assert!(env.loans().is_empty());
    assert!(env.events().is_empty());
}

#[tokio::test]
async fn consecutive_failures_open_breaker() {
    let env = Env::new().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/login.asp"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&env.server)
        .await;
    // 3 falhas seguidas (sessões manuais com 30 min entre elas) → pausa de 6 h.
    for i in 1..=3u32 {
        let out = env.renew().await;
        assert!(out.report.error.is_some());
        assert_eq!(out.report.requests_made, 1);
        assert_eq!(
            env.breaker(&account_scope(env.account.id))
                .consecutive_failures,
            i
        );
        env.clock.advance(Duration::minutes(31));
    }
    let br = env.breaker(&account_scope(env.account.id));
    assert!(br.open_until.unwrap() > env.clock.now() + Duration::hours(4));
    assert_eq!(env.requests().await, 3);
    assert!(env.renew_err().await);
    assert_eq!(env.requests().await, 3);
}

#[tokio::test]
async fn non_sophia_host_redirect_aborts() {
    let env = Env::new().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/login.asp"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://evil.example/login"),
        )
        .mount(&env.server)
        .await;
    let out = env.renew().await;
    assert!(out.report.error.is_some());
    assert_eq!(out.report.requests_made, 1);
    assert_eq!(env.requests().await, 1);
    assert_eq!(
        env.breaker(&account_scope(env.account.id))
            .consecutive_failures,
        1
    );
}

// ---------- outros comportamentos ----------

#[tokio::test]
async fn network_failure_schedules_retry() {
    let env = Env::new().await;
    env.standard(body("RENOVADO"), "2026-11-24").await;
    env.transport.fail_renew.store(true, Ordering::SeqCst);
    let out = env.renew().await;
    assert_eq!(out.report.error, None);
    assert_eq!(out.report.failed[0].result, RenewalResult::NetworkError);
    assert!(out.attempts[0].retry);
    let at = out.attempts[0].retry_at.unwrap();
    // Nova tentativa no dia do vencimento.
    assert_eq!(renovador_core::planner::local_date(at), day(17));
    let renews = env.schedules(ScheduleKind::Renew, ScheduleStatus::Pending);
    assert!(renews.iter().any(|s| s.run_at == at));
    assert!(env.loan("k1").loan.auto_renew);
    // Sem notificação de falha: ainda vai tentar.
    assert!(
        !out.notifications
            .iter()
            .any(|n| matches!(n, Notification::Failed { .. }))
    );
}

#[tokio::test]
async fn unconfirmed_renewal_is_not_success() {
    let env = Env::new().await;
    env.standard(body("RENOVADO"), "2026-11-17").await;
    let out = env.renew().await;
    assert_eq!(out.report.requests_made, 5);
    assert!(out.report.renewed.is_empty());
    assert_eq!(out.report.failed[0].result, RenewalResult::ParseError);
    assert_eq!(env.loan("k1").loan.due_date, day(17));
}

#[tokio::test]
async fn sync_detects_returned_loans_without_renewing() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    env.mount_list(
        list_html(&[
            ("k1", "Livro A", "2026-11-17"),
            ("k2", "Livro B", "2026-11-30"),
        ]),
        Some(list_html(&[("k2", "Livro B", "2026-11-30")])),
    )
    .await;
    let out = env.run(CycleKind::Sync, Trigger::Manual).await.unwrap();
    assert_eq!(out.report.loans_seen, 2);
    assert_eq!(out.report.requests_made, 3);
    assert!(env.events().is_empty());
    // Renovação de k1 agendada para hoje (catch-up).
    assert_eq!(
        env.schedules(ScheduleKind::Renew, ScheduleStatus::Pending)
            .len(),
        2
    );

    env.clock.advance(Duration::minutes(31));
    let out = env.run(CycleKind::Sync, Trigger::Manual).await.unwrap();
    assert_eq!(out.report.returned_detected, 1);
    assert_eq!(env.loan("k1").loan.status, LoanStatus::Returned);
    assert_eq!(env.loan("k2").loan.status, LoanStatus::Active);
    // O agendamento do livro devolvido foi cancelado.
    let renews = env.schedules(ScheduleKind::Renew, ScheduleStatus::Pending);
    assert_eq!(renews.len(), 1);
    assert_eq!(renews[0].loan_id, Some(env.loan("k2").loan.id));
    assert_eq!(env.requests().await, 6);
}

#[tokio::test]
async fn due_today_after_21h_flags_notification() {
    // 21h30 em São Paulo de 16/11; o livro vence hoje.
    let env = Env::at(Utc.with_ymd_and_hms(2026, 11, 17, 0, 30, 0).unwrap()).await;
    env.mount_login_ok().await;
    env.mount_list(list_html(&[("k1", "Livro A", "2026-11-16")]), None)
        .await;
    let out = env.run(CycleKind::RenewDue, Trigger::Manual).await.unwrap();
    assert!(out.report.renewed.is_empty(), "fora da janela não renova");
    assert!(out.needs_due_today_notification());
}

#[tokio::test]
async fn remote_policy_kill_switch_blocks_automatic_cycles() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    let engine = env
        .engine()
        .with_policy_source(Arc::new(StaticPolicy(RemotePolicy {
            automation_enabled: Some(false),
            ..RemotePolicy::default()
        })));
    let err = engine
        .run_cycle(
            &env.store,
            env.account.id,
            CycleKind::RenewDue,
            Trigger::Auto,
            &|_| {},
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Guard(GuardError::RemoteDisabled)));
    assert_eq!(env.requests().await, 0);
    // A política fica em cache: sem fonte, continua valendo.
    let err = env.run(CycleKind::Sync, Trigger::Auto).await.unwrap_err();
    assert!(matches!(err, Error::Guard(GuardError::RemoteDisabled)));
}

#[tokio::test]
async fn run_due_without_due_tasks_makes_no_requests() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    let out = env.engine().run_due(&env.store).await.unwrap();
    assert!(out.is_empty());
    assert_eq!(env.requests().await, 0);
}

#[tokio::test]
async fn run_due_executes_pending_renewal() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    env.mount_list_n(
        list_html(&[("k1", "Livro A", "2026-11-17")]),
        2,
        Some(list_html(&[("k1", "Livro A", "2026-11-24")])),
    )
    .await;
    env.mount_renew(body("RENOVADO")).await;
    // Primeiro ciclo só sincroniza e agenda a renovação de hoje.
    env.run(CycleKind::Sync, Trigger::Manual).await.unwrap();
    let pending = env.schedules(ScheduleKind::Renew, ScheduleStatus::Pending);
    assert_eq!(pending.len(), 1);
    env.clock
        .set(pending[0].run_at.max(env.clock.now()) + Duration::minutes(1));

    let out = env.engine().run_due(&env.store).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].report.error, None);
    assert_eq!(out[0].report.renewed.len(), 1);
    assert_eq!(out[0].report.renewed[0].trigger, Trigger::Auto);
    assert_eq!(
        env.schedules(ScheduleKind::Renew, ScheduleStatus::Done)
            .len(),
        1
    );
    env.assert_spacing();
}

#[test]
fn background_cycle_reports_errors_as_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bg.db");
    let key = keys::generate_key();
    Store::open(&path, &key).unwrap();
    let ok = renovador_core::ffi::run_background_cycle(path.to_str().unwrap(), &key);
    assert_eq!(ok, r#"{"notes":[],"outcomes":[]}"#);
    let bad =
        renovador_core::ffi::run_background_cycle(path.to_str().unwrap(), &keys::generate_key());
    assert!(bad.contains("\"error\""));
}

// ---------- "Renovar agora" de um empréstimo (Onda 3) ----------

#[tokio::test]
async fn renew_loan_renews_one_book_outside_the_window() {
    let env = Env::new().await;
    env.mount_login_ok().await;
    // Vence dia 25: com 1 dia de antecedência, a renovação automática só seria no dia 24.
    env.mount_list_n(
        list_html(&[("k1", "Livro A", "2026-11-25")]),
        2,
        Some(list_html(&[("k1", "Livro A", "2026-12-02")])),
    )
    .await;
    env.mount_renew(body("RENOVADO")).await;

    // O ciclo normal não renova (fora da janela)…
    let out = env.renew().await;
    assert!(out.report.renewed.is_empty());
    let loan_id = env.loan("k1").loan.id;
    // Espaçamento mínimo entre sessões do gate.
    env.clock.advance(chrono::Duration::hours(1));

    // …mas o pedido do usuário para este livro renova.
    let out = env
        .engine()
        .renew_loan(&env.store, loan_id, &|_| {})
        .await
        .unwrap();
    assert_eq!(out.report.error, None);
    assert_eq!(out.report.renewed.len(), 1);
    assert_eq!(out.report.renewed[0].loan_id, loan_id);
    assert_eq!(out.report.renewed[0].trigger, Trigger::Manual);
    assert_eq!(
        env.loan("k1").loan.due_date,
        NaiveDate::from_ymd_opt(2026, 12, 2).unwrap()
    );
    env.assert_spacing();
}

#[tokio::test]
async fn renew_loan_refuses_returned_loans_without_network() {
    let env = Env::new().await;
    env.standard(body("RENOVADO"), "2026-11-24").await;
    env.renew().await;
    let loan_id = env.loan("k1").loan.id;
    env.store.loan_mark_returned(loan_id).unwrap();
    let before = env.requests().await;
    let err = env
        .engine()
        .renew_loan(&env.store, loan_id, &|_| {})
        .await
        .unwrap_err();
    assert!(err.to_string().contains("devolvido"), "{err}");
    assert_eq!(env.requests().await, before);
}
