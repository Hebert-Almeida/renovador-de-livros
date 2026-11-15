mod commands;
mod events;
mod notify;
mod scheduler;
mod state;
#[cfg(desktop)]
mod tray;

use tauri_specta::{Builder, collect_commands, collect_events};

/// Builder do tauri-specta com todos os comandos e eventos (contrato congelado).
pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_status,
            commands::library_presets,
            commands::library_validate,
            commands::account_list,
            commands::account_create,
            commands::account_update,
            commands::account_delete,
            commands::account_test_login,
            commands::loan_list,
            commands::loan_get,
            commands::loan_set_auto_renew,
            commands::loan_mark_returned,
            commands::cycle_run_now,
            commands::quota_status,
            commands::schedule_list,
            commands::schedule_cancel,
            commands::schedule_pause,
            commands::schedule_reactivate,
            commands::schedule_reschedule,
            commands::history_list,
            commands::history_export,
            commands::history_clear,
            commands::stats_overview,
            commands::stats_monthly,
            commands::settings_get,
            commands::settings_set,
            commands::backup_create,
            commands::backup_restore,
            commands::data_wipe,
            commands::diagnostics_requests,
        ])
        .events(collect_events![
            events::CycleProgressEvent,
            events::CycleFinishedEvent,
            events::AutomationStateEvent,
        ])
}

pub const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/ipc/bindings.ts");

/// Gera `src/lib/ipc/bindings.ts`.
pub fn export_bindings() {
    specta_builder()
        .export(
            specta_typescript::Typescript::default()
                .header("// @ts-nocheck\n// Gerado pelo tauri-specta. NÃO EDITAR.\n"),
            BINDINGS_PATH,
        )
        .expect("falha ao exportar bindings");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta = specta_builder();

    #[cfg(debug_assertions)]
    export_bindings();

    let mut app = tauri::Builder::default();

    #[cfg(desktop)]
    {
        app = app
            .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
                tray::show_main(app);
            }))
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                Some(vec!["--minimized"]),
            ));
    }

    #[cfg(mobile)]
    {
        app = app.plugin(tauri_plugin_renovador_bg::init());
    }

    app.plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(state::AppState::default())
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            #[cfg(desktop)]
            tray::init(app)?;
            notify::init(app)?;
            scheduler::start(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Renovador de Livros")
        .run(|_app, _event| {
            #[cfg(desktop)]
            tray::on_run_event(_app, &_event);
        });
}
