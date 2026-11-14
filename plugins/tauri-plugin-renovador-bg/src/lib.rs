//! Plugin de segundo plano (W1-11).
//! Comandos: `schedule_next(at)` e `get_db_key()`.

use tauri::{
    Runtime,
    plugin::{Builder, TauriPlugin},
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("renovador-bg").build()
}
