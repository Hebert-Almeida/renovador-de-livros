//! Banco SQLCipher. API congelada na Onda 0; implementação na W1-02.

mod convert;
mod guard_impl;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::{Duration, Utc};
use rusqlite::{Connection, OptionalExtension, Row, ToSql, params, params_from_iter};
use zeroize::Zeroizing;

use crate::model::*;
use crate::{Error, Result};
use convert::*;

pub use guard_impl::session_start_kind;

/// Migrações aplicadas em ordem via `PRAGMA user_version`.
pub const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_init.sql")];

/// Chave do banco: 32 bytes aleatórios, zerada ao sair de escopo.
pub type DbKey = Zeroizing<[u8; 32]>;

/// Tabelas em ordem de dependência (pais antes dos filhos).
const TABLES: &[&str] = &[
    "account",
    "book",
    "loan",
    "renewal_event",
    "schedule",
    "request_log",
    "breaker_state",
    "setting",
];

const ACCOUNT_COLS: &str = "id, library_base_url, library_name, login, password IS NOT NULL, mode, \
    status, renew_days_before, remind_days_before, last_sync_at, last_renewal_at, \
    next_renewal_at, created_at, updated_at";
const BOOK_COLS: &str = "b.id, b.account_id, b.item_key, b.title, b.author, b.call_number, \
    b.library_branch, b.first_seen_at";
const LOAN_COLS: &str = "l.id, l.book_id, l.loaned_at, l.due_date, l.renewals_count, \
    l.renewals_limit, l.status, l.auto_renew, l.returned_at, l.updated_at";
const EVENT_COLS: &str = "e.id, e.loan_id, e.attempted_at, e.trigger, e.result, e.old_due_date, \
    e.new_due_date, e.message";
const SCHEDULE_COLS: &str = "id, account_id, loan_id, kind, run_at, status, origin, updated_at";

pub struct Store {
    pub(crate) conn: Mutex<Connection>,
    pub(crate) path: PathBuf,
}

fn row_account(r: &Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: r.get(0)?,
        library_base_url: r.get(1)?,
        library_name: r.get(2)?,
        login: r.get(3)?,
        has_password: r.get(4)?,
        mode: get_enum(r, 5)?,
        status: get_enum(r, 6)?,
        renew_days_before: r.get(7)?,
        remind_days_before: r.get(8)?,
        last_sync_at: get_ts_opt(r, 9)?,
        last_renewal_at: get_ts_opt(r, 10)?,
        next_renewal_at: get_ts_opt(r, 11)?,
        created_at: get_ts(r, 12)?,
        updated_at: get_ts(r, 13)?,
    })
}

fn row_book(r: &Row<'_>, o: usize) -> rusqlite::Result<Book> {
    Ok(Book {
        id: r.get(o)?,
        account_id: r.get(o + 1)?,
        item_key: r.get(o + 2)?,
        title: r.get(o + 3)?,
        author: r.get(o + 4)?,
        call_number: r.get(o + 5)?,
        library_branch: r.get(o + 6)?,
        first_seen_at: get_ts(r, o + 7)?,
    })
}

fn row_loan(r: &Row<'_>) -> rusqlite::Result<Loan> {
    Ok(Loan {
        id: r.get(0)?,
        book_id: r.get(1)?,
        loaned_at: get_ts_opt(r, 2)?,
        due_date: get_date(r, 3)?,
        renewals_count: r.get(4)?,
        renewals_limit: r.get(5)?,
        status: get_enum(r, 6)?,
        auto_renew: r.get(7)?,
        returned_at: get_ts_opt(r, 8)?,
        updated_at: get_ts(r, 9)?,
    })
}

fn row_loan_view(r: &Row<'_>) -> rusqlite::Result<LoanView> {
    Ok(LoanView {
        loan: row_loan(r)?,
        book: row_book(r, 10)?,
    })
}

