//! Testes da tabela 5.2 (anti-abuso) com `FakeClock`. Nenhum acesso à rede.

use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use ed25519_dalek::{Signer, SigningKey};
use renovador_core::clock::{Clock, FakeClock};
use renovador_core::guard::limits::*;
use renovador_core::guard::{
    DefaultGate, GuardError, LOGIN_KIND, MemoryGuardPersistence, RemotePolicy, RequestGate,
    RequestOutcome, SessionTicket, Sleeper, check_account_count, is_sophia_powered_by,
    policy_fetch_due,
};
use renovador_core::model::Trigger;

const BASE: &str = "https://biblioteca.sophia.com.br/5362";

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

struct Env {
    clock: Arc<FakeClock>,
    persistence: Arc<MemoryGuardPersistence>,
    sleeper: Arc<FakeSleeper>,
}

/// 10h em São Paulo (13h UTC), dentro da janela automática.
fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 10, 13, 0, 0).unwrap()
}

impl Env {
    fn new() -> Self {
        let clock = Arc::new(FakeClock::new(start()));
        Self {
            sleeper: Arc::new(FakeSleeper {
                clock: clock.clone(),
                waits: Mutex::new(Vec::new()),
            }),
            clock,
            persistence: Arc::new(MemoryGuardPersistence::new()),
        }
    }
    /// Um gate novo sobre a mesma persistência (equivale a reiniciar o app).
    fn gate(&self) -> DefaultGate {
        DefaultGate::new(self.persistence.clone(), self.clock.clone())
            .with_sleeper(self.sleeper.clone())
    }
    fn advance(&self, d: Duration) {
        self.clock.advance(d);
    }
}

async fn request(gate: &DefaultGate, t: &SessionTicket, outcome: RequestOutcome) {
    let p = gate.acquire(t, "list").await.expect("permit");
    gate.report(p, Some(200), outcome, None);
}

#[tokio::test]
async fn one_concurrent_request_globally() {
    let env = Env::new();
    let gate = Arc::new(env.gate());
    let a = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    let b = gate.begin_session(2, BASE, Trigger::Manual).await.unwrap();
    let held = gate.acquire(&a, "login").await.unwrap();
    let blocked = tokio::time::timeout(StdDuration::from_millis(100), gate.acquire(&b, "login"));
    assert!(
        blocked.await.is_err(),
        "segunda requisição não pode sair junto"
    );
    gate.report(held, Some(200), RequestOutcome::Ok, None);
    let p = tokio::time::timeout(StdDuration::from_secs(5), gate.acquire(&b, "login"))
        .await
        .expect("liberado após report")
        .unwrap();
    gate.report(p, Some(200), RequestOutcome::Ok, None);
}

#[tokio::test]
async fn spacing_is_at_least_two_seconds_plus_jitter() {
    let env = Env::new();
    let gate = env.gate();
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    let mut times = Vec::new();
    for _ in 0..5 {
        let p = gate.acquire(&t, "list").await.unwrap();
        times.push(env.clock.now());
        gate.report(p, Some(200), RequestOutcome::Ok, None);
    }
    for w in times.windows(2) {
        let gap = (w[1] - w[0]).to_std().unwrap();
        assert!(gap >= MIN_REQUEST_INTERVAL, "gap {gap:?}");
        assert!(
            gap <= MIN_REQUEST_INTERVAL + MAX_REQUEST_JITTER,
            "gap {gap:?}"
        );
    }
    assert_eq!(env.sleeper.waits.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn max_twenty_requests_per_session() {
    let env = Env::new();
    let gate = env.gate();
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    for _ in 0..MAX_REQUESTS_PER_SESSION {
        request(&gate, &t, RequestOutcome::Ok).await;
    }
    assert!(matches!(
        gate.acquire(&t, "list").await,
        Err(GuardError::SessionBudgetExhausted)
    ));
    gate.end_session(t.clone());
    assert!(gate.acquire(&t, "list").await.is_err());
}

#[tokio::test]
async fn two_auto_sessions_per_account_day() {
    let env = Env::new();
    let gate = env.gate();
    for _ in 0..MAX_AUTO_SESSIONS_PER_ACCOUNT_DAY {
        let t = gate.begin_session(1, BASE, Trigger::Auto).await.unwrap();
        gate.end_session(t);
    }
    // catch-up conta como automática
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::CatchUp).await,
        Err(GuardError::QuotaExhausted { retry_at: Some(_) })
    ));
    assert_eq!(gate.quota(1).unwrap().auto_sessions_left, 0);
    // outra conta não é afetada
    assert!(gate.begin_session(2, BASE, Trigger::Auto).await.is_ok());
    // no dia seguinte (10h) volta
    env.advance(Duration::days(1));
    assert!(gate.begin_session(1, BASE, Trigger::Auto).await.is_ok());
}

