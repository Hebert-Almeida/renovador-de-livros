//! Plugin de segundo plano (W1-11).
//!
//! Comandos: `schedule_next(at)` e `get_db_key()`. No Android a implementação
//! fica em Kotlin (`RenovadorBgPlugin`), no iOS em Swift; no desktop os comandos
//! respondem com [`Error::Unsupported`].

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Runtime, command,
    plugin::{Builder, TauriPlugin},
};

#[cfg(target_os = "android")]
mod jni_bridge;
#[cfg(mobile)]
mod mobile;

#[cfg(mobile)]
pub use mobile::RenovadorBg;

/// Acesso às APIs nativas do plugin a partir do app (`app.renovador_bg()`).
#[cfg(mobile)]
pub trait RenovadorBgExt<R: Runtime> {
    fn renovador_bg(&self) -> &RenovadorBg<R>;
}

#[cfg(mobile)]
impl<R: Runtime, T: tauri::Manager<R>> RenovadorBgExt<R> for T {
    fn renovador_bg(&self) -> &RenovadorBg<R> {
        self.state::<RenovadorBg<R>>().inner()
    }
}

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.renovador.bg";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("recurso disponível apenas no Android e no iOS")]
    Unsupported,
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Argumentos de `schedule_next`: instante planejado em RFC 3339.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleNextArgs {
    pub at: String,
}

/// Resposta de `get_db_key`: os 32 bytes da chave do banco em hexadecimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbKeyResponse {
    pub key_hex: String,
}

#[command]
async fn schedule_next<R: Runtime>(app: AppHandle<R>, at: String) -> Result<()> {
    #[cfg(mobile)]
    {
        use tauri::Manager;
        app.state::<mobile::RenovadorBg<R>>().schedule_next(at)
    }
    #[cfg(not(mobile))]
    {
        let _ = (app, at);
        Err(Error::Unsupported)
    }
}

#[command]
async fn get_db_key<R: Runtime>(app: AppHandle<R>) -> Result<DbKeyResponse> {
    #[cfg(mobile)]
    {
        use tauri::Manager;
        app.state::<mobile::RenovadorBg<R>>().get_db_key()
    }
    #[cfg(not(mobile))]
    {
        let _ = app;
        Err(Error::Unsupported)
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("renovador-bg")
        .invoke_handler(tauri::generate_handler![schedule_next, get_db_key])
        .setup(|_app, _api| {
            #[cfg(mobile)]
            {
                use tauri::Manager;
                _app.manage(mobile::init(_app, _api)?);
            }
            Ok(())
        })
        .build()
}
