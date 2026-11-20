//! Testes do cliente Sophia com wiremock e gate falso. Nunca acessa a rede real.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDate;
use renovador_core::Error;
use renovador_core::guard::{GuardError, Permit, RequestGate, RequestOutcome, SessionTicket};
use renovador_core::model::{Id, QuotaStatus, Trigger};
use renovador_core::sophia::client::{ClientConfig, HttpSophiaClient, Parsers, ReqwestTransport};
use renovador_core::sophia::{
    HttpResponse, LoggedSession, LoginResponse, LoginResult, RemoteLoan, RenewOutcome,
    SophiaClient, Transport,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

type Report = (Option<u16>, RequestOutcome, Option<Duration>);

#[derive(Default)]
struct FakeGate {
    acquired: Mutex<Vec<String>>,
    reports: Mutex<Vec<Report>>,
    deny: bool,
}

#[async_trait]
impl RequestGate for FakeGate {
    async fn begin_session(
        &self,
        account_id: Id,
        host: &str,
        trigger: Trigger,
    ) -> Result<SessionTicket, GuardError> {
        Ok(SessionTicket {
            session_id: "s".into(),
            account_id,
            host: host.into(),
            trigger,
        })
    }
    async fn acquire(&self, ticket: &SessionTicket, kind: &str) -> Result<Permit, GuardError> {
        if self.deny {
            return Err(GuardError::SessionBudgetExhausted);
        }
        let mut a = self.acquired.lock().unwrap();
        a.push(kind.into());
        Ok(Permit::for_tests(ticket.session_id.clone(), a.len() as u32))
    }
    fn report(&self, _p: Permit, s: Option<u16>, o: RequestOutcome, r: Option<Duration>) {
        self.reports.lock().unwrap().push((s, o, r));
    }
    fn end_session(&self, _ticket: SessionTicket) {}
    fn quota(&self, _account_id: Id) -> renovador_core::Result<QuotaStatus> {
        Err(Error::todo("fake"))
    }
}

impl FakeGate {
    fn kinds(&self) -> Vec<String> {
        self.acquired.lock().unwrap().clone()
    }
    fn outcomes(&self) -> Vec<RequestOutcome> {
        self.reports.lock().unwrap().iter().map(|r| r.1).collect()
    }
}

// Parsers falsos: o parser real é da W1-06.
fn fake_login(body: &str) -> renovador_core::Result<LoginResponse> {
    serde_json::from_str(body).map_err(|e| Error::Parse(e.to_string()))
}
fn fake_list(body: &str) -> renovador_core::Result<Vec<RemoteLoan>> {
    if !body.contains("<h3>") {
        return Err(Error::Parse("lista".into()));
    }
    Ok(vec![loan()])
}
fn fake_renewal(body: &str) -> renovador_core::Result<RenewOutcome> {
    if body.contains("renovado") {
        Ok(RenewOutcome::Success { new_due_date: None })
    } else {
        Err(Error::Parse("renovação".into()))
    }
}

fn loan() -> RemoteLoan {
    RemoteLoan {
        item_key: "k1".into(),
        title: "Livro".into(),
        due_date: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(),
        renewals_count: Some(0),
        renew_url: Some("renovar.asp?id=1".into()),
    }
}

fn config() -> ClientConfig {
    ClientConfig {
        require_https: false,
        logout_path: None,
        parsers: Parsers {
            login: fake_login,
            loan_list: fake_list,
            renewal: fake_renewal,
        },
    }
}

fn client_with(transport: Arc<dyn Transport>, gate: Arc<FakeGate>) -> HttpSophiaClient {
    HttpSophiaClient::new(transport, gate, config())
}

fn client(gate: Arc<FakeGate>) -> HttpSophiaClient {
    client_with(Arc::new(ReqwestTransport::new().unwrap()), gate)
}

fn base(server: &MockServer) -> String {
    format!("{}/5362", server.uri())
}

fn host(server: &MockServer) -> String {
    server.uri().trim_start_matches("http://").to_owned()
}

fn ticket(server: &MockServer) -> SessionTicket {
    SessionTicket {
        session_id: "s".into(),
        account_id: 1,
        host: host(server),
        trigger: Trigger::Manual,
    }
}

fn session(server: &MockServer) -> LoggedSession {
    LoggedSession {
        base_url: format!("{}/", base(server)),
        loans_path: "lista.asp?x=1".into(),
    }
}

async fn mount_login_page(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/5362/mobile/login.asp"))
        .and(header("accept-language", "pt-BR"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-powered-by", "Sophia")
                .insert_header("set-cookie", "TESTE=1; Path=/")
                .set_body_string("<form id=\"login\"></form>"),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn login_ok_sends_cookies_and_honest_user_agent() {
    let server = MockServer::start().await;
    mount_login_page(&server).await;
    Mock::given(method("POST"))
        .and(path("/5362/mobile/ajx_efetua_login.asp"))
        .and(header("cookie", "TESTE=1"))
        .and(header(
            "user-agent",
            renovador_core::sophia::user_agent().as_str(),
        ))
        .and(body_string_contains("codigo=aluno&senha=s%26nha"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"login_valido":"true","mensagem":"","url":"lista.asp?x=1","abrir_popup":false}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;
    // Destino do login: visitado como no navegador, conteúdo não usado.
    Mock::given(method("GET"))
        .and(path("/5362/mobile/lista.asp"))
        .and(header("cookie", "TESTE=1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html></html>"))
        .expect(1)
        .mount(&server)
        .await;
    let gate = Arc::new(FakeGate::default());
    let res = client(gate.clone())
        .login(&ticket(&server), &base(&server), "aluno", "s&nha")
        .await
        .unwrap();
    let LoginResult::Ok(s) = res else {
        panic!("esperava login ok")
    };
    assert_eq!(s.loans_path, "renovacoes.asp?idioma=ptbr&acesso=web");
    assert_eq!(gate.kinds(), ["login_page", "login", "login_landing"]);
    assert_eq!(
        gate.outcomes(),
        [RequestOutcome::Ok, RequestOutcome::Ok, RequestOutcome::Ok]
    );
}

#[tokio::test]
async fn login_error_reports_invalid_credentials() {
    let server = MockServer::start().await;
    mount_login_page(&server).await;
    Mock::given(method("POST"))
        .and(path("/5362/mobile/ajx_efetua_login.asp"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"login_valido":"false","mensagem":"Senha inválida","url":"","abrir_popup":false}"#,
        ))
        .mount(&server)
        .await;
    let gate = Arc::new(FakeGate::default());
    let res = client(gate.clone())
        .login(&ticket(&server), &base(&server), "aluno", "x")
        .await
        .unwrap();
    assert!(
        matches!(res, LoginResult::InvalidCredentials { message } if message == "Senha inválida")
    );
    assert_eq!(gate.outcomes()[1], RequestOutcome::InvalidCredentials);
}

#[tokio::test]
async fn list_and_renew_go_through_gate() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/lista.asp"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<h3>Livro</h3>"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/renovar.asp"))
        .respond_with(ResponseTemplate::new(200).set_body_string("renovado até 27/11/2026"))
        .expect(1)
        .mount(&server)
        .await;
    let gate = Arc::new(FakeGate::default());
    let c = client(gate.clone());
    let (t, s) = (ticket(&server), session(&server));
    let loans = c.list_loans(&t, &s).await.unwrap();
    assert_eq!(loans, vec![loan()]);
    let out = c.renew(&t, &s, &loans[0]).await.unwrap();
    assert_eq!(out, RenewOutcome::Success { new_due_date: None });
    c.logout(&t, &s).await.unwrap();
    assert_eq!(gate.kinds(), ["list", "renew"]);
}

#[tokio::test]
async fn unexpected_html_is_reported() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<p>outra coisa</p>"))
        .mount(&server)
        .await;
    let gate = Arc::new(FakeGate::default());
    let err = client(gate.clone())
        .list_loans(&ticket(&server), &session(&server))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Parse(_)));
    assert_eq!(gate.outcomes(), [RequestOutcome::UnexpectedHtml]);
}

