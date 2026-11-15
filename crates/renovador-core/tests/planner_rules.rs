//! Testes de tabela do planejador (seção 5.4).

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use renovador_core::model::*;
use renovador_core::planner::*;

const SEED: u64 = 42;
const ACC: Id = 1;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// Instante a partir da hora local de Brasília.
fn local(y: i32, m: u32, day: u32, h: u32, min: u32) -> DateTime<Utc> {
    TZ.with_ymd_and_hms(y, m, day, h, min, 0)
        .unwrap()
        .with_timezone(&Utc)
}

fn account(mode: AccountMode, status: AccountStatus) -> Account {
    let t = local(2026, 1, 1, 12, 0);
    Account {
        id: ACC,
        library_base_url: "https://biblioteca.sophia.com.br/5362".into(),
        library_name: "UFTM".into(),
        login: "x".into(),
        has_password: true,
        mode,
        status,
        renew_days_before: 1,
        remind_days_before: 2,
        last_sync_at: None,
        last_renewal_at: None,
        next_renewal_at: None,
        created_at: t,
        updated_at: t,
    }
}

fn loan(id: Id, due: NaiveDate, auto_renew: bool) -> LoanView {
    let t = local(2026, 1, 1, 12, 0);
    LoanView {
        loan: Loan {
            id,
            book_id: id,
            loaned_at: None,
            due_date: due,
            renewals_count: 0,
            renewals_limit: None,
            status: LoanStatus::Active,
            auto_renew,
            returned_at: None,
            updated_at: t,
        },
        book: Book {
            id,
            account_id: ACC,
            item_key: format!("k{id}"),
            title: format!("Livro {id}"),
            author: None,
            call_number: None,
            library_branch: None,
            first_seen_at: t,
        },
    }
}

fn sched(
    id: Id,
    loan_id: Option<Id>,
    kind: ScheduleKind,
    run_at: DateTime<Utc>,
    status: ScheduleStatus,
    origin: ScheduleOrigin,
) -> Schedule {
    Schedule {
        id,
        account_id: ACC,
        loan_id,
        kind,
        run_at,
        status,
        origin,
        updated_at: run_at,
    }
}

fn run(now: DateTime<Utc>, acc: &Account, loans: &[LoanView], existing: &[Schedule]) -> Plan {
    plan(&PlanInput {
        now,
        account: acc,
        loans,
        existing,
        install_seed: SEED,
    })
    .unwrap()
}

fn of_kind(p: &Plan, kind: ScheduleKind) -> Vec<&Schedule> {
    p.create.iter().filter(|s| s.kind == kind).collect()
}

#[test]
fn renewal_date_table() {
    for (due, n, want) in [
        (d(2026, 3, 1), 1, d(2026, 2, 28)),  // fim de fevereiro
        (d(2028, 3, 1), 1, d(2028, 2, 29)),  // ano bissexto
        (d(2026, 1, 1), 1, d(2025, 12, 31)), // virada de ano
        (d(2026, 5, 1), 2, d(2026, 4, 29)),
        (d(2026, 5, 10), 0, d(2026, 5, 10)),
    ] {
        assert_eq!(renewal_date(due, n), want, "{due} − {n}");
    }
}

#[test]
fn window_minute_is_stable_and_in_range() {
    for seed in [0, 1, 42, u64::MAX] {
        for acc in [0, 1, 7, -3, Id::MAX] {
            let m = window_minute(seed, acc);
            assert!(m < 840);
            assert_eq!(m, window_minute(seed, acc));
        }
    }
    let distinct: std::collections::HashSet<_> = (0..50).map(|s| window_minute(s, 1)).collect();
    assert!(distinct.len() > 10, "semente deve variar o minuto");
}

#[test]
fn at_minute_falls_in_window_in_sao_paulo() {
    let at = at_minute(d(2026, 6, 15), 0);
    assert_eq!(at, local(2026, 6, 15, 7, 0));
    assert_eq!(at, Utc.with_ymd_and_hms(2026, 6, 15, 10, 0, 0).unwrap());
    assert_eq!(at_minute(d(2026, 6, 15), 839), local(2026, 6, 15, 20, 59));
    assert!(in_window(at_minute(
        d(2026, 6, 15),
        window_minute(SEED, ACC)
    )));
}