fn row_event(r: &Row<'_>) -> rusqlite::Result<RenewalEvent> {
    Ok(RenewalEvent {
        id: r.get(0)?,
        loan_id: r.get(1)?,
        attempted_at: get_ts(r, 2)?,
        trigger: get_enum(r, 3)?,
        result: get_enum(r, 4)?,
        old_due_date: get_date_opt(r, 5)?,
        new_due_date: get_date_opt(r, 6)?,
        message: r.get(7)?,
    })
}

fn row_schedule(r: &Row<'_>) -> rusqlite::Result<Schedule> {
    Ok(Schedule {
        id: r.get(0)?,
        account_id: r.get(1)?,
        loan_id: r.get(2)?,
        kind: get_enum(r, 3)?,
        run_at: get_ts(r, 4)?,
        status: get_enum(r, 5)?,
        origin: get_enum(r, 6)?,
        updated_at: get_ts(r, 7)?,
    })
}

/// Converte o rowid (i64) do SQLite para [`Id`] (i32).
fn to_id(rowid: i64) -> Result<Id> {
    Id::try_from(rowid).map_err(|_| Error::Store(format!("id fora do intervalo: {rowid}")))
}

fn not_found(what: &str, id: Id) -> Error {
    Error::NotFound(format!("{what} {id}"))
}

fn collect<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    rows.collect::<rusqlite::Result<_>>().db()
}

/// Aplica a chave bruta (formato `x'..'` do SQLCipher).
fn apply_key(conn: &Connection, key: &[u8; 32]) -> Result<()> {
    use std::fmt::Write;
    let mut sql = Zeroizing::new(String::with_capacity(96));
    sql.push_str("PRAGMA key = \"x'");
    for b in key.iter() {
        let _ = write!(sql, "{b:02x}");
    }
    sql.push_str("'\";");
    conn.execute_batch(&sql).db()
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .db()?;
    let version = usize::try_from(version).map_err(|_| Error::Store("user_version".into()))?;
    if version > MIGRATIONS.len() {
        return Err(Error::Store(format!(
            "banco criado por versão mais nova (schema {version})"
        )));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.transaction().db()?;
        tx.execute_batch(sql).db()?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)
            .db()?;
        tx.commit().db()?;
    }
    Ok(())
}