async fn single_response_case(resp: ResponseTemplate) -> (Error, Arc<FakeGate>) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(resp)
        .expect(1) // nunca há nova tentativa
        .mount(&server)
        .await;
    let gate = Arc::new(FakeGate::default());
    let err = client(gate.clone())
        .list_loans(&ticket(&server), &session(&server))
        .await
        .unwrap_err();
    (err, gate)
}

#[tokio::test]
async fn captcha_aborts_without_retry() {
    let (err, gate) = single_response_case(
        ResponseTemplate::new(200).set_body_string("<div class=\"g-recaptcha\"></div>"),
    )
    .await;
    assert!(matches!(err, Error::HumanVerificationRequired));
    assert_eq!(gate.outcomes(), [RequestOutcome::HumanVerification]);
}

#[tokio::test]
async fn cloudflare_challenge_aborts() {
    let (err, gate) = single_response_case(
        ResponseTemplate::new(403).set_body_string("<title>Just a moment...</title>"),
    )
    .await;
    assert!(matches!(err, Error::HumanVerificationRequired));
    assert_eq!(gate.outcomes(), [RequestOutcome::HumanVerification]);
}

#[tokio::test]
async fn http_429_passes_retry_after_to_gate() {
    let (err, gate) =
        single_response_case(ResponseTemplate::new(429).insert_header("retry-after", "7200")).await;
    assert!(matches!(err, Error::Network(_)));
    let r = gate.reports.lock().unwrap()[0];
    assert_eq!(
        r,
        (
            Some(429),
            RequestOutcome::RateLimited,
            Some(Duration::from_secs(7200))
        )
    );
}

