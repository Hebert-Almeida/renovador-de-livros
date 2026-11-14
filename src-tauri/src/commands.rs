#![allow(unused_variables)]
//! Comandos IPC (seção 5.5). **Assinaturas congeladas na Onda 0.**
//! Corpos implementados na W2-02. `bindings.ts` é gerado a partir daqui.

use chrono::{DateTime, Utc};
use renovador_core::model::*;
use renovador_core::{Error, Result};
use tauri::State;

use crate::state::AppState;

fn todo<T>(what: &str) -> Result<T> {
    Err(Error::todo(what))
}

// --- Geral ---

#[tauri::command]
#[specta::specta]
pub async fn app_status(state: State<'_, AppState>) -> Result<AppStatus> {
    todo("app_status")
}

#[tauri::command]
#[specta::specta]
pub fn library_presets() -> Vec<LibraryPreset> {
    renovador_core::model::library_presets()
}

#[tauri::command]
#[specta::specta]
pub async fn library_validate(
    state: State<'_, AppState>,
    url: String,
) -> Result<LibraryValidation> {
    todo("library_validate")
}

// --- Contas ---

#[tauri::command]
#[specta::specta]
pub async fn account_list(state: State<'_, AppState>) -> Result<Vec<Account>> {
    todo("account_list")
}

#[tauri::command]
#[specta::specta]
pub async fn account_create(state: State<'_, AppState>, account: NewAccount) -> Result<Account> {
    todo("account_create")
}

#[tauri::command]
#[specta::specta]
pub async fn account_update(
    state: State<'_, AppState>,
    id: Id,
    patch: AccountPatch,
) -> Result<Account> {
    todo("account_update")
}

#[tauri::command]
#[specta::specta]
pub async fn account_delete(state: State<'_, AppState>, id: Id) -> Result<()> {
    todo("account_delete")
}

/// Testa o login (conta na cota).
#[tauri::command]
#[specta::specta]
pub async fn account_test_login(state: State<'_, AppState>, id: Id) -> Result<bool> {
    todo("account_test_login")
}

// --- Empréstimos ---

#[tauri::command]
#[specta::specta]
pub async fn loan_list(state: State<'_, AppState>, filter: LoanFilter) -> Result<Vec<LoanView>> {
    todo("loan_list")
}

#[tauri::command]
#[specta::specta]
pub async fn loan_get(state: State<'_, AppState>, id: Id) -> Result<LoanDetail> {
    todo("loan_get")
}

#[tauri::command]
#[specta::specta]
pub async fn loan_set_auto_renew(state: State<'_, AppState>, id: Id, on: bool) -> Result<()> {
    todo("loan_set_auto_renew")
}

#[tauri::command]
#[specta::specta]
pub async fn loan_mark_returned(state: State<'_, AppState>, id: Id) -> Result<()> {
    todo("loan_mark_returned")
}

// --- Execução e cota ---

#[tauri::command]
#[specta::specta]
pub async fn cycle_run_now(
    state: State<'_, AppState>,
    account_id: Id,
    kind: CycleKind,
) -> Result<CycleReport> {
    todo("cycle_run_now")
}

#[tauri::command]
#[specta::specta]
pub async fn quota_status(state: State<'_, AppState>, account_id: Id) -> Result<QuotaStatus> {
    todo("quota_status")
}

// --- Agenda ---

#[tauri::command]
#[specta::specta]
pub async fn schedule_list(
    state: State<'_, AppState>,
    filter: ScheduleFilter,
) -> Result<Vec<Schedule>> {
    todo("schedule_list")
}

#[tauri::command]
#[specta::specta]
pub async fn schedule_cancel(state: State<'_, AppState>, id: Id) -> Result<Schedule> {
    todo("schedule_cancel")
}

#[tauri::command]
#[specta::specta]
pub async fn schedule_pause(state: State<'_, AppState>, id: Id) -> Result<Schedule> {
    todo("schedule_pause")
}

#[tauri::command]
#[specta::specta]
pub async fn schedule_reactivate(state: State<'_, AppState>, id: Id) -> Result<Schedule> {
    todo("schedule_reactivate")
}

/// Reagenda dentro da janela permitida.
#[tauri::command]
#[specta::specta]
pub async fn schedule_reschedule(
    state: State<'_, AppState>,
    id: Id,
    run_at: DateTime<Utc>,
) -> Result<Schedule> {
    todo("schedule_reschedule")
}

// --- Histórico ---

#[tauri::command]
#[specta::specta]
pub async fn history_list(
    state: State<'_, AppState>,
    filter: HistoryFilter,
    page: Page,
) -> Result<HistoryPage> {
    todo("history_list")
}

#[tauri::command]
#[specta::specta]
pub async fn history_export(
    state: State<'_, AppState>,
    format: ExportFormat,
    path: String,
) -> Result<u32> {
    todo("history_export")
}

#[tauri::command]
#[specta::specta]
pub async fn history_clear(
    state: State<'_, AppState>,
    before: Option<DateTime<Utc>>,
) -> Result<u32> {
    todo("history_clear")
}

// --- Estatísticas ---

#[tauri::command]
#[specta::specta]
pub async fn stats_overview(state: State<'_, AppState>, range: DateRange) -> Result<StatsOverview> {
    todo("stats_overview")
}

#[tauri::command]
#[specta::specta]
pub async fn stats_monthly(
    state: State<'_, AppState>,
    range: DateRange,
) -> Result<Vec<MonthlyStat>> {
    todo("stats_monthly")
}

// --- Ajustes e dados ---

#[tauri::command]
#[specta::specta]
pub async fn settings_get(state: State<'_, AppState>) -> Result<Settings> {
    todo("settings_get")
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set(state: State<'_, AppState>, patch: SettingsPatch) -> Result<Settings> {
    todo("settings_set")
}

#[tauri::command]
#[specta::specta]
pub async fn backup_create(
    state: State<'_, AppState>,
    path: String,
    passphrase: String,
) -> Result<()> {
    todo("backup_create")
}

#[tauri::command]
#[specta::specta]
pub async fn backup_restore(
    state: State<'_, AppState>,
    path: String,
    passphrase: String,
) -> Result<()> {
    todo("backup_restore")
}

#[tauri::command]
#[specta::specta]
pub async fn data_wipe(state: State<'_, AppState>) -> Result<()> {
    todo("data_wipe")
}

// --- Diagnóstico ---

#[tauri::command]
#[specta::specta]
pub async fn diagnostics_requests(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<RequestLogEntry>> {
    todo("diagnostics_requests")
}
