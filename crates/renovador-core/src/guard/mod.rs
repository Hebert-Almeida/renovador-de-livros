//! Portão anti-abuso: toda requisição ao Sophia passa por [`RequestGate`].
//! Implementação na W1-01. Contratos (traits e tipos) congelados na Onda 0.

mod gate;
pub mod limits;
pub mod memory;
pub mod policy;

pub use gate::{
    DefaultGate, LOGIN_KIND, Sleeper, TokioSleeper, account_scope, check_account_count, host_scope,
    is_sophia_powered_by, session_kind, session_trigger, validate_library_url,
};
pub use memory::MemoryGuardPersistence;
pub use policy::{EffectiveLimits, policy_fetch_due};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{Id, QuotaStatus, Trigger};

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize, specta::Type)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum GuardError {
    #[error("cota diária esgotada")]
    QuotaExhausted { retry_at: Option<DateTime<Utc>> },
    #[error("aguarde antes de sincronizar de novo")]
    TooSoon { retry_at: DateTime<Utc> },
    #[error("limite de requisições da sessão atingido")]
    SessionBudgetExhausted,
    #[error("automação pausada após falhas consecutivas")]
    BreakerOpen { until: DateTime<Utc> },
    #[error("o servidor pediu para aguardar")]
    RetryAfter { until: DateTime<Utc> },
    #[error("fora da janela de execução automática")]
    OutsideWindow { next_window: DateTime<Utc> },
    #[error("automação desligada pela política remota")]
    RemoteDisabled,
    #[error("host não permitido: {host}")]
    HostNotAllowed { host: String },
    #[error("limite de contas ou logins atingido")]
    AccountLimit,
}

/// Desfecho de uma requisição, informado ao gate para alimentar o breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestOutcome {
    Ok,
    NetworkError,
    ServerError,
    UnexpectedHtml,
    RateLimited,
    HumanVerification,
    InvalidCredentials,
}

/// Registro de uma requisição feita (persistido em `request_log`).
#[derive(Debug, Clone)]
pub struct RequestRecord {
    pub account_id: Option<Id>,
    pub host: String,
    pub at: DateTime<Utc>,
    pub kind: String,
    pub session_id: String,
    pub http_status: Option<u16>,
    pub outcome: RequestOutcome,
}

/// Estado do breaker por escopo (`account:<id>` ou `host:<host>`).
#[derive(Debug, Clone, Default)]
pub struct BreakerState {
    pub consecutive_failures: u32,
    pub open_until: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

/// Persistência usada pelo gate. Implementada pelo `Store` (W1-02) e por uma
/// versão em memória para testes (W1-01). Reiniciar o app não zera as cotas.
pub trait GuardPersistence: Send + Sync {
    fn record_request(&self, rec: &RequestRecord) -> crate::Result<()>;
    /// Sessões iniciadas desde `since`, filtradas por conta e gatilho.
    fn count_sessions(
        &self,
        account_id: Option<Id>,
        trigger: Option<Trigger>,
        since: DateTime<Utc>,
    ) -> crate::Result<u32>;
    fn last_session_start(
        &self,
        account_id: Id,
        trigger: Trigger,
    ) -> crate::Result<Option<DateTime<Utc>>>;
    fn distinct_logins_since(&self, since: DateTime<Utc>) -> crate::Result<u32>;
    fn breaker(&self, scope: &str) -> crate::Result<BreakerState>;
    fn set_breaker(&self, scope: &str, state: &BreakerState) -> crate::Result<()>;
}

/// Política remota assinada (Ed25519). Só aperta os limites.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemotePolicy {
    pub automation_enabled: Option<bool>,
    pub max_auto_sessions_per_account_day: Option<u32>,
    pub max_manual_sessions_per_account_day: Option<u32>,
    pub max_sessions_per_install_day: Option<u32>,
    pub max_requests_per_session: Option<u32>,
    pub min_request_interval_secs: Option<u64>,
    #[serde(default)]
    pub blocked_hosts: Vec<String>,
    pub message: Option<String>,
}

/// Permissão para fazer **uma** requisição. Liberar (drop) devolve o semáforo.
#[derive(Debug)]
pub struct Permit {
    pub session_id: String,
    pub request_index: u32,
    pub(crate) account_id: Id,
    pub(crate) host: String,
    pub(crate) kind: String,
    pub(crate) _slot: Option<tokio::sync::OwnedSemaphorePermit>,
}

#[cfg(feature = "test-util")]
impl Permit {
    /// Permissão avulsa para gates falsos em testes.
    pub fn for_tests(session_id: String, request_index: u32) -> Self {
        Self {
            session_id,
            request_index,
            account_id: 0,
            host: String::new(),
            kind: String::new(),
            _slot: None,
        }
    }
}

/// Sessão aberta no gate (login → logout de uma conta).
#[derive(Debug, Clone)]
pub struct SessionTicket {
    pub session_id: String,
    pub account_id: Id,
    pub host: String,
    pub trigger: Trigger,
}

/// Portão anti-abuso. Toda requisição HTTP ao Sophia exige um [`Permit`].
#[async_trait]
pub trait RequestGate: Send + Sync {
    /// Abre uma sessão, verificando cotas, breaker, janela e política remota.
    async fn begin_session(
        &self,
        account_id: Id,
        host: &str,
        trigger: Trigger,
    ) -> Result<SessionTicket, GuardError>;
    /// Espera o espaçamento mínimo e devolve a permissão para uma requisição.
    async fn acquire(&self, ticket: &SessionTicket, kind: &str) -> Result<Permit, GuardError>;
    /// Informa o desfecho da requisição (alimenta breaker e `request_log`).
    fn report(
        &self,
        permit: Permit,
        http_status: Option<u16>,
        outcome: RequestOutcome,
        retry_after: Option<std::time::Duration>,
    );
    fn end_session(&self, ticket: SessionTicket);
    fn quota(&self, account_id: Id) -> crate::Result<QuotaStatus>;
}