#[tokio::test]
async fn http_503_is_rate_limited() {
    let (_, gate) = single_response_case(ResponseTemplate::new(503)).await;
    assert_eq!(gate.outcomes(), [RequestOutcome::RateLimited]);
}

#[tokio::test]
async fn http_500_is_server_error() {
    let (_, gate) = single_response_case(ResponseTemplate::new(500)).await;
    assert_eq!(gate.outcomes(), [RequestOutcome::ServerError]);
}

#[tokio::test]
async fn redirect_to_foreign_host_aborts() {
    let other = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<h3>x</h3>"))
        .mount(&other)
        .await;
    // Mesmo IP, outra porta: conta como outro host.
    let (err, _) = single_response_case(
        ResponseTemplate::new(302).insert_header("location", format!("{}/fora", other.uri())),
    )
    .await;
    assert!(matches!(err, Error::InvalidLibrary(_)));
}

#[tokio::test]
async fn validate_library_requires_sophia_header() {
    let server = MockServer::start().await;
    mount_login_page(&server).await;
    let gate = Arc::new(FakeGate::default());
    let h = client(gate.clone())
        .validate_library(&base(&server))
        .await
        .unwrap();
    assert_eq!(h, host(&server));

    let plain = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&plain)
        .await;
    let err = client(gate)
        .validate_library(&base(&plain))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidLibrary(_)));
}

#[tokio::test]
async fn validate_library_accepts_sophia_page_without_header() {
    // O servidor real pode não enviar `x-powered-by: Sophia`; a página do
    // Terminal Web mobile tem título e logotipo próprios.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/5362/mobile/login.asp"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-powered-by", "ASP.NET")
                .set_body_string(
                    "<html><head><title>Sophia Biblioteca - Mobile</title></head>                     <body><img id=\"imglogotiposophia\" alt=\"Sophia\"></body></html>",
                ),
        )
        .mount(&server)
        .await;
    let h = client(Arc::new(FakeGate::default()))
        .validate_library(&base(&server))
        .await
        .unwrap();
    assert_eq!(h, host(&server));
}

