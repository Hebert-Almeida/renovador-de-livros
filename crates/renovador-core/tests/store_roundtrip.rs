//! Testes de ida e volta do `Store` (W1-02).

use chrono::{Duration, NaiveDate, TimeZone, Utc};
use renovador_core::guard::{BreakerState, GuardPersistence, RequestOutcome, RequestRecord};
use renovador_core::model::*;
use renovador_core::store::{DbKey, Store, session_start_kind};
use zeroize::Zeroizing;

fn key(b: u8) -> DbKey {
    Zeroizing::new([b; 32])
}

fn new_account(login: &str, password: Option<&str>) -> NewAccount {
    NewAccount {
        library_base_url: "https://biblioteca.exemplo.test/1".into(),
        library_name: "Biblioteca Teste".into(),
        login: login.into(),
        password: password.map(Into::into),
        mode: AccountMode::Auto,
        renew_days_before: 1,
        remind_days_before: 2,
    }
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn loan(book_id: Id, due: NaiveDate) -> Loan {
    Loan {
        id: 0,
        book_id,
        loaned_at: Some(Utc.with_ymd_and_hms(2026, 1, 2, 12, 0, 0).unwrap()),
        due_date: due,
        renewals_count: 0,
        renewals_limit: Some(3),
        status: LoanStatus::Active,
        auto_renew: true,
        returned_at: None,
        updated_at: Utc::now(),
    }
}

#[test]
fn file_is_encrypted_and_needs_the_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite");
    {
        let s = Store::open(&path, &key(7)).unwrap();
        s.account_create(&new_account("segredo", Some("pw")))
            .unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.starts_with(b"SQLite format 3"));
    assert!(Store::open(&path, &key(8)).is_err(), "chave errada");
    // Sem chave nenhuma (SQLite puro) também falha.
    let plain = rusqlite::Connection::open(&path).unwrap();
    assert!(
        plain
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| r
                .get::<_, i64>(0))
            .is_err()
    );
    drop(plain);
    let s = Store::open(&path, &key(7)).unwrap();
    assert_eq!(s.account_list().unwrap()[0].login, "segredo");
}

