//! Limites anti-abuso (seção 5.2 do plano). **Contrato congelado.**
//!
//! Estas constantes são o único lugar onde os limites existem. Elas NUNCA podem
//! ser afrouxadas por arquivo de configuração, CLI ou variável de ambiente.
//! A política remota assinada só pode apertá-las (ver `guard::RemotePolicy`).

use std::time::Duration;

/// Requisições simultâneas em toda a instalação.
pub const MAX_CONCURRENT_REQUESTS: usize = 1;
/// Intervalo mínimo entre duas requisições.
pub const MIN_REQUEST_INTERVAL: Duration = Duration::from_secs(2);
/// Jitter aleatório máximo somado ao intervalo mínimo.
pub const MAX_REQUEST_JITTER: Duration = Duration::from_secs(1);
/// Máximo de requisições HTTP numa mesma sessão (login → logout).
pub const MAX_REQUESTS_PER_SESSION: u32 = 20;

/// Sessões automáticas por conta por dia.
pub const MAX_AUTO_SESSIONS_PER_ACCOUNT_DAY: u32 = 2;
/// Sessões manuais por conta por dia.
pub const MAX_MANUAL_SESSIONS_PER_ACCOUNT_DAY: u32 = 4;
/// Espera mínima entre sessões manuais da mesma conta.
pub const MIN_MANUAL_SESSION_GAP: Duration = Duration::from_secs(30 * 60);
/// Sessões por instalação por dia (todas as contas somadas).
pub const MAX_SESSIONS_PER_INSTALL_DAY: u32 = 10;

/// Contas por instalação.
pub const MAX_ACCOUNTS: u32 = 3;
/// Contas criadas por padrão no onboarding.
pub const DEFAULT_ACCOUNTS: u32 = 1;
/// Logins distintos testados por instalação por dia.
pub const MAX_DISTINCT_LOGINS_PER_DAY: u32 = 3;

/// Falhas consecutivas que abrem o breaker por [`BREAKER_SHORT_PAUSE`].
pub const BREAKER_SHORT_THRESHOLD: u32 = 3;
pub const BREAKER_SHORT_PAUSE: Duration = Duration::from_secs(6 * 60 * 60);
/// Falhas consecutivas que abrem o breaker por [`BREAKER_LONG_PAUSE`] (com notificação).
pub const BREAKER_LONG_THRESHOLD: u32 = 5;
pub const BREAKER_LONG_PAUSE: Duration = Duration::from_secs(24 * 60 * 60);

/// Pausa mínima após HTTP 429/503, mesmo se `Retry-After` for menor.
pub const MIN_RETRY_AFTER: Duration = Duration::from_secs(60 * 60);

/// Timeout de cada requisição.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Novas tentativas permitidas em GET idempotente após erro de rede.
pub const MAX_NETWORK_RETRIES: u32 = 1;

/// Janela de execução automática (hora local America/Sao_Paulo), [início, fim).
pub const AUTO_WINDOW_START_HOUR: u32 = 7;
pub const AUTO_WINDOW_END_HOUR: u32 = 21;

/// Intervalo mínimo entre consultas à política remota.
pub const REMOTE_POLICY_MIN_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Só HTTPS é aceito para o endereço da biblioteca.
pub const REQUIRE_HTTPS: bool = true;
/// Cabeçalho que identifica um servidor Sophia.
pub const SOPHIA_POWERED_BY: &str = "Sophia";
