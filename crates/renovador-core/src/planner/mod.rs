//! Planejador (seção 5.4): funções puras, fuso America/Sao_Paulo. W1-04.
//!
//! Regras:
//! - renova em vencimento − N dias (`renew_days_before`), num minuto fixo da janela 07h–21h
//!   derivado da semente da instalação e da conta;
//! - se o horário planejado já passou e o livro ainda não venceu, executa o quanto antes
//!   (catch-up), respeitando a janela;
//! - livros vencidos não são renovados (o Sophia recusa) e recebem lembrete;
//! - lembretes em D−2 e D0 para livros sem renovação automática;
//! - sincronização de descoberta a cada 3 dias;
//! - nova tentativa no dia do vencimento só para falha de rede ([`retry_after_failure`]);
//! - agendamentos `origin=user` pausados ou cancelados nunca são recriados.

use chrono::{DateTime, Days, Duration, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::America::Sao_Paulo;

use crate::model::*;
use crate::{Error, Result};

pub const TZ: chrono_tz::Tz = Sao_Paulo;
/// Intervalo da sincronização de descoberta.
pub const DISCOVERY_SYNC_DAYS: i64 = 3;
/// Início da janela de execução (hora local, inclusiva).
pub const WINDOW_START_HOUR: u32 = 7;
/// Fim da janela de execução (hora local, exclusiva).
pub const WINDOW_END_HOUR: u32 = 21;
/// Dias antes do vencimento em que sai o primeiro lembrete.
pub const REMIND_EARLY_DAYS: u64 = 2;

const WINDOW_MINUTES: u32 = (WINDOW_END_HOUR - WINDOW_START_HOUR) * 60;

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

/// Calcula os agendamentos desejados para a conta e compara com os existentes.
///
/// Os agendamentos criados têm `id = 0` (o `Store` atribui o id real). Só agendamentos
/// pendentes de origem `system` são cancelados; os do usuário nunca são tocados.
pub fn plan(input: &PlanInput<'_>) -> Result<Plan> {
    let acc = input.account;
    let now = input.now;
    if input.existing.iter().any(|s| s.account_id != acc.id) {
        return Err(Error::Invalid(
            "agendamento de outra conta na entrada do planejador".into(),
        ));
    }

    let mut out = Plan::default();
    let system_pending =
        |s: &&Schedule| s.origin == ScheduleOrigin::System && s.status == ScheduleStatus::Pending;

    // Contas pausadas, bloqueadas ou com senha inválida não recebem agendamentos novos.
    if acc.status != AccountStatus::Active {
        out.cancel = input
            .existing
            .iter()
            .filter(system_pending)
            .map(|s| s.id)
            .collect();
        return Ok(out);
    }

    let today = local_date(now);
    let minute = window_minute(input.install_seed, acc.id);
    let mut wanted: Vec<(ScheduleKind, Option<Id>, DateTime<Utc>)> = Vec::new();

    // Sincronização de descoberta.
    let sync_at = acc
        .last_sync_at
        .map_or(now, |last| last + Duration::days(DISCOVERY_SYNC_DAYS));
    wanted.push((ScheduleKind::Sync, None, next_in_window(sync_at.max(now))));

    for lv in input.loans {
        let loan = &lv.loan;
        if loan.status == LoanStatus::Returned || loan.returned_at.is_some() {
            continue;
        }
        let id = Some(loan.id);
        if loan.due_date < today || loan.status == LoanStatus::Overdue {
            // Vencido: lembrete o quanto antes, sem tentativa de renovação.
            wanted.push((ScheduleKind::Remind, id, next_in_window(now)));
            continue;
        }
        if acc.mode == AccountMode::Auto && loan.auto_renew {
            let date = renewal_date(loan.due_date, acc.renew_days_before).max(today);
            let at = catch_up(at_minute(date, minute), now);
            if local_date(at) > loan.due_date {
                // A janela de hoje já fechou e amanhã o livro estará vencido.
                wanted.push((ScheduleKind::Remind, id, next_in_window(now)));
            } else {
                wanted.push((ScheduleKind::Renew, id, at));
            }
        } else {
            for at in reminder_times(loan.due_date, minute, now) {
                wanted.push((ScheduleKind::Remind, id, at));
            }
        }
    }

    for s in input.existing.iter().filter(system_pending) {
        let still_wanted = wanted
            .iter()
            .any(|&(k, l, at)| k == s.kind && l == s.loan_id && same_slot(k, s.run_at, at));
        if !still_wanted {
            out.cancel.push(s.id);
        }
    }

    for (kind, loan_id, run_at) in wanted {
        if is_covered(input.existing, kind, loan_id, run_at) {
            continue;
        }
        out.create.push(Schedule {
            id: 0,
            account_id: acc.id,
            loan_id,
            kind,
            run_at,
            status: ScheduleStatus::Pending,
            origin: ScheduleOrigin::System,
            updated_at: now,
        });
    }
    Ok(out)
}

/// Dois horários representam o mesmo agendamento? Renovação/sincronização: um por
/// empréstimo/conta; lembretes: um por dia local.
fn same_slot(kind: ScheduleKind, a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
    match kind {
        ScheduleKind::Remind => local_date(a) == local_date(b),
        ScheduleKind::Renew | ScheduleKind::Sync => true,
    }
}

/// `true` se um agendamento existente já cobre (ou bloqueia) o desejado.
fn is_covered(
    existing: &[Schedule],
    kind: ScheduleKind,
    loan_id: Option<Id>,
    run_at: DateTime<Utc>,
) -> bool {
    existing
        .iter()
        .filter(|s| s.kind == kind && s.loan_id == loan_id)
        .any(|s| match (s.origin, s.status) {
            // Decisão do usuário: nunca recriar.
            (ScheduleOrigin::User, ScheduleStatus::Paused | ScheduleStatus::Cancelled) => true,
            // Um pendente (do usuário ou do sistema) já cobre o mesmo slot.
            (ScheduleOrigin::User, ScheduleStatus::Pending) => true,
            (_, ScheduleStatus::Pending) => same_slot(kind, s.run_at, run_at),
            // Já executado no mesmo dia local: não repetir.
            (_, ScheduleStatus::Done | ScheduleStatus::Failed) => {
                local_date(s.run_at) == local_date(run_at)
            }
            _ => false,
        })
}

/// Data em que um empréstimo deve ser renovado (vencimento − N dias).
pub fn renewal_date(due: NaiveDate, days_before: u8) -> NaiveDate {
    due - chrono::Days::new(u64::from(days_before))
}

/// Empréstimos que devem ser renovados agora.
///
/// Inclui catch-up (a data de renovação já passou, mas o vencimento não). Respeita a
/// janela 07h–21h e ignora contas manuais/inativas, livros sem renovação automática
/// e livros vencidos.
pub fn due_for_renewal<'a>(
    now: DateTime<Utc>,
    account: &Account,
    loans: &'a [LoanView],
) -> Vec<&'a LoanView> {
    if account.mode != AccountMode::Auto
        || account.status != AccountStatus::Active
        || !in_window(now)
    {
        return Vec::new();
    }
    let today = local_date(now);
    loans
        .iter()
        .filter(|lv| {
            let l = &lv.loan;
            l.status == LoanStatus::Active
                && l.returned_at.is_none()
                && l.auto_renew
                && l.due_date >= today
                && renewal_date(l.due_date, account.renew_days_before) <= today
        })
        .collect()
}

