use chrono::NaiveDate;
use serde::Deserialize;

/// JSON de `ajx_efetua_login.asp`: `{login_valido, mensagem, url, abrir_popup}`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub login_valido: String,
    #[serde(default)]
    pub mensagem: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub abrir_popup: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum LoginResult {
    Ok(LoggedSession),
    InvalidCredentials { message: String },
}

/// Sessão autenticada (cookies ficam no transporte).
#[derive(Debug, Clone)]
pub struct LoggedSession {
    pub base_url: String,
    /// URL relativa da lista de empréstimos (já sem as aspas das pontas).
    pub loans_path: String,
}

/// Empréstimo como aparece no site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteLoan {
    pub item_key: String,
    pub title: String,
    pub due_date: NaiveDate,
    pub renewals_count: Option<u32>,
    /// Link GET de renovação, se a renovação estiver disponível.
    pub renew_url: Option<String>,
}

/// Classificação da resposta de renovação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenewOutcome {
    Success { new_due_date: Option<NaiveDate> },
    Reserved { message: String },
    LimitReached { message: String },
    Blocked { message: String },
    Overdue { message: String },
}
