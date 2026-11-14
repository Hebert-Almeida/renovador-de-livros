//! Motor HTTP do Sophia (seção 5.1). Cliente na W1-05, parser na W1-06.

pub mod client;
pub mod detect;
pub mod model;
pub mod parser;
pub mod selectors;

use async_trait::async_trait;

use crate::Result;
use crate::guard::SessionTicket;
pub use model::*;

/// Resposta HTTP bruta, já decodificada para texto.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub final_url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Transporte HTTP (reqwest em produção, fake/wiremock nos testes).
#[async_trait]
pub trait Transport: Send + Sync {
    async fn get(&self, url: &str) -> Result<HttpResponse>;
    async fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpResponse>;
}

/// Operações de alto nível no Sophia. Cada requisição passa pelo `RequestGate`.
#[async_trait]
pub trait SophiaClient: Send + Sync {
    /// Confere se o endereço é uma biblioteca Sophia (HTTPS + `x-powered-by`).
    async fn validate_library(&self, base_url: &str) -> Result<String>;
    async fn login(
        &self,
        ticket: &SessionTicket,
        base_url: &str,
        login: &str,
        password: &str,
    ) -> Result<LoginResult>;
    async fn list_loans(
        &self,
        ticket: &SessionTicket,
        session: &LoggedSession,
    ) -> Result<Vec<RemoteLoan>>;
    async fn renew(
        &self,
        ticket: &SessionTicket,
        session: &LoggedSession,
        loan: &RemoteLoan,
    ) -> Result<RenewOutcome>;
    async fn logout(&self, ticket: &SessionTicket, session: &LoggedSession) -> Result<()>;
}

/// User-Agent honesto (nunca imitar navegador).
pub fn user_agent() -> String {
    format!(
        "RenovadorLivrosUFTM/{} (+https://github.com/Hebert-Almeida/renovador-de-livros)",
        crate::VERSION
    )
}
