//! Tipos de domínio (contrato congelado). Espelham `store/migrations/0001_init.sql`.
//! Datas: timestamps em UTC; `due_date` é data civil em America/Sao_Paulo.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Identificador de registro. `i32` porque o TypeScript não representa `i64` com segurança;
/// o `Store` converte os rowids do SQLite.
pub type Id = i32;

macro_rules! str_enum {
    ($name:ident { $($variant:ident => $s:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $s),+ }
            }
        }

        impl std::str::FromStr for $name {
            type Err = crate::Error;
            fn from_str(s: &str) -> crate::Result<Self> {
                match s {
                    $($s => Ok(Self::$variant),)+
                    other => Err(crate::Error::Invalid(format!(
                        "{} desconhecido: {}", stringify!($name), other
                    ))),
                }
            }
        }
    };
}

str_enum!(AccountMode { Auto => "auto", Manual => "manual" });
str_enum!(AccountStatus {
    Active => "active",
    Paused => "paused",
    InvalidPassword => "invalid_password",
    Blocked => "blocked",
});
str_enum!(LoanStatus { Active => "active", Returned => "returned", Overdue => "overdue" });
str_enum!(Trigger { Auto => "auto", Manual => "manual", CatchUp => "catch_up" });
str_enum!(RenewalResult {
    Success => "success",
    Reserved => "reserved",
    LimitReached => "limit_reached",
    Blocked => "blocked",
    Overdue => "overdue",
    NetworkError => "network_error",
    ParseError => "parse_error",
    Captcha => "captcha",
    RateLimited => "rate_limited",
});
str_enum!(ScheduleKind { Renew => "renew", Sync => "sync", Remind => "remind" });
str_enum!(ScheduleStatus {
    Pending => "pending",
    Paused => "paused",
    Done => "done",
    Cancelled => "cancelled",
    Failed => "failed",
});
str_enum!(ScheduleOrigin { System => "system", User => "user" });
str_enum!(CycleKind { Sync => "sync", RenewDue => "renew_due" });
str_enum!(ExportFormat { Csv => "csv", Json => "json" });
str_enum!(Theme { System => "system", Light => "light", Dark => "dark", Contrast => "contrast" });
str_enum!(MotionPref { System => "system", Reduced => "reduced", Full => "full" });
// Período do dia da renovação automática (Onda 3). O minuto continua sorteado
// dentro do período para espalhar os acessos ao Sophia.
str_enum!(RenewPeriod { Auto => "auto", Morning => "morning", Afternoon => "afternoon", Evening => "evening" });
str_enum!(CycleStep {
    Waiting => "waiting",
    Login => "login",
    List => "list",
    Renew => "renew",
    Confirm => "confirm",
    Save => "save",
    Done => "done",
});

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Account {
    pub id: Id,
    pub library_base_url: String,
    pub library_name: String,
    pub login: String,
    /// `false` = modo "sem salvar senha". A senha nunca é enviada ao frontend.
    pub has_password: bool,
    pub mode: AccountMode,
    pub status: AccountStatus,
    pub renew_days_before: u8,
    pub remind_days_before: u8,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_renewal_at: Option<DateTime<Utc>>,
    pub next_renewal_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NewAccount {
    pub library_base_url: String,
    pub library_name: String,
    pub login: String,
    /// `None` = não salvar a senha.
    pub password: Option<String>,
    pub mode: AccountMode,
    pub renew_days_before: u8,
    pub remind_days_before: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AccountPatch {
    pub library_base_url: Option<String>,
    pub library_name: Option<String>,
    pub login: Option<String>,
    /// Nova senha. Use `clear_password` para apagar a salva.
    pub password: Option<String>,
    pub clear_password: bool,
    pub mode: Option<AccountMode>,
    pub status: Option<AccountStatus>,
    pub renew_days_before: Option<u8>,
    pub remind_days_before: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Book {
    pub id: Id,
    pub account_id: Id,
    pub item_key: String,
    pub title: String,
    pub author: Option<String>,
    pub call_number: Option<String>,
    pub library_branch: Option<String>,
    pub first_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Loan {
    pub id: Id,
    pub book_id: Id,
    pub loaned_at: Option<DateTime<Utc>>,
    pub due_date: NaiveDate,
    pub renewals_count: u32,
    pub renewals_limit: Option<u32>,
    pub status: LoanStatus,
    pub auto_renew: bool,
    pub returned_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// Empréstimo com o livro, para listas da UI.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LoanView {
    pub loan: Loan,
    pub book: Book,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LoanDetail {
    pub view: LoanView,
    pub events: Vec<RenewalEvent>,
    pub schedules: Vec<Schedule>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct LoanFilter {
    pub account_id: Option<Id>,
    pub status: Option<LoanStatus>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RenewalEvent {
    pub id: Id,
    pub loan_id: Id,
    pub attempted_at: DateTime<Utc>,
    pub trigger: Trigger,
    pub result: RenewalResult,
    pub old_due_date: Option<NaiveDate>,
    pub new_due_date: Option<NaiveDate>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct HistoryFilter {
    pub account_id: Option<Id>,
    pub loan_id: Option<Id>,
    pub result: Option<RenewalResult>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct Page {
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HistoryPage {
    pub items: Vec<RenewalEvent>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Schedule {
    pub id: Id,
    pub account_id: Id,
    pub loan_id: Option<Id>,
    pub kind: ScheduleKind,
    pub run_at: DateTime<Utc>,
    pub status: ScheduleStatus,
    pub origin: ScheduleOrigin,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ScheduleFilter {
    pub account_id: Option<Id>,
    pub kind: Option<ScheduleKind>,
    pub status: Option<ScheduleStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RequestLogEntry {
    pub id: Id,
    pub account_id: Option<Id>,
    pub host: String,
    pub at: DateTime<Utc>,
    pub kind: String,
    pub session_id: String,
    pub http_status: Option<u16>,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LibraryPreset {
    pub name: String,
    pub base_url: String,
    /// `true` só para a UFTM; outras bibliotecas são experimentais.
    pub supported: bool,
}

pub fn library_presets() -> Vec<LibraryPreset> {
    vec![LibraryPreset {
        name: "UFTM — Universidade Federal do Triângulo Mineiro".into(),
        base_url: "https://biblioteca.sophia.com.br/5362".into(),
        supported: true,
    }]
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LibraryValidation {
    pub ok: bool,
    pub normalized_url: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct QuotaStatus {
    pub account_id: Id,
    pub manual_sessions_left: u32,
    pub auto_sessions_left: u32,
    pub install_sessions_left: u32,
    /// Próximo instante em que uma sessão manual é permitida.
    pub next_allowed_at: Option<DateTime<Utc>>,
    pub breaker_open_until: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AutomationState {
    pub enabled: bool,
    pub remote_enabled: bool,
    pub paused_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AppStatus {
    pub version: String,
    pub accounts: u32,
    pub automation: AutomationState,
    pub next_run_at: Option<DateTime<Utc>>,
    pub onboarding_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Settings {
    pub theme: Theme,
    pub motion: MotionPref,
    pub notifications: bool,
    pub autostart: bool,
    pub app_lock: bool,
    pub onboarding_done: bool,
    pub privacy_accepted_at: Option<DateTime<Utc>>,
    /// Período do dia em que as renovações automáticas são planejadas.
    pub renew_period: RenewPeriod,
    /// Automação pausada pela bandeja (persiste entre reinícios).
    pub automation_paused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            motion: MotionPref::System,
            notifications: true,
            autostart: true,
            app_lock: false,
            onboarding_done: false,
            privacy_accepted_at: None,
            renew_period: RenewPeriod::Auto,
            automation_paused: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct SettingsPatch {
    pub theme: Option<Theme>,
    pub motion: Option<MotionPref>,
    pub notifications: Option<bool>,
    pub autostart: Option<bool>,
    pub app_lock: Option<bool>,
    pub onboarding_done: Option<bool>,
    pub privacy_accepted_at: Option<DateTime<Utc>>,
    pub renew_period: Option<RenewPeriod>,
    pub automation_paused: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct StatsOverview {
    pub distinct_books: u32,
    pub active_loans: u32,
    pub renewals_total: u32,
    /// `None` sem tentativas no período (não há taxa a mostrar).
    pub success_rate: Option<f64>,
    pub days_gained: u32,
    pub most_renewed: Option<Book>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MonthlyStat {
    /// `YYYY-MM`.
    pub month: String,
    pub renewals: u32,
    pub successes: u32,
    pub failures: u32,
}

/// Progresso de um ciclo (evento `cycle://progress`).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CycleProgress {
    pub account_id: Id,
    pub step: CycleStep,
    pub message: String,
    pub current: Option<u32>,
    pub total: Option<u32>,
}

/// Resultado de um ciclo (evento `cycle://finished`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct CycleReport {
    pub account_id: Id,
    pub requests_made: u32,
    pub loans_seen: u32,
    pub renewed: Vec<RenewalEvent>,
    pub failed: Vec<RenewalEvent>,
    pub returned_detected: u32,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn enums_round_trip_with_sql_values() {
        for r in [
            RenewalResult::Success,
            RenewalResult::LimitReached,
            RenewalResult::RateLimited,
        ] {
            assert_eq!(RenewalResult::from_str(r.as_str()).unwrap(), r);
        }
        assert_eq!(Trigger::CatchUp.as_str(), "catch_up");
        assert!(AccountStatus::from_str("xyz").is_err());
    }
}