#[tokio::test]
async fn four_manual_sessions_with_thirty_minute_gap() {
    let env = Env::new();
    let gate = env.gate();
    gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    env.advance(Duration::minutes(10));
    match gate.begin_session(1, BASE, Trigger::Manual).await {
        Err(GuardError::TooSoon { retry_at }) => {
            assert_eq!(retry_at, start() + Duration::minutes(30))
        }
        other => panic!("esperava TooSoon, veio {other:?}"),
    }
    assert_eq!(
        gate.quota(1).unwrap().next_allowed_at,
        Some(start() + Duration::minutes(30))
    );
    env.advance(Duration::minutes(20));
    for _ in 1..MAX_MANUAL_SESSIONS_PER_ACCOUNT_DAY {
        gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
        env.advance(Duration::minutes(30));
    }
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Manual).await,
        Err(GuardError::QuotaExhausted { .. })
    ));
    assert_eq!(gate.quota(1).unwrap().manual_sessions_left, 0);
}

#[tokio::test]
async fn ten_sessions_per_install_day() {
    let env = Env::new();
    let gate = env.gate();
    let mut n = 0;
    'outer: for _ in 0..MAX_MANUAL_SESSIONS_PER_ACCOUNT_DAY {
        for acc in 1..=3 {
            for trig in [Trigger::Manual, Trigger::Auto] {
                if n == MAX_SESSIONS_PER_INSTALL_DAY {
                    break 'outer;
                }
                if gate.begin_session(acc, BASE, trig).await.is_ok() {
                    n += 1;
                }
            }
        }
        env.advance(Duration::minutes(31));
    }
    assert_eq!(n, MAX_SESSIONS_PER_INSTALL_DAY);
    assert_eq!(gate.quota(1).unwrap().install_sessions_left, 0);
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Manual).await,
        Err(GuardError::QuotaExhausted { .. })
    ));
}

#[tokio::test]
async fn accounts_and_distinct_logins_limits() {
    assert_eq!(DEFAULT_ACCOUNTS, 1);
    assert!(check_account_count(MAX_ACCOUNTS - 1).is_ok());
    assert!(matches!(
        check_account_count(MAX_ACCOUNTS),
        Err(GuardError::AccountLimit)
    ));

    let env = Env::new();
    let gate = env.gate();
    for acc in 1..=MAX_DISTINCT_LOGINS_PER_DAY as i32 {
        let t = gate.begin_session(acc, BASE, Trigger::Auto).await.unwrap();
        let p = gate.acquire(&t, LOGIN_KIND).await.unwrap();
        gate.report(p, Some(200), RequestOutcome::Ok, None);
    }
    assert!(matches!(
        gate.begin_session(99, BASE, Trigger::Auto).await,
        Err(GuardError::AccountLimit)
    ));
    // uma conta já testada hoje continua podendo
    assert!(gate.begin_session(1, BASE, Trigger::Auto).await.is_ok());
}

#[tokio::test]
async fn wrong_password_pauses_account_until_edited() {
    let env = Env::new();
    let gate = env.gate();
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    request(&gate, &t, RequestOutcome::InvalidCredentials).await;
    assert!(gate.acquire(&t, "list").await.is_err());
    env.advance(Duration::days(30));
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Manual).await,
        Err(GuardError::BreakerOpen { .. })
    ));
    assert_eq!(
        gate.quota(1).unwrap().reason.as_deref(),
        Some("invalid_credentials")
    );
    gate.resume_account(1).unwrap();
    assert!(gate.begin_session(1, BASE, Trigger::Manual).await.is_ok());
}

