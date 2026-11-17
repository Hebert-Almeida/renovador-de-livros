//! Notificações (W2-03): envio pelo plugin de notificações. Os textos (seção
//! 5.4) ficam em `renovador_core::engine::notes`, compartilhados com o mobile.

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

pub use renovador_core::engine::notes::*;

use crate::state::AppState;

/// `true` se o usuário deixou as notificações ligadas (padrão: ligadas).
fn enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<AppState>()
        .with_store(|s| s.settings_get().ok())
        .flatten()
        .is_none_or(|s| s.notifications)
}

pub fn send<R: Runtime>(app: &AppHandle<R>, note: Note) {
    if !enabled(app) {
        return;
    }
    if let Err(e) = app
        .notification()
        .builder()
        .title(note.title)
        .body(note.body)
        .show()
    {
        tracing::warn!("falha ao notificar: {e}");
    }
}

pub fn init(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri_plugin_notification::PermissionState;
    let n = app.notification();
    if !matches!(n.permission_state(), Ok(PermissionState::Granted)) {
        let _ = n.request_permission();
    }
    Ok(())
}
