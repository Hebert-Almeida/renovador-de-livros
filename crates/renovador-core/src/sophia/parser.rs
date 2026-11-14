//! Parser das páginas do Sophia (W1-06). Estrutura desconhecida → `Error::Parse`.

use super::model::*;
use crate::{Error, Result};

/// Lê o JSON do login. O `url` vem entre aspas: remove um caractere de cada ponta.
pub fn parse_login(_json: &str) -> Result<LoginResponse> {
    Err(Error::todo("sophia::parser::parse_login"))
}

pub fn parse_loan_list(_html: &str) -> Result<Vec<RemoteLoan>> {
    Err(Error::todo("sophia::parser::parse_loan_list"))
}

pub fn classify_renewal(_html: &str) -> Result<RenewOutcome> {
    Err(Error::todo("sophia::parser::classify_renewal"))
}

/// `true` se a página pede CAPTCHA ou desafio do Cloudflare.
pub fn needs_human_verification(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    super::selectors::HUMAN_VERIFICATION_MARKERS
        .iter()
        .any(|m| lower.contains(&m.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn detects_cloudflare_challenge() {
        assert!(super::needs_human_verification(
            "<title>Just a moment...</title>"
        ));
        assert!(!super::needs_human_verification("<h3>Livro</h3>"));
    }
}