#[test]
fn next_in_window_table() {
    for (now, want) in [
        (local(2026, 6, 15, 6, 59), local(2026, 6, 15, 7, 0)),
        (local(2026, 6, 15, 7, 0), local(2026, 6, 15, 7, 0)),
        (local(2026, 6, 15, 20, 59), local(2026, 6, 15, 20, 59)),
        (local(2026, 6, 15, 21, 0), local(2026, 6, 16, 7, 0)),
        (local(2026, 6, 30, 23, 30), local(2026, 7, 1, 7, 0)), // fim de mês
        (local(2026, 6, 15, 0, 30), local(2026, 6, 15, 7, 0)), // madrugada (UTC já é outro dia)
    ] {
        assert_eq!(next_in_window(now), want, "{now}");
    }
}

#[test]
fn local_date_uses_sao_paulo() {
    // 23h30 em Brasília = 02h30 UTC do dia seguinte.
    assert_eq!(local_date(local(2026, 6, 15, 23, 30)), d(2026, 6, 15));
}

#[test]
fn renew_schedule_table() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    let minute = window_minute(SEED, ACC);
    let now = local(2026, 6, 15, 7, 0); // início da janela: o minuto planejado ainda não passou

    // (vencimento, deve haver renovação?, data local esperada)
    let cases = [
        ("vence amanhã", d(2026, 6, 16), true, d(2026, 6, 15)),
        ("vence em 5 dias", d(2026, 6, 20), true, d(2026, 6, 19)),
        (
            "vence hoje (catch-up)",
            d(2026, 6, 15),
            true,
            d(2026, 6, 15),
        ),
        ("vence 1º do mês", d(2026, 7, 1), true, d(2026, 6, 30)),
    ];
    for (name, due, _, want_date) in cases {
        let p = run(now, &acc, &[loan(10, due, true)], &[]);
        let renew = of_kind(&p, ScheduleKind::Renew);
        assert_eq!(renew.len(), 1, "{name}");
        assert_eq!(local_date(renew[0].run_at), want_date, "{name}");
        assert!(in_window(renew[0].run_at), "{name}");
        assert!(renew[0].run_at >= now, "{name}");
        assert_eq!(renew[0].origin, ScheduleOrigin::System);
        if want_date > d(2026, 6, 15) {
            assert_eq!(renew[0].run_at, at_minute(want_date, minute), "{name}");
        }
        assert!(of_kind(&p, ScheduleKind::Remind).is_empty(), "{name}");
    }

    // Vencido: nenhuma renovação, lembrete imediato.
    let p = run(now, &acc, &[loan(10, d(2026, 6, 14), true)], &[]);
    assert!(of_kind(&p, ScheduleKind::Renew).is_empty());
    let r = of_kind(&p, ScheduleKind::Remind);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].run_at, now);
}

#[test]
fn catch_up_when_planned_minute_passed() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    // Renovação era ontem; app ficou fechado. Agora 22h (fora da janela).
    let now = local(2026, 6, 15, 22, 0);
    let p = run(now, &acc, &[loan(10, d(2026, 6, 16), true)], &[]);
    let renew = of_kind(&p, ScheduleKind::Renew);
    assert_eq!(renew[0].run_at, local(2026, 6, 16, 7, 0));

    // Dentro da janela: executa já.
    let now = local(2026, 6, 15, 20, 59);
    let p = run(now, &acc, &[loan(10, d(2026, 6, 16), true)], &[]);
    assert_eq!(of_kind(&p, ScheduleKind::Renew)[0].run_at, now);
}

#[test]
fn reminders_d2_and_d0_without_auto_renew() {
    let now = local(2026, 6, 10, 7, 0);
    let minute = window_minute(SEED, ACC);
    for acc in [
        account(AccountMode::Manual, AccountStatus::Active),
        account(AccountMode::Auto, AccountStatus::Active),
    ] {
        let auto = acc.mode == AccountMode::Auto;
        let p = run(now, &acc, &[loan(10, d(2026, 7, 1), !auto)], &[]);
        // Conta auto com livro sem auto_renew, ou conta manual: lembretes.
        let p = if auto {
            run(now, &acc, &[loan(10, d(2026, 7, 1), false)], &[])
        } else {
            p
        };
        let r: Vec<_> = of_kind(&p, ScheduleKind::Remind)
            .iter()
            .map(|s| s.run_at)
            .collect();
        assert_eq!(
            r,
            vec![
                at_minute(d(2026, 6, 29), minute),
                at_minute(d(2026, 7, 1), minute)
            ]
        );
        assert!(of_kind(&p, ScheduleKind::Renew).is_empty());
    }

    // Vence amanhã: D−2 já passou, só D0.
    let acc = account(AccountMode::Manual, AccountStatus::Active);
    let p = run(now, &acc, &[loan(10, d(2026, 6, 11), false)], &[]);
    let r = of_kind(&p, ScheduleKind::Remind);
    assert_eq!(r.len(), 1);
    assert_eq!(local_date(r[0].run_at), d(2026, 6, 11));
}

