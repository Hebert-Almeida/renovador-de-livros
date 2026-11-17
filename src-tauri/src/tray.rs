//! Bandeja do sistema (W2-03): Abrir / Sincronizar agora / Pausar automação / Sair.
//!
//! Fechar a janela destrói o WebView (libera memória); o app continua na bandeja e
//! "Abrir" recria a janela a partir da configuração.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindowBuilder};

const MAIN: &str = "main";
/// Argumento passado pelo autostart (ver `lib.rs`).
pub const MINIMIZED_ARG: &str = "--minimized";

/// Mostra e foca a janela principal, recriando o WebView se ele foi liberado.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let Some(cfg) = app.config().app.windows.iter().find(|w| w.label == MAIN) else {
        return;
    };
    match WebviewWindowBuilder::from_config(app, cfg).and_then(|b| b.build()) {
        Ok(w) => {
            let _ = w.set_focus();
        }
        Err(e) => tracing::warn!("falha ao abrir a janela: {e}"),
    }
}

pub fn init(app: &mut tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "Sincronizar agora", true, None::<&str>)?;
    let pause =
        CheckMenuItem::with_id(app, "pause", "Pausar automação", true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &sync, &pause, &sep, &quit])?;

    let pause_item = pause.clone();
    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Renovador de Livros")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "sync" => crate::scheduler::sync_now(app),
            "pause" => {
                let paused = crate::scheduler::toggle_pause(app);
                let _ = pause_item.set_checked(paused);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    // Iniciado pelo sistema: fica só na bandeja, sem WebView.
    if std::env::args().any(|a| a == MINIMIZED_ARG)
        && let Some(w) = app.get_webview_window(MAIN)
    {
        let _ = w.destroy();
    }
    Ok(())
}

/// Trata eventos do runtime: manter o app vivo quando a última janela fecha
/// (o WebView é destruído normalmente; só o processo continua na bandeja).
pub fn on_run_event(_app: &AppHandle, event: &tauri::RunEvent) {
    if let tauri::RunEvent::ExitRequested {
        code: None, api, ..
    } = event
    {
        api.prevent_exit();
    }
}