/// Nova tentativa após uma falha de renovação: **só** para erro de rede, no dia do
/// vencimento, dentro da janela. Reserva, limite, bloqueio etc. retornam `None`.
pub fn retry_after_failure(
    result: RenewalResult,
    due: NaiveDate,
    now: DateTime<Utc>,
    install_seed: u64,
    account_id: Id,
) -> Option<DateTime<Utc>> {
    if result != RenewalResult::NetworkError {
        return None;
    }
    let today = local_date(now);
    let at = if today < due {
        at_minute(due, window_minute(install_seed, account_id))
    } else if today == due {
        next_in_window(now + Duration::hours(1))
    } else {
        return None;
    };
    (local_date(at) == due).then_some(at)
}

/// Horários dos lembretes D−2 e D0 ainda válidos. Um lembrete de hoje cujo minuto
/// já passou sai assim que possível (catch-up), se ainda couber na janela de hoje.
pub fn reminder_times(due: NaiveDate, minute: u32, now: DateTime<Utc>) -> Vec<DateTime<Utc>> {
    let today = local_date(now);
    let mut out = Vec::new();
    for date in [due - Days::new(REMIND_EARLY_DAYS), due] {
        if date < today {
            continue;
        }
        let at = catch_up(at_minute(date, minute), now);
        if local_date(at) == date {
            out.push(at);
        }
    }
    out
}

/// Minuto (a partir das 07:00) fixo por instalação e conta, em `0..840`.
pub fn window_minute(install_seed: u64, account_id: Id) -> u32 {
    // splitmix64: determinístico e bem distribuído, sem dependência nova.
    let mut z = install_seed ^ u64::from(account_id as u32).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z % u64::from(WINDOW_MINUTES)) as u32
}

/// Instante UTC de `date` às 07:00 + `minute` no horário de Brasília.
pub fn at_minute(date: NaiveDate, minute: u32) -> DateTime<Utc> {
    let minute = minute % WINDOW_MINUTES;
    let t = NaiveTime::from_hms_opt(WINDOW_START_HOUR + minute / 60, minute % 60, 0)
        .expect("hora válida");
    local_to_utc(date.and_time(t))
}

/// Data civil em America/Sao_Paulo.
pub fn local_date(at: DateTime<Utc>) -> NaiveDate {
    at.with_timezone(&TZ).date_naive()
}

/// `true` se `at` cai na janela 07h–21h local.
pub fn in_window(at: DateTime<Utc>) -> bool {
    (WINDOW_START_HOUR..WINDOW_END_HOUR).contains(&at.with_timezone(&TZ).hour())
}

/// Primeiro instante ≥ `at` dentro da janela.
pub fn next_in_window(at: DateTime<Utc>) -> DateTime<Utc> {
    if in_window(at) {
        return at;
    }
    let local = at.with_timezone(&TZ);
    let date = if local.hour() < WINDOW_START_HOUR {
        local.date_naive()
    } else {
        local.date_naive() + Days::new(1)
    };
    at_minute(date, 0)
}

/// Catch-up: se o horário planejado já passou, executa assim que possível.
fn catch_up(planned: DateTime<Utc>, now: DateTime<Utc>) -> DateTime<Utc> {
    if planned >= now {
        planned
    } else {
        next_in_window(now)
    }
}

fn local_to_utc(naive: chrono::NaiveDateTime) -> DateTime<Utc> {
    // Sem horário de verão desde 2019, mas tratamos lacunas/ambiguidades por segurança.
    TZ.from_local_datetime(&naive)
        .earliest()
        .or_else(|| {
            TZ.from_local_datetime(&(naive + Duration::hours(1)))
                .earliest()
        })
        .expect("hora local válida")
        .with_timezone(&Utc)
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