#[test]
fn discovery_sync_every_three_days() {
    let now = local(2026, 6, 10, 12, 0);
    let mut acc = account(AccountMode::Auto, AccountStatus::Active);

    // Nunca sincronizou: agora.
    let p = run(now, &acc, &[], &[]);
    assert_eq!(of_kind(&p, ScheduleKind::Sync)[0].run_at, now);

    // Sincronizou há 1 dia: daqui a 2 dias.
    acc.last_sync_at = Some(now - Duration::days(1));
    let p = run(now, &acc, &[], &[]);
    assert_eq!(
        of_kind(&p, ScheduleKind::Sync)[0].run_at,
        now + Duration::days(2)
    );

    // Atrasada: agora.
    acc.last_sync_at = Some(now - Duration::days(10));
    let p = run(now, &acc, &[], &[]);
    assert_eq!(of_kind(&p, ScheduleKind::Sync)[0].run_at, now);

    // Última sync às 22h: a próxima cai fora da janela e é empurrada para 07h.
    acc.last_sync_at = Some(local(2026, 6, 9, 22, 0));
    let p = run(now, &acc, &[], &[]);
    assert_eq!(
        of_kind(&p, ScheduleKind::Sync)[0].run_at,
        local(2026, 6, 13, 7, 0)
    );
}

#[test]
fn user_paused_or_cancelled_is_never_recreated() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    let now = local(2026, 6, 15, 7, 0);
    let loans = [loan(10, d(2026, 6, 20), true)];
    for status in [ScheduleStatus::Paused, ScheduleStatus::Cancelled] {
        let existing = [
            sched(
                1,
                Some(10),
                ScheduleKind::Renew,
                local(2026, 6, 19, 9, 0),
                status,
                ScheduleOrigin::User,
            ),
            sched(
                2,
                None,
                ScheduleKind::Sync,
                local(2026, 6, 16, 9, 0),
                status,
                ScheduleOrigin::User,
            ),
        ];
        let p = run(now, &acc, &loans, &existing);
        assert!(p.create.is_empty(), "{status:?}: {:?}", p.create);
        assert!(p.cancel.is_empty(), "{status:?}");
    }

    // O mesmo de origem system é recriado.
    let existing = [sched(
        1,
        Some(10),
        ScheduleKind::Renew,
        local(2026, 6, 19, 9, 0),
        ScheduleStatus::Cancelled,
        ScheduleOrigin::System,
    )];
    let p = run(now, &acc, &loans, &existing);
    assert_eq!(of_kind(&p, ScheduleKind::Renew).len(), 1);
}

#[test]
fn existing_pending_is_kept_and_stale_is_cancelled() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    let now = local(2026, 6, 15, 7, 0);
    let loans = [loan(10, d(2026, 6, 20), true)];
    let existing = [
        sched(
            1,
            Some(10),
            ScheduleKind::Renew,
            local(2026, 6, 19, 9, 0),
            ScheduleStatus::Pending,
            ScheduleOrigin::System,
        ),
        // Empréstimo devolvido/desaparecido.
        sched(
            2,
            Some(99),
            ScheduleKind::Renew,
            local(2026, 6, 19, 9, 0),
            ScheduleStatus::Pending,
            ScheduleOrigin::System,
        ),
        // Do usuário: nunca cancelado pelo sistema.
        sched(
            3,
            Some(98),
            ScheduleKind::Remind,
            local(2026, 6, 19, 9, 0),
            ScheduleStatus::Pending,
            ScheduleOrigin::User,
        ),
    ];
    let p = run(now, &acc, &loans, &existing);
    assert!(of_kind(&p, ScheduleKind::Renew).is_empty());
    assert_eq!(p.cancel, vec![2]);
}

