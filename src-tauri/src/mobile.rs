//! Integração com o plugin de segundo plano no mobile (Onda 3).
//!
//! - A chave do banco vem do AndroidKeyStore pelo plugin (`get_db_key`): é a
//!   mesma que o `RenewWorker` usa, então o app e o WorkManager abrem o mesmo
//!   banco.
//! - Com o app aberto, a próxima execução única do WorkManager acompanha o
//!   primeiro agendamento pendente da agenda (`schedule_next`). O periódico de
//!   6 h, criado pelo plugin, cobre o resto.
//!
//! O módulo compila em todas as plataformas para os testes de `decode_key`
//! rodarem no CI; o que depende do plugin só existe no mobile.

#[cfg(mobile)]
pub use imp::{PluginKeyStore, start};

use renovador_core::store::DbKey;
use renovador_core::{Error, Result};

/// Decodifica os 32 bytes da chave em hexadecimal (64 caracteres).
#[cfg_attr(not(mobile), allow(dead_code))]
fn decode_key(hex: &str) -> Result<DbKey> {
    let bad = || Error::Keys("chave do cofre do Android em formato inválido".into());
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return Err(bad());
    }
    let mut key = DbKey::new([0u8; 32]);
    for (i, pair) in bytes.as_chunks::<2>().0.iter().enumerate() {
        let hi = (pair[0] as char).to_digit(16).ok_or_else(bad)?;
        let lo = (pair[1] as char).to_digit(16).ok_or_else(bad)?;
        key[i] = (hi * 16 + lo) as u8;
    }
    Ok(key)
}

#[cfg(mobile)]
mod imp {
    use std::time::Duration;

    use chrono::{DateTime, Utc};
    use renovador_core::keys::KeyStore;
    use renovador_core::model::{ScheduleFilter, ScheduleStatus};
    use renovador_core::store::DbKey;
    use renovador_core::{Error, Result};
    use tauri::{AppHandle, Manager};
    use tauri_plugin_renovador_bg::RenovadorBgExt;

    use super::decode_key;
    use crate::state::AppState;

    /// Frequência com que a agenda é conferida enquanto o app está aberto (só leitura local).
    const POLL: Duration = Duration::from_secs(60);

    /// `KeyStore` que lê a chave do AndroidKeyStore pelo plugin. O lado Kotlin
    /// (`KeyVault.getOrCreateDbKey`) cria a chave na primeira vez e a guarda
    /// envolvida por AES-GCM, então `store` e `delete` não têm o que fazer aqui.
    pub struct PluginKeyStore {
        app: AppHandle,
    }

    impl PluginKeyStore {
        pub fn new(app: AppHandle) -> Self {
            Self { app }
        }
    }

    impl KeyStore for PluginKeyStore {
        fn load(&self) -> Result<Option<DbKey>> {
            let resp = self
                .app
                .renovador_bg()
                .get_db_key()
                .map_err(|e| Error::Keys(format!("cofre do Android: {e}")))?;
            decode_key(&resp.key_hex).map(Some)
        }
        fn store(&self, _key: &DbKey) -> Result<()> {
            Ok(())
        }
        fn delete(&self) -> Result<()> {
            Ok(())
        }
    }

    /// Mantém a execução única do WorkManager no horário do próximo agendamento pendente.
    pub fn start(app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            let mut last: Option<DateTime<Utc>> = None;
            loop {
                let next = next_run(&app);
                if next.is_some() && next != last {
                    let at = next.expect("checado acima").to_rfc3339();
                    match app.renovador_bg().schedule_next(at) {
                        Ok(()) => last = next,
                        Err(e) => tracing::warn!("não foi possível agendar o segundo plano: {e}"),
                    }
                }
                tokio::time::sleep(POLL).await;
            }
        });
    }

    fn next_run(app: &AppHandle) -> Option<DateTime<Utc>> {
        let state = app.try_state::<AppState>()?;
        state
            .with_store(|store| {
                let filter = ScheduleFilter {
                    status: Some(ScheduleStatus::Pending),
                    ..Default::default()
                };
                store
                    .schedule_list(&filter)
                    .ok()?
                    .into_iter()
                    .map(|s| s.run_at)
                    .min()
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::decode_key;

    #[test]
    fn decodifica_hex_de_32_bytes() {
        let hex = "00ff10".to_string() + &"ab".repeat(29);
        let k = decode_key(&hex).unwrap();
        assert_eq!(k[0], 0x00);
        assert_eq!(k[1], 0xff);
        assert_eq!(k[2], 0x10);
        assert_eq!(k[31], 0xab);
    }

    #[test]
    fn rejeita_tamanho_ou_digito_invalido() {
        assert!(decode_key("abcd").is_err());
        assert!(decode_key(&"zz".repeat(32)).is_err());
    }
}
