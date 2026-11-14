use serde::de::DeserializeOwned;
use tauri::{
    AppHandle, Runtime,
    plugin::{PluginApi, PluginHandle},
};

use crate::{DbKeyResponse, Result, ScheduleNextArgs};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_renovador_bg);

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> std::result::Result<RenovadorBg<R>, Box<dyn std::error::Error>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin(crate::PLUGIN_IDENTIFIER, "RenovadorBgPlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_renovador_bg)?;
    Ok(RenovadorBg(handle))
}

/// Acesso às APIs nativas do plugin.
pub struct RenovadorBg<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> RenovadorBg<R> {
    pub fn schedule_next(&self, at: String) -> Result<()> {
        self.0
            .run_mobile_plugin::<serde_json::Value>("scheduleNext", ScheduleNextArgs { at })
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn get_db_key(&self) -> Result<DbKeyResponse> {
        self.0.run_mobile_plugin("getDbKey", ()).map_err(Into::into)
    }
}
