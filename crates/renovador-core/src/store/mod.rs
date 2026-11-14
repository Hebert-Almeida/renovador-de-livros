//! Banco SQLCipher. API congelada na Onda 0; implementação na W1-02.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use zeroize::Zeroizing;

use crate::model::*;
use crate::{Error, Result};

/// Migrações aplicadas em ordem via `PRAGMA user_version`.
pub const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_init.sql")];

/// Chave do banco: 32 bytes aleatórios, zerada ao sair de escopo.
pub type DbKey = Zeroizing<[u8; 32]>;

#[allow(dead_code)] // campos usados a partir da W1-02
pub struct Store {
    pub(crate) conn: Connection,
    pub(crate) path: PathBuf,
}

impl Store {
    /// Abre (ou cria) o banco criptografado e roda as migrações.
    pub fn open(_path: &Path, _key: &DbKey) -> Result<Self> {
        Err(Error::todo("store::open"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    // --- contas ---
    pub fn account_list(&self) -> Result<Vec<Account>> {
        Err(Error::todo("store::account_list"))
    }
    pub fn account_get(&self, _id: Id) -> Result<Account> {
        Err(Error::todo("store::account_get"))
    }
    /// Senha salva (nunca vai ao frontend).
    pub fn account_password(&self, _id: Id) -> Result<Option<Zeroizing<String>>> {
        Err(Error::todo("store::account_password"))
    }
    pub fn account_create(&self, _new: &NewAccount) -> Result<Account> {
        Err(Error::todo("store::account_create"))
    }
    pub fn account_update(&self, _id: Id, _patch: &AccountPatch) -> Result<Account> {
        Err(Error::todo("store::account_update"))
    }
    pub fn account_delete(&self, _id: Id) -> Result<()> {
        Err(Error::todo("store::account_delete"))
    }

    // --- livros e empréstimos ---
    pub fn book_upsert(&self, _account_id: Id, _item_key: &str, _title: &str) -> Result<Book> {
        Err(Error::todo("store::book_upsert"))
    }
    pub fn loan_list(&self, _filter: &LoanFilter) -> Result<Vec<LoanView>> {
        Err(Error::todo("store::loan_list"))
    }
    pub fn loan_get(&self, _id: Id) -> Result<LoanDetail> {
        Err(Error::todo("store::loan_get"))
    }
    pub fn loan_upsert(&self, _loan: &Loan) -> Result<Loan> {
        Err(Error::todo("store::loan_upsert"))
    }
    pub fn loan_set_auto_renew(&self, _id: Id, _on: bool) -> Result<()> {
        Err(Error::todo("store::loan_set_auto_renew"))
    }
    pub fn loan_mark_returned(&self, _id: Id) -> Result<()> {
        Err(Error::todo("store::loan_mark_returned"))
    }

    // --- histórico ---
    pub fn event_insert(&self, _event: &RenewalEvent) -> Result<RenewalEvent> {
        Err(Error::todo("store::event_insert"))
    }
    pub fn history_list(&self, _filter: &HistoryFilter, _page: Page) -> Result<HistoryPage> {
        Err(Error::todo("store::history_list"))
    }
    pub fn history_clear(&self, _before: Option<chrono::DateTime<chrono::Utc>>) -> Result<u32> {
        Err(Error::todo("store::history_clear"))
    }

    // --- agenda ---
    pub fn schedule_list(&self, _filter: &ScheduleFilter) -> Result<Vec<Schedule>> {
        Err(Error::todo("store::schedule_list"))
    }
    pub fn schedule_insert(&self, _s: &Schedule) -> Result<Schedule> {
        Err(Error::todo("store::schedule_insert"))
    }
    pub fn schedule_set_status(
        &self,
        _id: Id,
        _status: ScheduleStatus,
        _origin: ScheduleOrigin,
    ) -> Result<Schedule> {
        Err(Error::todo("store::schedule_set_status"))
    }
    pub fn schedule_reschedule(
        &self,
        _id: Id,
        _run_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Schedule> {
        Err(Error::todo("store::schedule_reschedule"))
    }

    // --- estatísticas, ajustes, diagnóstico ---
    pub fn stats_overview(&self, _range: DateRange) -> Result<StatsOverview> {
        Err(Error::todo("store::stats_overview"))
    }
    pub fn stats_monthly(&self, _range: DateRange) -> Result<Vec<MonthlyStat>> {
        Err(Error::todo("store::stats_monthly"))
    }
    pub fn settings_get(&self) -> Result<Settings> {
        Err(Error::todo("store::settings_get"))
    }
    pub fn settings_set(&self, _patch: &SettingsPatch) -> Result<Settings> {
        Err(Error::todo("store::settings_set"))
    }
    pub fn requests_recent(&self, _limit: u32) -> Result<Vec<RequestLogEntry>> {
        Err(Error::todo("store::requests_recent"))
    }

    // --- dados ---
    pub fn backup_create(&self, _path: &Path, _passphrase: &str) -> Result<()> {
        Err(Error::todo("store::backup_create"))
    }
    pub fn backup_restore(&self, _path: &Path, _passphrase: &str) -> Result<()> {
        Err(Error::todo("store::backup_restore"))
    }
    pub fn wipe(self) -> Result<()> {
        Err(Error::todo("store::wipe"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrations_are_embedded() {
        assert!(super::MIGRATIONS[0].contains("CREATE TABLE account"));
    }
}