#[test]
fn inactive_account_gets_nothing_and_cancels_system_pending() {
    let now = local(2026, 6, 15, 7, 0);
    let loans = [loan(10, d(2026, 6, 20), true)];
    let existing = [sched(
        5,
        Some(10),
        ScheduleKind::Renew,
        local(2026, 6, 19, 9, 0),
        ScheduleStatus::Pending,
        ScheduleOrigin::System,
    )];
    for status in [
        AccountStatus::Paused,
        AccountStatus::Blocked,
        AccountStatus::InvalidPassword,
    ] {
        let p = run(now, &account(AccountMode::Auto, status), &loans, &existing);
        assert!(p.create.is_empty());
        assert_eq!(p.cancel, vec![5]);
    }
}

#[test]
fn returned_loans_are_ignored() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    let mut l = loan(10, d(2026, 6, 16), true);
    l.loan.status = LoanStatus::Returned;
    let p = run(local(2026, 6, 15, 7, 0), &acc, &[l], &[]);
    assert!(of_kind(&p, ScheduleKind::Renew).is_empty());
    assert!(of_kind(&p, ScheduleKind::Remind).is_empty());
}

#[test]
fn due_for_renewal_table() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    let now = local(2026, 6, 30, 10, 0);
    let loans = [
        loan(1, d(2026, 7, 1), true),  // amanhã (vira o mês): renova
        loan(2, d(2026, 6, 30), true), // hoje: renova (catch-up)
        loan(3, d(2026, 6, 29), true), // vencido: não
        loan(4, d(2026, 7, 5), true),  // cedo demais
        loan(5, d(2026, 7, 1), false), // sem auto_renew
    ];
    let ids: Vec<_> = due_for_renewal(now, &acc, &loans)
        .iter()
        .map(|l| l.loan.id)
        .collect();
    assert_eq!(ids, vec![1, 2]);

    // Fora da janela, conta manual ou pausada: nada.
    assert!(due_for_renewal(local(2026, 6, 30, 22, 0), &acc, &loans).is_empty());
    let manual = account(AccountMode::Manual, AccountStatus::Active);
    assert!(due_for_renewal(now, &manual, &loans).is_empty());
    let paused = account(AccountMode::Auto, AccountStatus::Paused);
    assert!(due_for_renewal(now, &paused, &loans).is_empty());
}

#[test]
fn retry_only_for_network_error() {
    let due = d(2026, 6, 16);
    let now = local(2026, 6, 15, 10, 0);
    let at = retry_after_failure(RenewalResult::NetworkError, due, now, SEED, ACC).unwrap();
    assert_eq!(at, at_minute(due, window_minute(SEED, ACC)));

    for r in [
        RenewalResult::Reserved,
        RenewalResult::LimitReached,
        RenewalResult::Blocked,
        RenewalResult::Overdue,
        RenewalResult::Captcha,
        RenewalResult::ParseError,
        RenewalResult::RateLimited,
        RenewalResult::Success,
    ] {
        assert_eq!(retry_after_failure(r, due, now, SEED, ACC), None, "{r:?}");
    }

    // No próprio dia do vencimento: daqui a 1 h, se couber na janela.
    let now = local(2026, 6, 16, 10, 0);
    assert_eq!(
        retry_after_failure(RenewalResult::NetworkError, due, now, SEED, ACC),
        Some(local(2026, 6, 16, 11, 0))
    );
    let late = local(2026, 6, 16, 20, 30);
    assert_eq!(
        retry_after_failure(RenewalResult::NetworkError, due, late, SEED, ACC),
        None
    );
    // Já vencido.
    let after = local(2026, 6, 17, 10, 0);
    assert_eq!(
        retry_after_failure(RenewalResult::NetworkError, due, after, SEED, ACC),
        None
    );
}

#[test]
fn foreign_schedule_is_rejected() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    let mut s = sched(
        1,
        None,
        ScheduleKind::Sync,
        local(2026, 6, 16, 9, 0),
        ScheduleStatus::Pending,
        ScheduleOrigin::System,
    );
    s.account_id = 2;
    let r = plan(&PlanInput {
        now: local(2026, 6, 15, 9, 0),
        account: &acc,
        loans: &[],
        existing: &[s],
        install_seed: SEED,
    });
    assert!(r.is_err());
}

#[test]
fn due_today_after_window_becomes_reminder() {
    let acc = account(AccountMode::Auto, AccountStatus::Active);
    let now = local(2026, 6, 16, 22, 0);
    let p = run(now, &acc, &[loan(10, d(2026, 6, 16), true)], &[]);
    assert!(of_kind(&p, ScheduleKind::Renew).is_empty());
    assert_eq!(of_kind(&p, ScheduleKind::Remind).len(), 1);
}