#[test]
fn accounts_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::open(&dir.path().join("db"), &key(1)).unwrap();
    let a = s.account_create(&new_account("ana", Some("pw1"))).unwrap();
    assert!(a.has_password);
    assert_eq!(a.status, AccountStatus::Active);
    assert_eq!(s.account_password(a.id).unwrap().unwrap().as_str(), "pw1");
    let b = s.account_create(&new_account("bia", None)).unwrap();
    assert!(!b.has_password);
    assert!(s.account_password(b.id).unwrap().is_none());
    assert!(
        s.account_create(&new_account("ana", None)).is_err(),
        "UNIQUE"
    );

    let up = s
        .account_update(
            a.id,
            &AccountPatch {
                library_name: Some("Outra".into()),
                status: Some(AccountStatus::Paused),
                renew_days_before: Some(3),
                clear_password: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(up.library_name, "Outra");
    assert_eq!(up.status, AccountStatus::Paused);
    assert_eq!(up.renew_days_before, 3);
    assert!(!up.has_password);
    let up = s
        .account_update(
            a.id,
            &AccountPatch {
                password: Some("novo".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(up.has_password);
    assert!(
        s.account_update(
            a.id,
            &AccountPatch {
                renew_days_before: Some(9),
                ..Default::default()
            }
        )
        .is_err(),
        "CHECK"
    );
    assert_eq!(s.account_list().unwrap().len(), 2);
    s.account_delete(b.id).unwrap();
    assert!(matches!(
        s.account_get(b.id),
        Err(renovador_core::Error::NotFound(_))
    ));
    assert!(s.account_delete(b.id).is_err());
}

#[test]
fn books_loans_events_schedules_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::open(&dir.path().join("db"), &key(2)).unwrap();
    let acc = s.account_create(&new_account("ana", None)).unwrap();
    let b1 = s.book_upsert(acc.id, "K1", "Dom Casmurro").unwrap();
    let b1b = s
        .book_upsert(acc.id, "K1", "Dom Casmurro (2ª ed.)")
        .unwrap();
    assert_eq!(b1.id, b1b.id);
    assert_eq!(b1b.title, "Dom Casmurro (2ª ed.)");
    let b2 = s.book_upsert(acc.id, "K2", "Iracema").unwrap();

    let l1 = s.loan_upsert(&loan(b1.id, d(2026, 2, 10))).unwrap();
    assert!(l1.id > 0);
    assert_eq!(l1.due_date, d(2026, 2, 10));
    // Sem id: reaproveita o empréstimo em aberto do mesmo livro.
    let mut again = loan(b1.id, d(2026, 2, 17));
    again.renewals_count = 1;
    let l1b = s.loan_upsert(&again).unwrap();
    assert_eq!(l1b.id, l1.id);
    assert_eq!(l1b.renewals_count, 1);
    let l2 = s.loan_upsert(&loan(b2.id, d(2026, 2, 5))).unwrap();

    let all = s.loan_list(&LoanFilter::default()).unwrap();
    assert_eq!(
        all.iter().map(|v| v.loan.id).collect::<Vec<_>>(),
        [l2.id, l1.id]
    );
    let found = s
        .loan_list(&LoanFilter {
            search: Some("casm".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].book.item_key, "K1");

    s.loan_set_auto_renew(l2.id, false).unwrap();
    s.loan_mark_returned(l2.id).unwrap();
    let l2d = s.loan_get(l2.id).unwrap();
    assert!(!l2d.view.loan.auto_renew);
    assert_eq!(l2d.view.loan.status, LoanStatus::Returned);
    assert!(l2d.view.loan.returned_at.is_some());
    let active = s
        .loan_list(&LoanFilter {
            status: Some(LoanStatus::Active),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(active.len(), 1);

    let t0 = Utc.with_ymd_and_hms(2026, 2, 9, 10, 0, 0).unwrap();
    let ev = s
        .event_insert(&RenewalEvent {
            id: 0,
            loan_id: l1.id,
            attempted_at: t0,
            trigger: Trigger::CatchUp,
            result: RenewalResult::Success,
            old_due_date: Some(d(2026, 2, 10)),
            new_due_date: Some(d(2026, 2, 17)),
            message: Some("ok".into()),
        })
        .unwrap();
    assert_eq!(ev.attempted_at, t0);
    assert_eq!(ev.trigger, Trigger::CatchUp);
    s.event_insert(&RenewalEvent {
        id: 0,
        loan_id: l1.id,
        attempted_at: Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap(),
        trigger: Trigger::Auto,
        result: RenewalResult::LimitReached,
        old_due_date: None,
        new_due_date: None,
        message: None,
    })
    .unwrap();

    let page = s
        .history_list(
            &HistoryFilter {
                account_id: Some(acc.id),
                ..Default::default()
            },
            Page {
                offset: 0,
                limit: 1,
            },
        )
        .unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].result, RenewalResult::LimitReached);
    let ok = s
        .history_list(
            &HistoryFilter {
                result: Some(RenewalResult::Success),
                ..Default::default()
            },
            Page {
                offset: 0,
                limit: 10,
            },
        )
        .unwrap();
    assert_eq!(ok.total, 1);

    let range = DateRange {
        from: d(2026, 1, 1),
        to: d(2026, 12, 31),
    };
    let ov = s.stats_overview(range).unwrap();
    assert_eq!(ov.renewals_total, 2);
    assert_eq!(ov.days_gained, 7);
    assert!((ov.success_rate - 0.5).abs() < 1e-9);
    assert_eq!(ov.active_loans, 1);
    assert_eq!(ov.distinct_books, 2);
    assert_eq!(ov.most_renewed.unwrap().id, b1.id);
    let monthly = s.stats_monthly(range).unwrap();
    assert_eq!(monthly.len(), 2);
    assert_eq!(monthly[0].month, "2026-02");
    assert_eq!(monthly[1].failures, 1);

    let run_at = Utc.with_ymd_and_hms(2026, 2, 16, 10, 0, 0).unwrap();
    let sc = s
        .schedule_insert(&Schedule {
            id: 0,
            account_id: acc.id,
            loan_id: Some(l1.id),
            kind: ScheduleKind::Renew,
            run_at,
            status: ScheduleStatus::Pending,
            origin: ScheduleOrigin::System,
            updated_at: Utc::now(),
        })
        .unwrap();
    assert_eq!(sc.run_at, run_at);
    let sc2 = s
        .schedule_set_status(sc.id, ScheduleStatus::Paused, ScheduleOrigin::User)
        .unwrap();
    assert_eq!(sc2.status, ScheduleStatus::Paused);
    assert_eq!(sc2.origin, ScheduleOrigin::User);
    let later = run_at + Duration::hours(3);
    assert_eq!(s.schedule_reschedule(sc.id, later).unwrap().run_at, later);
    assert_eq!(
        s.schedule_list(&ScheduleFilter {
            status: Some(ScheduleStatus::Paused),
            ..Default::default()
        })
        .unwrap()
        .len(),
        1
    );
    let detail = s.loan_get(l1.id).unwrap();
    assert_eq!(detail.events.len(), 2);
    assert_eq!(detail.schedules.len(), 1);

    assert_eq!(
        s.history_clear(Some(Utc.with_ymd_and_hms(2026, 2, 20, 0, 0, 0).unwrap()))
            .unwrap(),
        1
    );
    assert_eq!(s.history_clear(None).unwrap(), 1);

    // ON DELETE CASCADE (foreign_keys ligado).
    s.account_delete(acc.id).unwrap();
    assert!(s.loan_list(&LoanFilter::default()).unwrap().is_empty());
    assert!(
        s.schedule_list(&ScheduleFilter::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn settings_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let s = Store::open(&path, &key(3)).unwrap();
    let def = s.settings_get().unwrap();
    assert_eq!(def.theme, Theme::System);
    let at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let out = s
        .settings_set(&SettingsPatch {
            theme: Some(Theme::Contrast),
            notifications: Some(false),
            privacy_accepted_at: Some(at),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(out.theme, Theme::Contrast);
    assert!(!out.notifications);
    drop(s);
    let s = Store::open(&path, &key(3)).unwrap();
    let got = s.settings_get().unwrap();
    assert_eq!(got.theme, Theme::Contrast);
    assert_eq!(got.privacy_accepted_at, Some(at));
    assert!(got.autostart);
}

#[test]
fn guard_persistence_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let t0 = Utc.with_ymd_and_hms(2026, 5, 1, 10, 0, 0).unwrap();
    {
        let s = Store::open(&path, &key(4)).unwrap();
        let acc = s.account_create(&new_account("ana", None)).unwrap();
        let rec = |kind: String, sid: &str, at| RequestRecord {
            account_id: Some(acc.id),
            host: "biblioteca.exemplo.test".into(),
            at,
            kind,
            session_id: sid.into(),
            http_status: Some(200),
            outcome: RequestOutcome::Ok,
        };
        s.record_request(&rec(session_start_kind(Trigger::Manual), "s1", t0))
            .unwrap();
        s.record_request(&rec("login".into(), "s1", t0)).unwrap();
        s.record_request(&rec("list".into(), "s1", t0 + Duration::seconds(3)))
            .unwrap();
        s.record_request(&rec(
            session_start_kind(Trigger::Auto),
            "s2",
            t0 + Duration::hours(1),
        ))
        .unwrap();
        s.record_request(&rec("login".into(), "s2", t0 + Duration::hours(1)))
            .unwrap();
        s.set_breaker(
            "host:biblioteca.exemplo.test",
            &BreakerState {
                consecutive_failures: 3,
                open_until: Some(t0 + Duration::hours(6)),
                reason: Some("falhas".into()),
            },
        )
        .unwrap();
    }
    let s = Store::open(&path, &key(4)).unwrap();
    let id = s.account_list().unwrap()[0].id;
    assert_eq!(s.count_sessions(Some(id), None, t0).unwrap(), 2);
    assert_eq!(
        s.count_sessions(Some(id), Some(Trigger::Manual), t0)
            .unwrap(),
        1
    );
    assert_eq!(
        s.count_sessions(None, Some(Trigger::CatchUp), t0).unwrap(),
        0
    );
    assert_eq!(
        s.count_sessions(None, None, t0 + Duration::minutes(30))
            .unwrap(),
        1
    );
    assert_eq!(
        s.last_session_start(id, Trigger::Auto).unwrap(),
        Some(t0 + Duration::hours(1))
    );
    assert_eq!(s.last_session_start(id, Trigger::CatchUp).unwrap(), None);
    assert_eq!(s.distinct_logins_since(t0).unwrap(), 1);
    let br = s.breaker("host:biblioteca.exemplo.test").unwrap();
    assert_eq!(br.consecutive_failures, 3);
    assert_eq!(br.open_until, Some(t0 + Duration::hours(6)));
    assert_eq!(s.breaker("account:99").unwrap().consecutive_failures, 0);
    let recent = s.requests_recent(2).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].outcome, "ok");
}

#[test]
fn backup_restore_and_wipe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let bk = dir.path().join("backup.rlb");
    let s = Store::open(&path, &key(5)).unwrap();
    let acc = s.account_create(&new_account("ana", Some("pw"))).unwrap();
    let b = s.book_upsert(acc.id, "K1", "Livro").unwrap();
    s.loan_upsert(&loan(b.id, d(2026, 2, 10))).unwrap();
    s.settings_set(&SettingsPatch {
        theme: Some(Theme::Dark),
        ..Default::default()
    })
    .unwrap();
    assert!(s.backup_create(&bk, "").is_err());
    s.backup_create(&bk, "senha forte").unwrap();
    assert!(!std::fs::read(&bk).unwrap().starts_with(b"SQLite format 3"));

    // Muda tudo e restaura.
    s.account_delete(acc.id).unwrap();
    s.account_create(&new_account("intruso", None)).unwrap();
    assert!(s.backup_restore(&bk, "senha errada").is_err());
    assert_eq!(s.account_list().unwrap()[0].login, "intruso", "nada mudou");
    s.backup_restore(&bk, "senha forte").unwrap();
    let accs = s.account_list().unwrap();
    assert_eq!(accs.len(), 1);
    assert_eq!(accs[0].login, "ana");
    assert_eq!(
        s.account_password(accs[0].id).unwrap().unwrap().as_str(),
        "pw"
    );
    assert_eq!(s.loan_list(&LoanFilter::default()).unwrap().len(), 1);
    assert_eq!(s.settings_get().unwrap().theme, Theme::Dark);

    // Restaurar num banco novo, com outra chave.
    let other = Store::open(&dir.path().join("db2"), &key(6)).unwrap();
    other.backup_restore(&bk, "senha forte").unwrap();
    assert_eq!(other.account_list().unwrap()[0].login, "ana");

    s.wipe().unwrap();
    assert!(!path.exists());
    assert!(!dir.path().join("db-wal").exists());
    let fresh = Store::open(&path, &key(5)).unwrap();
    assert!(fresh.account_list().unwrap().is_empty());
}