#[tokio::test]
async fn three_failures_pause_six_hours_five_pause_a_day() {
    let env = Env::new();
    let gate = env.gate();
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    request(&gate, &t, RequestOutcome::NetworkError).await;
    request(&gate, &t, RequestOutcome::ServerError).await;
    let opened_at = env.clock.now();
    let p = gate.acquire(&t, "list").await.unwrap();
    let opened_at = opened_at.max(env.clock.now());
    gate.report(p, Some(500), RequestOutcome::UnexpectedHtml, None);
    let until = match gate.acquire(&t, "list").await {
        Err(GuardError::BreakerOpen { until }) => until,
        other => panic!("esperava breaker aberto, veio {other:?}"),
    };
    assert_eq!(
        until,
        opened_at + Duration::from_std(BREAKER_SHORT_PAUSE).unwrap()
    );
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Manual).await,
        Err(GuardError::BreakerOpen { .. })
    ));

    env.clock.set(until + Duration::seconds(1));
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    request(&gate, &t, RequestOutcome::NetworkError).await; // 4ª: sem nova pausa
    let p = gate.acquire(&t, "list").await.unwrap();
    let at = env.clock.now();
    gate.report(p, None, RequestOutcome::NetworkError, None); // 5ª
    match gate.begin_session(1, BASE, Trigger::Manual).await {
        Err(GuardError::BreakerOpen { until }) => {
            assert_eq!(until, at + Duration::from_std(BREAKER_LONG_PAUSE).unwrap())
        }
        other => panic!("esperava pausa de 24 h, veio {other:?}"),
    }
}

#[tokio::test]
async fn success_resets_failure_counter() {
    let env = Env::new();
    let gate = env.gate();
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    request(&gate, &t, RequestOutcome::NetworkError).await;
    request(&gate, &t, RequestOutcome::NetworkError).await;
    request(&gate, &t, RequestOutcome::Ok).await;
    request(&gate, &t, RequestOutcome::NetworkError).await;
    request(&gate, &t, RequestOutcome::NetworkError).await;
    assert!(gate.acquire(&t, "list").await.is_ok());
}

#[tokio::test]
async fn retry_after_respected_with_one_hour_minimum() {
    let env = Env::new();
    let gate = env.gate();
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    let p = gate.acquire(&t, "list").await.unwrap();
    let at = env.clock.now();
    gate.report(
        p,
        Some(429),
        RequestOutcome::RateLimited,
        Some(StdDuration::from_secs(30)),
    );
    let min = at + Duration::from_std(MIN_RETRY_AFTER).unwrap();
    assert!(matches!(gate.acquire(&t, "list").await,
        Err(GuardError::RetryAfter { until }) if until == min));
    // vale para o host inteiro, inclusive outra conta
    assert!(matches!(
        gate.begin_session(2, BASE, Trigger::Manual).await,
        Err(GuardError::RetryAfter { .. })
    ));

    env.clock.set(min);
    let t = gate.begin_session(2, BASE, Trigger::Manual).await.unwrap();
    let p = gate.acquire(&t, "list").await.unwrap();
    let at = env.clock.now();
    gate.report(
        p,
        Some(503),
        RequestOutcome::RateLimited,
        Some(StdDuration::from_secs(3 * 3600)),
    );
    env.advance(Duration::hours(2));
    assert!(matches!(
        gate.begin_session(3, BASE, Trigger::Manual).await,
        Err(GuardError::RetryAfter { until }) if until == at + Duration::hours(3)
    ));
}

#[tokio::test]
async fn human_verification_stops_immediately() {
    let env = Env::new();
    let gate = env.gate();
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    request(&gate, &t, RequestOutcome::HumanVerification).await;
    assert!(gate.acquire(&t, "list").await.is_err());
    env.advance(Duration::days(2));
    assert!(gate.begin_session(1, BASE, Trigger::Manual).await.is_err());
    gate.resume_account(1).unwrap();
    assert!(gate.begin_session(1, BASE, Trigger::Manual).await.is_ok());
}

#[tokio::test]
async fn library_address_must_be_https_sophia() {
    let env = Env::new();
    let gate = env.gate();
    for bad in [
        "http://biblioteca.sophia.com.br/5362",
        "biblioteca.sophia.com.br",
        "ftp://x.com",
        "https://user:pw@x.com",
        "",
    ] {
        assert!(
            matches!(
                gate.begin_session(1, bad, Trigger::Manual).await,
                Err(GuardError::HostNotAllowed { .. })
            ),
            "{bad}"
        );
    }
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    assert_eq!(t.host, "biblioteca.sophia.com.br");
    assert!(is_sophia_powered_by(Some("Sophia")));
    assert!(is_sophia_powered_by(Some("ASP.NET, sophia")));
    assert!(!is_sophia_powered_by(Some("nginx")));
    assert!(!is_sophia_powered_by(None));
}

