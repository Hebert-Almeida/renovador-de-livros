use serde::Serialize;

/// Erro do núcleo. Serializável para chegar ao frontend com `kind` estável.
#[derive(Debug, thiserror::Error, Serialize, specta::Type)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum Error {
    #[error("ainda não implementado: {0}")]
    NotImplemented(String),
    #[error("limite de uso atingido: {0}")]
    Guard(#[from] crate::guard::GuardError),
    #[error("erro de banco de dados: {0}")]
    Store(String),
    #[error("erro no cofre de chaves: {0}")]
    Keys(String),
    #[error("erro de rede: {0}")]
    Network(String),
    #[error("resposta inesperada do Sophia: {0}")]
    Parse(String),
    #[error("login ou senha inválidos")]
    InvalidCredentials,
    #[error("o site pediu verificação humana (CAPTCHA/Cloudflare); abra o site oficial")]
    HumanVerificationRequired,
    #[error("endereço não é uma biblioteca Sophia válida: {0}")]
    InvalidLibrary(String),
    #[error("entrada inválida: {0}")]
    Invalid(String),
    #[error("não encontrado: {0}")]
    NotFound(String),
}

impl Error {
    pub fn todo(what: &str) -> Self {
        Self::NotImplemented(what.to_owned())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