impl Store {
    /// Abre (ou cria) o banco criptografado e roda as migrações.
    pub fn open(path: &Path, key: &DbKey) -> Result<Self> {
        let mut conn = Connection::open(path).db()?;
        apply_key(&conn, key)?;
        // Falha aqui se a chave estiver errada ("file is not a database").
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|e| Error::Store(format!("não foi possível abrir o banco: {e}")))?;
        let mode: String = conn
            .query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))
            .db()?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(Error::Store(format!("journal_mode inesperado: {mode}")));
        }
        conn.pragma_update(None, "foreign_keys", true).db()?;
        conn.busy_timeout(std::time::Duration::from_secs(5)).db()?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    // --- contas ---
    pub fn account_list(&self) -> Result<Vec<Account>> {
        let conn = self.conn();
        let mut st = conn
            .prepare(&format!("SELECT {ACCOUNT_COLS} FROM account ORDER BY id"))
            .db()?;
        collect(st.query_map([], row_account).db()?)
    }

    pub fn account_get(&self, id: Id) -> Result<Account> {
        Self::account_get_in(&self.conn(), id)
    }

    fn account_get_in(conn: &Connection, id: Id) -> Result<Account> {
        conn.query_row(
            &format!("SELECT {ACCOUNT_COLS} FROM account WHERE id = ?1"),
            [id],
            row_account,
        )
        .optional()
        .db()?
        .ok_or_else(|| not_found("conta", id))
    }

    /// Senha salva (nunca vai ao frontend).
    pub fn account_password(&self, id: Id) -> Result<Option<Zeroizing<String>>> {
        let pw: Option<Option<String>> = self
            .conn()
            .query_row("SELECT password FROM account WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .db()?;
        match pw {
            None => Err(not_found("conta", id)),
            Some(p) => Ok(p.map(Zeroizing::new)),
        }
    }

    pub fn account_create(&self, new: &NewAccount) -> Result<Account> {
        let conn = self.conn();
        let now = ts(Utc::now());
        conn.execute(
            "INSERT INTO account (library_base_url, library_name, login, password, mode, \
             renew_days_before, remind_days_before, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                new.library_base_url,
                new.library_name,
                new.login,
                new.password,
                new.mode.as_str(),
                new.renew_days_before,
                new.remind_days_before,
                now
            ],
        )
        .db()?;
        let id = to_id(conn.last_insert_rowid())?;
        Self::account_get_in(&conn, id)
    }

    pub fn account_update(&self, id: Id, patch: &AccountPatch) -> Result<Account> {
        let mut conn = self.conn();
        let tx = conn.transaction().db()?;
        Self::account_get_in(&tx, id)?;
        tx.execute(
            "UPDATE account SET \
               library_base_url = COALESCE(?2, library_base_url), \
               library_name = COALESCE(?3, library_name), \
               login = COALESCE(?4, login), \
               password = CASE WHEN ?5 IS NOT NULL THEN ?5 WHEN ?6 THEN NULL ELSE password END, \
               mode = COALESCE(?7, mode), \
               status = COALESCE(?8, status), \
               renew_days_before = COALESCE(?9, renew_days_before), \
               remind_days_before = COALESCE(?10, remind_days_before), \
               updated_at = ?11 \
             WHERE id = ?1",
            params![
                id,
                patch.library_base_url,
                patch.library_name,
                patch.login,
                patch.password,
                patch.clear_password,
                patch.mode.map(|m| m.as_str()),
                patch.status.map(|s| s.as_str()),
                patch.renew_days_before,
                patch.remind_days_before,
                ts(Utc::now())
            ],
        )
        .db()?;
        let acc = Self::account_get_in(&tx, id)?;
        tx.commit().db()?;
        Ok(acc)
    }

    pub fn account_delete(&self, id: Id) -> Result<()> {
        let n = self
            .conn()
            .execute("DELETE FROM account WHERE id = ?1", [id])
            .db()?;
        if n == 0 {
            return Err(not_found("conta", id));
        }
        Ok(())
    }

    // --- livros e empréstimos ---
    pub fn book_upsert(&self, account_id: Id, item_key: &str, title: &str) -> Result<Book> {
        self.conn()
            .query_row(
                "INSERT INTO book (account_id, item_key, title, first_seen_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (account_id, item_key) DO UPDATE SET title = excluded.title \
                 RETURNING id, account_id, item_key, title, author, call_number, \
                   library_branch, first_seen_at",
                params![account_id, item_key, title, ts(Utc::now())],
                |r| row_book(r, 0),
            )
            .db()
    }

    pub fn loan_list(&self, filter: &LoanFilter) -> Result<Vec<LoanView>> {
        let conn = self.conn();
        let search = filter
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                let esc = s
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_");
                format!("%{esc}%")
            });
        let mut st = conn
            .prepare(&format!(
                "SELECT {LOAN_COLS}, {BOOK_COLS} FROM loan l JOIN book b ON b.id = l.book_id \
                 WHERE (?1 IS NULL OR b.account_id = ?1) \
                   AND (?2 IS NULL OR l.status = ?2) \
                   AND (?3 IS NULL OR b.title LIKE ?3 ESCAPE '\\' \
                        OR b.author LIKE ?3 ESCAPE '\\' OR b.call_number LIKE ?3 ESCAPE '\\') \
                 ORDER BY l.due_date, l.id"
            ))
            .db()?;
        collect(
            st.query_map(
                params![filter.account_id, filter.status.map(|s| s.as_str()), search],
                row_loan_view,
            )
            .db()?,
        )
    }

    fn loan_view_in(conn: &Connection, id: Id) -> Result<LoanView> {
        conn.query_row(
            &format!(
                "SELECT {LOAN_COLS}, {BOOK_COLS} FROM loan l JOIN book b ON b.id = l.book_id \
                 WHERE l.id = ?1"
            ),
            [id],
            row_loan_view,
        )
        .optional()
        .db()?
        .ok_or_else(|| not_found("empréstimo", id))
    }

    pub fn loan_get(&self, id: Id) -> Result<LoanDetail> {
        let conn = self.conn();
        let view = Self::loan_view_in(&conn, id)?;
        let mut st = conn
            .prepare(&format!(
                "SELECT {EVENT_COLS} FROM renewal_event e WHERE e.loan_id = ?1 \
                 ORDER BY e.attempted_at DESC, e.id DESC"
            ))
            .db()?;
        let events = collect(st.query_map([id], row_event).db()?)?;
        let mut st = conn
            .prepare(&format!(
                "SELECT {SCHEDULE_COLS} FROM schedule WHERE loan_id = ?1 ORDER BY run_at, id"
            ))
            .db()?;
        let schedules = collect(st.query_map([id], row_schedule).db()?)?;
        Ok(LoanDetail {
            view,
            events,
            schedules,
        })
    }

    /// Insere ou atualiza um empréstimo. Com `id > 0`, atualiza esse registro;
    /// senão, reaproveita o empréstimo em aberto do mesmo livro ou cria um novo.
    /// `updated_at` é sempre definido pelo banco.
    pub fn loan_upsert(&self, loan: &Loan) -> Result<Loan> {
        let mut conn = self.conn();
        let tx = conn.transaction().db()?;
        let target: Option<i64> = if loan.id > 0 {
            Some(loan.id.into())
        } else {
            tx.query_row(
                "SELECT id FROM loan WHERE book_id = ?1 AND status != 'returned' \
                 ORDER BY id DESC LIMIT 1",
                [loan.book_id],
                |r| r.get(0),
            )
            .optional()
            .db()?
        };
        let loaned_at = ts_opt(loan.loaned_at);
        let due = date(loan.due_date);
        let returned_at = ts_opt(loan.returned_at);
        let now = ts(Utc::now());
        let status = loan.status.as_str();
        let mut values: Vec<&dyn ToSql> = vec![
            &loan.book_id,
            &loaned_at,
            &due,
            &loan.renewals_count,
            &loan.renewals_limit,
            &status,
            &loan.auto_renew,
            &returned_at,
            &now,
        ];
        let id = match target {
            Some(ref id) => {
                values.push(id);
                let n = tx
                    .execute(
                        "UPDATE loan SET book_id = ?1, loaned_at = ?2, due_date = ?3, \
                         renewals_count = ?4, renewals_limit = ?5, status = ?6, auto_renew = ?7, \
                         returned_at = ?8, updated_at = ?9 WHERE id = ?10",
                        params_from_iter(values),
                    )
                    .db()?;
                if n == 0 {
                    return Err(not_found("empréstimo", loan.id));
                }
                *id
            }
            None => {
                tx.execute(
                    "INSERT INTO loan (book_id, loaned_at, due_date, renewals_count, \
                     renewals_limit, status, auto_renew, returned_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params_from_iter(values),
                )
                .db()?;
                tx.last_insert_rowid()
            }
        };
        let id = to_id(id)?;
        let out = tx
            .query_row(
                &format!("SELECT {LOAN_COLS} FROM loan l WHERE l.id = ?1"),
                [id],
                row_loan,
            )
            .db()?;
        tx.commit().db()?;
        Ok(out)
    }

    pub fn loan_set_auto_renew(&self, id: Id, on: bool) -> Result<()> {
        let n = self
            .conn()
            .execute(
                "UPDATE loan SET auto_renew = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, on, ts(Utc::now())],
            )
            .db()?;
        if n == 0 {
            return Err(not_found("empréstimo", id));
        }
        Ok(())
    }

    pub fn loan_mark_returned(&self, id: Id) -> Result<()> {
        let n = self
            .conn()
            .execute(
                "UPDATE loan SET status = 'returned', returned_at = COALESCE(returned_at, ?2), \
                 updated_at = ?2 WHERE id = ?1",
                params![id, ts(Utc::now())],
            )
            .db()?;
        if n == 0 {
            return Err(not_found("empréstimo", id));
        }
        Ok(())
    }

    // --- histórico ---
    pub fn event_insert(&self, event: &RenewalEvent) -> Result<RenewalEvent> {
        self.conn()
            .query_row(
                "INSERT INTO renewal_event (loan_id, attempted_at, trigger, result, \
                 old_due_date, new_due_date, message) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 RETURNING id, loan_id, attempted_at, trigger, result, old_due_date, \
                   new_due_date, message",
                params![
                    event.loan_id,
                    ts(event.attempted_at),
                    event.trigger.as_str(),
                    event.result.as_str(),
                    date_opt(event.old_due_date),
                    date_opt(event.new_due_date),
                    event.message
                ],
                row_event,
            )
            .db()
    }

    pub fn history_list(&self, filter: &HistoryFilter, page: Page) -> Result<HistoryPage> {
        let conn = self.conn();
        let filter_sql = "FROM renewal_event e JOIN loan l ON l.id = e.loan_id \
             JOIN book b ON b.id = l.book_id \
             WHERE (?1 IS NULL OR b.account_id = ?1) AND (?2 IS NULL OR e.loan_id = ?2) \
               AND (?3 IS NULL OR e.result = ?3) AND (?4 IS NULL OR e.attempted_at >= ?4) \
               AND (?5 IS NULL OR e.attempted_at <= ?5)";
        let from = ts_opt(filter.from);
        let to = ts_opt(filter.to);
        let result = filter.result.map(|r| r.as_str());
        let total: u32 = conn
            .query_row(
                &format!("SELECT count(*) {filter_sql}"),
                params![filter.account_id, filter.loan_id, result, from, to],
                |r| r.get(0),
            )
            .db()?;
        let mut st = conn
            .prepare(&format!(
                "SELECT {EVENT_COLS} {filter_sql} ORDER BY e.attempted_at DESC, e.id DESC \
                 LIMIT ?6 OFFSET ?7"
            ))
            .db()?;
        let items = collect(
            st.query_map(
                params![
                    filter.account_id,
                    filter.loan_id,
                    result,
                    from,
                    to,
                    page.limit,
                    page.offset
                ],
                row_event,
            )
            .db()?,
        )?;
        Ok(HistoryPage { items, total })
    }

    pub fn history_clear(&self, before: Option<chrono::DateTime<chrono::Utc>>) -> Result<u32> {
        let n = self
            .conn()
            .execute(
                "DELETE FROM renewal_event WHERE ?1 IS NULL OR attempted_at < ?1",
                [ts_opt(before)],
            )
            .db()?;
        u32::try_from(n).map_err(|_| Error::Store("contagem fora do intervalo".into()))
    }

    // --- agenda ---
    pub fn schedule_list(&self, filter: &ScheduleFilter) -> Result<Vec<Schedule>> {
        let conn = self.conn();
        let mut st = conn
            .prepare(&format!(
                "SELECT {SCHEDULE_COLS} FROM schedule \
                 WHERE (?1 IS NULL OR account_id = ?1) AND (?2 IS NULL OR kind = ?2) \
                   AND (?3 IS NULL OR status = ?3) ORDER BY run_at, id"
            ))
            .db()?;
        collect(
            st.query_map(
                params![
                    filter.account_id,
                    filter.kind.map(|k| k.as_str()),
                    filter.status.map(|s| s.as_str())
                ],
                row_schedule,
            )
            .db()?,
        )
    }

    pub fn schedule_insert(&self, s: &Schedule) -> Result<Schedule> {
        self.conn()
            .query_row(
                &format!(
                    "INSERT INTO schedule (account_id, loan_id, kind, run_at, status, origin, \
                     updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING {SCHEDULE_COLS}"
                ),
                params![
                    s.account_id,
                    s.loan_id,
                    s.kind.as_str(),
                    ts(s.run_at),
                    s.status.as_str(),
                    s.origin.as_str(),
                    ts(Utc::now())
                ],
                row_schedule,
            )
            .db()
    }

    /// `column` é sempre uma constante interna (nunca entrada do usuário).
    fn schedule_update(
        &self,
        id: Id,
        column: &'static str,
        value: &str,
        origin: ScheduleOrigin,
    ) -> Result<Schedule> {
        self.conn()
            .query_row(
                &format!(
                    "UPDATE schedule SET {column} = ?2, origin = ?3, updated_at = ?4 \
                     WHERE id = ?1 RETURNING {SCHEDULE_COLS}"
                ),
                params![id, value, origin.as_str(), ts(Utc::now())],
                row_schedule,
            )
            .optional()
            .db()?
            .ok_or_else(|| not_found("agendamento", id))
    }

    pub fn schedule_set_status(
        &self,
        id: Id,
        status: ScheduleStatus,
        origin: ScheduleOrigin,
    ) -> Result<Schedule> {
        self.schedule_update(id, "status", status.as_str(), origin)
    }

    /// Reagenda. Mudar o horário é ação do usuário (`origin = user`).
    pub fn schedule_reschedule(
        &self,
        id: Id,
        run_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Schedule> {
        self.schedule_update(id, "run_at", &ts(run_at), ScheduleOrigin::User)
    }

    // --- estatísticas, ajustes, diagnóstico ---

    /// Limites textuais `[from, to + 1 dia)` para comparar com timestamps ISO.
    fn range_bounds(range: DateRange) -> (String, String) {
        (date(range.from), date(range.to + Duration::days(1)))
    }

    pub fn stats_overview(&self, range: DateRange) -> Result<StatsOverview> {
        let conn = self.conn();
        let (from, to) = Self::range_bounds(range);
        let (total, successes, days): (u32, u32, f64) = conn
            .query_row(
                "SELECT count(*), \
                   coalesce(sum(result = 'success'), 0), \
                   coalesce(sum(CASE WHEN result = 'success' AND old_due_date IS NOT NULL \
                     AND new_due_date IS NOT NULL \
                     THEN max(julianday(new_due_date) - julianday(old_due_date), 0) END), 0.0) \
                 FROM renewal_event WHERE attempted_at >= ?1 AND attempted_at < ?2",
                params![from, to],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .db()?;
        let distinct_books: u32 = conn
            .query_row(
                "SELECT count(DISTINCT l.book_id) FROM loan l WHERE \
                   (l.due_date >= ?3 AND l.due_date <= ?4) \
                   OR (l.loaned_at >= ?1 AND l.loaned_at < ?2) \
                   OR EXISTS (SELECT 1 FROM renewal_event e WHERE e.loan_id = l.id \
                              AND e.attempted_at >= ?1 AND e.attempted_at < ?2)",
                params![from, to, date(range.from), date(range.to)],
                |r| r.get(0),
            )
            .db()?;
        let active_loans: u32 = conn
            .query_row(
                "SELECT count(*) FROM loan WHERE status IN ('active', 'overdue')",
                [],
                |r| r.get(0),
            )
            .db()?;
        let most_renewed = conn
            .query_row(
                &format!(
                    "SELECT {BOOK_COLS} FROM renewal_event e JOIN loan l ON l.id = e.loan_id \
                     JOIN book b ON b.id = l.book_id \
                     WHERE e.result = 'success' AND e.attempted_at >= ?1 AND e.attempted_at < ?2 \
                     GROUP BY b.id ORDER BY count(*) DESC, b.id LIMIT 1"
                ),
                params![from, to],
                |r| row_book(r, 0),
            )
            .optional()
            .db()?;
        Ok(StatsOverview {
            distinct_books,
            active_loans,
            renewals_total: total,
            success_rate: if total == 0 {
                0.0
            } else {
                f64::from(successes) / f64::from(total)
            },
            days_gained: days.round().max(0.0) as u32,
            most_renewed,
        })
    }

    pub fn stats_monthly(&self, range: DateRange) -> Result<Vec<MonthlyStat>> {
        let conn = self.conn();
        let (from, to) = Self::range_bounds(range);
        let mut st = conn
            .prepare(
                "SELECT substr(attempted_at, 1, 7) AS m, count(*), \
                   coalesce(sum(result = 'success'), 0) \
                 FROM renewal_event WHERE attempted_at >= ?1 AND attempted_at < ?2 \
                 GROUP BY m ORDER BY m",
            )
            .db()?;
        collect(
            st.query_map(params![from, to], |r| {
                let renewals: u32 = r.get(1)?;
                let successes: u32 = r.get(2)?;
                Ok(MonthlyStat {
                    month: r.get(0)?,
                    renewals,
                    successes,
                    failures: renewals - successes,
                })
            })
            .db()?,
        )
    }

    pub fn settings_get(&self) -> Result<Settings> {
        Self::settings_get_in(&self.conn())
    }

    fn settings_get_in(conn: &Connection) -> Result<Settings> {
        fn parse<T: serde::de::DeserializeOwned>(k: &str, v: &str) -> Result<T> {
            serde_json::from_str(v).map_err(|e| Error::Store(format!("ajuste {k}: {e}")))
        }
        let mut s = Settings::default();
        let mut st = conn.prepare("SELECT key, value FROM setting").db()?;
        let rows = collect(
            st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .db()?,
        )?;
        for (k, v) in rows {
            match k.as_str() {
                "theme" => s.theme = parse(&k, &v)?,
                "motion" => s.motion = parse(&k, &v)?,
                "notifications" => s.notifications = parse(&k, &v)?,
                "autostart" => s.autostart = parse(&k, &v)?,
                "app_lock" => s.app_lock = parse(&k, &v)?,
                "onboarding_done" => s.onboarding_done = parse(&k, &v)?,
                "privacy_accepted_at" => s.privacy_accepted_at = parse(&k, &v)?,
                _ => {} // chaves internas ou desconhecidas são ignoradas
            }
        }
        Ok(s)
    }

    pub fn settings_set(&self, patch: &SettingsPatch) -> Result<Settings> {
        fn json<T: serde::Serialize>(v: T) -> Result<String> {
            serde_json::to_string(&v).map_err(|e| Error::Store(e.to_string()))
        }
        let mut pairs: Vec<(&str, String)> = Vec::new();
        if let Some(v) = patch.theme {
            pairs.push(("theme", json(v)?));
        }
        if let Some(v) = patch.motion {
            pairs.push(("motion", json(v)?));
        }
        if let Some(v) = patch.notifications {
            pairs.push(("notifications", json(v)?));
        }
        if let Some(v) = patch.autostart {
            pairs.push(("autostart", json(v)?));
        }
        if let Some(v) = patch.app_lock {
            pairs.push(("app_lock", json(v)?));
        }
        if let Some(v) = patch.onboarding_done {
            pairs.push(("onboarding_done", json(v)?));
        }
        if let Some(v) = patch.privacy_accepted_at {
            pairs.push(("privacy_accepted_at", json(v)?));
        }
        let mut conn = self.conn();
        let tx = conn.transaction().db()?;
        for (k, v) in pairs {
            tx.execute(
                "INSERT INTO setting (key, value) VALUES (?1, ?2) \
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![k, v],
            )
            .db()?;
        }
        let s = Self::settings_get_in(&tx)?;
        tx.commit().db()?;
        Ok(s)
    }

    pub fn requests_recent(&self, limit: u32) -> Result<Vec<RequestLogEntry>> {
        let conn = self.conn();
        let mut st = conn
            .prepare(
                "SELECT id, account_id, host, at, kind, session_id, http_status, outcome \
                 FROM request_log ORDER BY at DESC, id DESC LIMIT ?1",
            )
            .db()?;
        collect(
            st.query_map([limit], |r| {
                Ok(RequestLogEntry {
                    id: r.get(0)?,
                    account_id: r.get(1)?,
                    host: r.get(2)?,
                    at: get_ts(r, 3)?,
                    kind: r.get(4)?,
                    session_id: r.get(5)?,
                    http_status: r.get(6)?,
                    outcome: r.get(7)?,
                })
            })
            .db()?,
        )
    }

    // --- dados ---

    /// Exporta uma cópia completa, criptografada com `passphrase` (SQLCipher).
    /// Sobrescreve `path` se já existir.
    pub fn backup_create(&self, path: &Path, passphrase: &str) -> Result<()> {
        if passphrase.is_empty() {
            return Err(Error::Invalid(
                "a senha do backup não pode ser vazia".into(),
            ));
        }
        if same_file(path, &self.path) {
            return Err(Error::Invalid(
                "o backup não pode sobrescrever o banco".into(),
            ));
        }
        remove_if_exists(path)?;
        let conn = self.conn();
        attach(&conn, path, passphrase)?;
        let res = conn
            .query_row("SELECT sqlcipher_export('bk')", [], |_| Ok(()))
            .db()
            .and_then(|()| {
                conn.pragma_update(Some("bk"), "user_version", MIGRATIONS.len() as i64)
                    .db()
            });
        let det = conn.execute_batch("DETACH DATABASE bk").db();
        res.and(det)
    }

    /// Substitui todo o conteúdo pelo do backup. Senha errada falha sem alterar nada.
    pub fn backup_restore(&self, path: &Path, passphrase: &str) -> Result<()> {
        if !path.is_file() {
            return Err(Error::NotFound(format!("backup {}", path.display())));
        }
        if same_file(path, &self.path) {
            return Err(Error::Invalid(
                "o backup não pode ser o próprio banco".into(),
            ));
        }
        let mut conn = self.conn();
        attach(&conn, path, passphrase)?;
        let res = restore_from_attached(&mut conn);
        let det = conn.execute_batch("DETACH DATABASE bk").db();
        res.and(det)
    }

    /// "Apagar tudo": fecha o banco e remove o arquivo (e os do WAL).
    pub fn wipe(self) -> Result<()> {
        let conn = self.conn.into_inner().unwrap_or_else(|p| p.into_inner());
        conn.close().map_err(|(_, e)| Error::Store(e.to_string()))?;
        for suffix in ["", "-wal", "-shm"] {
            let mut p = self.path.clone().into_os_string();
            p.push(suffix);
            remove_if_exists(Path::new(&p))?;
        }
        Ok(())
    }
}

fn restore_from_attached(conn: &mut Connection) -> Result<()> {
    conn.query_row("SELECT count(*) FROM bk.sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|_| Error::Invalid("senha do backup incorreta ou arquivo inválido".into()))?;
    let v: i64 = conn
        .query_row("PRAGMA bk.user_version", [], |r| r.get(0))
        .db()?;
    if v != MIGRATIONS.len() as i64 {
        return Err(Error::Invalid(format!(
            "versão do backup incompatível (schema {v})"
        )));
    }
    let tx = conn.transaction().db()?;
    tx.pragma_update(None, "defer_foreign_keys", true).db()?;
    for t in TABLES.iter().rev() {
        tx.execute(&format!("DELETE FROM main.{t}"), []).db()?;
    }
    for t in TABLES {
        tx.execute(&format!("INSERT INTO main.{t} SELECT * FROM bk.{t}"), [])
            .db()?;
    }
    tx.commit().db()
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

fn attach(conn: &Connection, path: &Path, passphrase: &str) -> Result<()> {
    let p = path
        .to_str()
        .ok_or_else(|| Error::Invalid("caminho inválido".into()))?;
    conn.execute("ATTACH DATABASE ?1 AS bk KEY ?2", params![p, passphrase])
        .db()
        .map(|_| ())
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::Store(format!("{}: {e}", path.display()))),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrations_are_embedded() {
        assert!(super::MIGRATIONS[0].contains("CREATE TABLE account"));
    }
}
