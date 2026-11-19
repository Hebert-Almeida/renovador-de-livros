//! Comandos IPC (seção 5.5). **Assinaturas congeladas na Onda 0.**
//! Corpos implementados na W2-02. `bindings.ts` é gerado a partir daqui.
//! A lógica fica em `state.rs` (sem Tauri, testável); aqui só se delega.

use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Utc};
use renovador_core::Result;
use renovador_core::model::*;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event as _;

use crate::events::{CycleFinishedEvent, CycleProgressEvent};
use crate::state::{AppConfig, AppState, TraceStatus};

/// Handle do app, para emitir eventos a partir dos comandos.
static APP: OnceLock<AppHandle> = OnceLock::new();

/// Abre o banco e liga o progresso dos ciclos ao evento `cycle://progress`.
/// Chamado no `setup` do app. Falhar ao abrir o banco não derruba o app: os
/// comandos devolvem erro de banco.
pub fn setup(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let _ = APP.set(handle.clone());
    let state = app.state::<AppState>();
    state.set_progress_sink(Arc::new(move |p| {
        let _ = CycleProgressEvent(p).emit(&handle);
    }));
    let data_dir = app.path().app_data_dir()?;
    let config = AppConfig::production(data_dir);
    // No mobile a chave vem do AndroidKeyStore pelo plugin, a mesma do RenewWorker.
    #[cfg(mobile)]
    let config = config.map(|mut c| {
        c.keystore = Arc::new(crate::mobile::PluginKeyStore::new(app.handle().clone()));
        c
    });
    if let Err(e) = config.and_then(|c| state.init(c)) {
        tracing::error!("não foi possível abrir o banco: {e}");
    }
    Ok(())
}

// --- Geral ---

#[tauri::command]
#[specta::specta]
pub async fn app_status(state: State<'_, AppState>) -> Result<AppStatus> {
    state.app_status()
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
    state.library_validate(&url).await
}

// --- Contas ---

#[tauri::command]
#[specta::specta]
pub async fn account_list(state: State<'_, AppState>) -> Result<Vec<Account>> {
    state.account_list()
}

#[tauri::command]
#[specta::specta]
pub async fn account_create(state: State<'_, AppState>, account: NewAccount) -> Result<Account> {
    state.account_create(account)
}

#[tauri::command]
#[specta::specta]
pub async fn account_update(
    state: State<'_, AppState>,
    id: Id,
    patch: AccountPatch,
) -> Result<Account> {
    state.account_update(id, patch)
}

#[tauri::command]
#[specta::specta]
pub async fn account_delete(state: State<'_, AppState>, id: Id) -> Result<()> {
    state.account_delete(id)
}

/// Testa o login (conta na cota).
#[tauri::command]
#[specta::specta]
pub async fn account_test_login(state: State<'_, AppState>, id: Id) -> Result<bool> {
    state.account_test_login(id).await
}

// --- Empréstimos ---

#[tauri::command]
#[specta::specta]
pub async fn loan_list(state: State<'_, AppState>, filter: LoanFilter) -> Result<Vec<LoanView>> {
    state.loan_list(&filter)
}

#[tauri::command]
#[specta::specta]
pub async fn loan_get(state: State<'_, AppState>, id: Id) -> Result<LoanDetail> {
    state.loan_get(id)
}

#[tauri::command]
#[specta::specta]
pub async fn loan_set_auto_renew(state: State<'_, AppState>, id: Id, on: bool) -> Result<()> {
    state.loan_set_auto_renew(id, on)
}

#[tauri::command]
#[specta::specta]
pub async fn loan_mark_returned(state: State<'_, AppState>, id: Id) -> Result<()> {
    state.loan_mark_returned(id)
}

// --- Execução e cota ---

#[tauri::command]
#[specta::specta]
pub async fn cycle_run_now(
    state: State<'_, AppState>,
    account_id: Id,
    kind: CycleKind,
) -> Result<CycleReport> {
    let report = state.cycle_run_now(account_id, kind).await?;
    if let Some(app) = APP.get() {
        let _ = CycleFinishedEvent(report.clone()).emit(app);
    }
    Ok(report)
}

/// "Renovar agora" de um empréstimo (Onda 3): renova só esse livro, mesmo fora
/// da janela de vencimento, com o mesmo gate e cota de um ciclo manual.
#[tauri::command]
#[specta::specta]
pub async fn loan_renew(state: State<'_, AppState>, loan_id: Id) -> Result<CycleReport> {
    let report = state.loan_renew(loan_id).await?;
    if let Some(app) = APP.get() {
        let _ = CycleFinishedEvent(report.clone()).emit(app);
    }
    Ok(report)
}