#[tokio::test]
async fn auto_only_inside_window() {
    let env = Env::new();
    let gate = env.gate();
    // 22h em São Paulo = 01h UTC do dia seguinte
    env.clock
        .set(Utc.with_ymd_and_hms(2026, 3, 11, 1, 0, 0).unwrap());
    match gate.begin_session(1, BASE, Trigger::Auto).await {
        Err(GuardError::OutsideWindow { next_window }) => assert_eq!(
            next_window,
            Utc.with_ymd_and_hms(2026, 3, 11, 10, 0, 0).unwrap()
        ),
        other => panic!("esperava fora da janela, veio {other:?}"),
    }
    assert!(gate.begin_session(1, BASE, Trigger::Manual).await.is_ok());
    // 06h59 local
    env.clock
        .set(Utc.with_ymd_and_hms(2026, 3, 11, 9, 59, 0).unwrap());
    assert!(gate.begin_session(2, BASE, Trigger::CatchUp).await.is_err());
    env.advance(Duration::minutes(1));
    assert!(gate.begin_session(2, BASE, Trigger::CatchUp).await.is_ok());
}

#[tokio::test]
async fn restart_does_not_reset_quota() {
    let env = Env::new();
    {
        let gate = env.gate();
        for _ in 0..MAX_AUTO_SESSIONS_PER_ACCOUNT_DAY {
            gate.begin_session(1, BASE, Trigger::Auto).await.unwrap();
        }
        gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
        let t = gate
            .begin_session(2, "https://outra.sophia.com.br/1", Trigger::Manual)
            .await
            .unwrap();
        for _ in 0..BREAKER_SHORT_THRESHOLD {
            request(&gate, &t, RequestOutcome::NetworkError).await;
        }
    }
    let gate = env.gate(); // "reinício"
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Auto).await,
        Err(GuardError::QuotaExhausted { .. })
    ));
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Manual).await,
        Err(GuardError::TooSoon { .. })
    ));
    assert!(gate.quota(2).unwrap().breaker_open_until.is_some());
    assert!(env.persistence.records().len() > 4);
}

// ---------- política remota ----------

fn keypair() -> (SigningKey, [u8; 32]) {
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pk = sk.verifying_key().to_bytes();
    (sk, pk)
}

#[test]
fn remote_policy_signature_is_verified() {
    let (sk, pk) = keypair();
    let json = br#"{"automation_enabled":false,"blocked_hosts":["evil.example"]}"#;
    let sig = sk.sign(json).to_bytes();
    let p = RemotePolicy::verify_and_parse(json, &sig, &pk).unwrap();
    assert_eq!(p.automation_enabled, Some(false));

    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(sig);
    assert!(RemotePolicy::verify_and_parse(json, b64.as_bytes(), &pk).is_ok());

    let tampered = br#"{"automation_enabled":true,"blocked_hosts":["evil.example"]}"#;
    assert!(RemotePolicy::verify_and_parse(tampered, &sig, &pk).is_err());
    let other = SigningKey::from_bytes(&[8u8; 32])
        .verifying_key()
        .to_bytes();
    assert!(RemotePolicy::verify_and_parse(json, &sig, &other).is_err());
    assert!(RemotePolicy::verify_and_parse(json, b"lixo", &pk).is_err());
}