#[tokio::test]
async fn https_is_required_by_default() {
    let gate: Arc<FakeGate> = Arc::new(FakeGate::default());
    let c = HttpSophiaClient::new(
        Arc::new(ReqwestTransport::new().unwrap()),
        gate.clone(),
        ClientConfig::default(),
    );
    let err = c
        .validate_library("http://127.0.0.1:1/5362")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidLibrary(_)));
    assert!(gate.kinds().is_empty());
}

#[tokio::test]
async fn gate_denial_blocks_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let gate = Arc::new(FakeGate {
        deny: true,
        ..Default::default()
    });
    let err = client(gate)
        .list_loans(&ticket(&server), &session(&server))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        Error::Guard(GuardError::SessionBudgetExhausted)
    ));
}

/// Transporte que falha nas primeiras `fail` chamadas.
struct Flaky {
    inner: ReqwestTransport,
    fail: u32,
    calls: AtomicU32,
}

#[async_trait]
impl Transport for Flaky {
    async fn get(&self, url: &str) -> renovador_core::Result<HttpResponse> {
        if self.calls.fetch_add(1, Ordering::SeqCst) < self.fail {
            return Err(Error::Network("conexão recusada".into()));
        }
        self.inner.get(url).await
    }
    async fn post_form(
        &self,
        url: &str,
        form: &[(&str, &str)],
    ) -> renovador_core::Result<HttpResponse> {
        if self.calls.fetch_add(1, Ordering::SeqCst) < self.fail {
            return Err(Error::Network("conexão recusada".into()));
        }
        self.inner.post_form(url, form).await
    }
}

fn flaky(fail: u32) -> Arc<Flaky> {
    Arc::new(Flaky {
        inner: ReqwestTransport::new().unwrap(),
        fail,
        calls: AtomicU32::new(0),
    })
}

#[tokio::test]
async fn idempotent_get_retries_once_on_network_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<h3>Livro</h3>"))
        .mount(&server)
        .await;
    let gate = Arc::new(FakeGate::default());
    let t = flaky(1);
    client_with(t.clone(), gate.clone())
        .list_loans(&ticket(&server), &session(&server))
        .await
        .unwrap();
    assert_eq!(gate.kinds(), ["list", "list"]);
    assert_eq!(
        gate.outcomes(),
        [RequestOutcome::NetworkError, RequestOutcome::Ok]
    );

    // Duas falhas seguidas: desiste após 1 nova tentativa.
    let gate = Arc::new(FakeGate::default());
    let t = flaky(5);
    let err = client_with(t.clone(), gate.clone())
        .list_loans(&ticket(&server), &session(&server))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Network(_)));
    assert_eq!(t.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn renew_and_post_never_retry() {
    let server = MockServer::start().await;
    let gate = Arc::new(FakeGate::default());
    let t = flaky(5);
    let c = client_with(t.clone(), gate.clone());
    let _ = c
        .renew(&ticket(&server), &session(&server), &loan())
        .await
        .unwrap_err();
    assert_eq!(t.calls.load(Ordering::SeqCst), 1);

    // POST de login: a página carrega, o POST falha uma vez e não é repetido.
    mount_login_page(&server).await;
    let t = Arc::new(Flaky {
        inner: ReqwestTransport::new().unwrap(),
        fail: 0,
        calls: AtomicU32::new(0),
    });
    let gate = Arc::new(FakeGate::default());
    let c = client_with(t.clone(), gate.clone());
    // Sem mock para o POST → 404: erro de parsing, um único POST.
    let err = c
        .login(&ticket(&server), &base(&server), "a", "b")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Parse(_)));
    assert_eq!(gate.kinds(), ["login_page", "login"]);
}