#[tauri::command]
#[specta::specta]
pub async fn quota_status(state: State<'_, AppState>, account_id: Id) -> Result<QuotaStatus> {
    state.quota_status(account_id)
}

// --- Agenda ---

#[tauri::command]
#[specta::specta]
pub async fn schedule_list(
    state: State<'_, AppState>,
    filter: ScheduleFilter,
) -> Result<Vec<Schedule>> {
    state.schedule_list(&filter)
}

#[tauri::command]
#[specta::specta]
pub async fn schedule_cancel(state: State<'_, AppState>, id: Id) -> Result<Schedule> {
    state.schedule_cancel(id)
}

#[tauri::command]
#[specta::specta]
pub async fn schedule_pause(state: State<'_, AppState>, id: Id) -> Result<Schedule> {
    state.schedule_pause(id)
}

#[tauri::command]
#[specta::specta]
pub async fn schedule_reactivate(state: State<'_, AppState>, id: Id) -> Result<Schedule> {
    state.schedule_reactivate(id)
}

/// Reagenda dentro da janela permitida.
#[tauri::command]
#[specta::specta]
pub async fn schedule_reschedule(
    state: State<'_, AppState>,
    id: Id,
    run_at: DateTime<Utc>,
) -> Result<Schedule> {
    state.schedule_reschedule(id, run_at)
}

// --- Histórico ---

#[tauri::command]
#[specta::specta]
pub async fn history_list(
    state: State<'_, AppState>,
    filter: HistoryFilter,
    page: Page,
) -> Result<HistoryPage> {
    state.history_list(&filter, page)
}

/// `path` vem do diálogo de arquivo (`@tauri-apps/plugin-dialog`) no frontend.
#[tauri::command]
#[specta::specta]
pub async fn history_export(
    state: State<'_, AppState>,
    format: ExportFormat,
    path: String,
) -> Result<u32> {
    state.history_export(format, &path)
}

#[tauri::command]
#[specta::specta]
pub async fn history_clear(
    state: State<'_, AppState>,
    before: Option<DateTime<Utc>>,
) -> Result<u32> {
    state.history_clear(before)
}

// --- Estatísticas ---

#[tauri::command]
#[specta::specta]
pub async fn stats_overview(state: State<'_, AppState>, range: DateRange) -> Result<StatsOverview> {
    state.stats_overview(range)
}

#[tauri::command]
#[specta::specta]
pub async fn stats_monthly(
    state: State<'_, AppState>,
    range: DateRange,
) -> Result<Vec<MonthlyStat>> {
    state.stats_monthly(range)
}

// --- Ajustes e dados ---

#[tauri::command]
#[specta::specta]
pub async fn settings_get(state: State<'_, AppState>) -> Result<Settings> {
    state.settings_get()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set(state: State<'_, AppState>, patch: SettingsPatch) -> Result<Settings> {
    state.settings_set(&patch)
}

#[tauri::command]
#[specta::specta]
pub async fn backup_create(
    state: State<'_, AppState>,
    path: String,
    passphrase: String,
) -> Result<()> {
    state.backup_create(&path, &passphrase)
}

#[tauri::command]
#[specta::specta]
pub async fn backup_restore(
    state: State<'_, AppState>,
    path: String,
    passphrase: String,
) -> Result<()> {
    state.backup_restore(&path, &passphrase)
}

#[tauri::command]
#[specta::specta]
pub async fn data_wipe(state: State<'_, AppState>) -> Result<()> {
    state.data_wipe()
}

// --- Diagnóstico ---

#[tauri::command]
#[specta::specta]
pub async fn diagnostics_requests(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<RequestLogEntry>> {
    state.diagnostics_requests(limit)
}

/// Estado do registro detalhado (páginas gravadas para diagnóstico).
#[tauri::command]
#[specta::specta]
pub async fn diagnostics_trace_status(state: State<'_, AppState>) -> Result<TraceStatus> {
    state.diagnostics_trace_status()
}

/// Liga ou desliga o registro detalhado; desligar apaga as páginas.
#[tauri::command]
#[specta::specta]
pub async fn diagnostics_trace_set(state: State<'_, AppState>, on: bool) -> Result<TraceStatus> {
    state.diagnostics_trace_set(on)
}

/// Abre a pasta do registro detalhado no explorador de arquivos.
#[tauri::command]
#[specta::specta]
pub async fn diagnostics_trace_open(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    use tauri_plugin_opener::OpenerExt as _;
    let dir = state.diagnostics_trace_dir()?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| renovador_core::Error::Invalid(format!("não foi possível abrir a pasta: {e}")))
}