#[tokio::test]
async fn remote_policy_never_loosens() {
    let loose = RemotePolicy {
        automation_enabled: Some(true),
        max_auto_sessions_per_account_day: Some(100),
        max_manual_sessions_per_account_day: Some(100),
        max_sessions_per_install_day: Some(100),
        max_requests_per_session: Some(1000),
        min_request_interval_secs: Some(0),
        blocked_hosts: vec![],
        message: None,
    };
    let eff = loose.effective();
    assert_eq!(eff, RemotePolicy::default().effective());
    assert_eq!(
        eff.max_auto_sessions_per_account_day,
        MAX_AUTO_SESSIONS_PER_ACCOUNT_DAY
    );
    assert_eq!(eff.min_request_interval, MIN_REQUEST_INTERVAL);

    // no gate: continua 2 sessões automáticas e 20 requisições
    let env = Env::new();
    let gate = env.gate().with_policy(&loose);
    let t = gate.begin_session(1, BASE, Trigger::Auto).await.unwrap();
    for _ in 0..MAX_REQUESTS_PER_SESSION {
        request(&gate, &t, RequestOutcome::Ok).await;
    }
    assert!(gate.acquire(&t, "x").await.is_err());
    gate.begin_session(1, BASE, Trigger::Auto).await.unwrap();
    assert!(gate.begin_session(1, BASE, Trigger::Auto).await.is_err());

    // um merge posterior também não afrouxa algo já apertado
    let tight = RemotePolicy {
        automation_enabled: Some(false),
        max_auto_sessions_per_account_day: Some(1),
        min_request_interval_secs: Some(10),
        blocked_hosts: vec!["a.example".into()],
        ..Default::default()
    };
    let merged = tight.merge(&loose).merge(&RemotePolicy {
        blocked_hosts: vec!["b.example".into()],
        ..Default::default()
    });
    assert_eq!(merged.automation_enabled, Some(false));
    assert_eq!(merged.max_auto_sessions_per_account_day, Some(1));
    assert_eq!(merged.min_request_interval_secs, Some(10));
    assert_eq!(merged.blocked_hosts, vec!["a.example", "b.example"]);
}

#[tokio::test]
async fn remote_policy_tightens() {
    let env = Env::new();
    let policy = RemotePolicy {
        automation_enabled: Some(false),
        max_manual_sessions_per_account_day: Some(1),
        max_requests_per_session: Some(2),
        min_request_interval_secs: Some(10),
        blocked_hosts: vec!["Bloqueado.Example".into()],
        ..Default::default()
    };
    let gate = env.gate().with_policy(&policy);
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Auto).await,
        Err(GuardError::RemoteDisabled)
    ));
    assert!(matches!(
        gate.begin_session(1, "https://bloqueado.example/x", Trigger::Manual)
            .await,
        Err(GuardError::HostNotAllowed { .. })
    ));
    let t = gate.begin_session(1, BASE, Trigger::Manual).await.unwrap();
    let p = gate.acquire(&t, "a").await.unwrap();
    let t0 = env.clock.now();
    gate.report(p, Some(200), RequestOutcome::Ok, None);
    let p = gate.acquire(&t, "b").await.unwrap();
    assert!(env.clock.now() - t0 >= Duration::seconds(10));
    gate.report(p, Some(200), RequestOutcome::Ok, None);
    assert!(matches!(
        gate.acquire(&t, "c").await,
        Err(GuardError::SessionBudgetExhausted)
    ));
    env.advance(Duration::hours(1));
    assert!(matches!(
        gate.begin_session(1, BASE, Trigger::Manual).await,
        Err(GuardError::QuotaExhausted { .. })
    ));
}

#[test]
fn remote_policy_fetched_at_most_daily() {
    let now = start();
    assert!(policy_fetch_due(None, now));
    assert!(!policy_fetch_due(Some(now - Duration::hours(23)), now));
    assert!(policy_fetch_due(Some(now - Duration::hours(24)), now));
}

#[test]
fn constants_match_table() {
    assert_eq!(MAX_CONCURRENT_REQUESTS, 1);
    assert_eq!(MIN_REQUEST_INTERVAL, StdDuration::from_secs(2));
    assert_eq!(MAX_REQUEST_JITTER, StdDuration::from_secs(1));
    assert_eq!(MAX_REQUESTS_PER_SESSION, 20);
    assert_eq!(MAX_AUTO_SESSIONS_PER_ACCOUNT_DAY, 2);
    assert_eq!(MAX_MANUAL_SESSIONS_PER_ACCOUNT_DAY, 4);
    assert_eq!(MAX_SESSIONS_PER_INSTALL_DAY, 10);
    assert_eq!(MAX_ACCOUNTS, 3);
    assert_eq!(MAX_DISTINCT_LOGINS_PER_DAY, 3);
    assert_eq!(MIN_RETRY_AFTER, StdDuration::from_secs(3600));
}
