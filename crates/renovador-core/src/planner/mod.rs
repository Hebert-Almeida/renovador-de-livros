//! Planejador (seção 5.4): funções puras, fuso America/Sao_Paulo. W1-04.

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::America::Sao_Paulo;

use crate::model::*;
use crate::{Error, Result};

pub const TZ: chrono_tz::Tz = Sao_Paulo;
/// Intervalo da sincronização de descoberta.
pub const DISCOVERY_SYNC_DAYS: i64 = 3;

/// Entrada do planejador para uma conta.
#[derive(Debug, Clone)]
pub struct PlanInput<'a> {
    pub now: DateTime<Utc>,
    pub account: &'a Account,
    pub loans: &'a [LoanView],
    pub existing: &'a [Schedule],
    /// Semente fixa por instalação (minuto aleatório na janela 07h–21h).
    pub install_seed: u64,
}

/// O que deve ser criado/alterado na agenda.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub create: Vec<Schedule>,
    pub cancel: Vec<Id>,
}

pub fn plan(_input: &PlanInput<'_>) -> Result<Plan> {
    Err(Error::todo("planner::plan"))
}

/// Data em que um empréstimo deve ser renovado (vencimento − N dias).
pub fn renewal_date(due: NaiveDate, days_before: u8) -> NaiveDate {
    due - chrono::Days::new(u64::from(days_before))
}

/// Empréstimos que devem ser renovados agora.
pub fn due_for_renewal<'a>(
    _now: DateTime<Utc>,
    _account: &Account,
    _loans: &'a [LoanView],
) -> Vec<&'a LoanView> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    #[test]
    fn renewal_date_subtracts_days() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(
            super::renewal_date(d, 1),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
    }
}
