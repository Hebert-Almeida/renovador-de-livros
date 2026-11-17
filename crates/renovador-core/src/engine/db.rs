//! Acesso direto ao banco usado só pelo motor: campos de conta que o
//! `AccountPatch` não cobre, chaves internas de `setting` e contagem de
//! requisições da sessão. Não muda o esquema (`0001_init.sql`).

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, params};

use crate::guard::RemotePolicy;
use crate::model::Id;
use crate::store::Store;
use crate::{Error, Result};

/// Chaves internas em `setting` (o `settings_get` ignora chaves desconhecidas).
const KEY_INSTALL_SEED: &str = "engine.install_seed";
const KEY_POLICY: &str = "engine.remote_policy";
const KEY_POLICY_FETCHED_AT: &str = "engine.remote_policy_fetched_at";

fn ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Micros, true)
}

fn db<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|e| Error::Store(e.to_string()))
}

fn setting_get(store: &Store, key: &str) -> Result<Option<String>> {
    db(store
        .conn()
        .query_row("SELECT value FROM setting WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional())
}

fn setting_put(store: &Store, key: &str, value: &str) -> Result<()> {
    db(store.conn().execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    ))
    .map(|_| ())
}

/// Semente fixa da instalação (minuto aleatório da janela). Criada na 1ª vez.
pub fn install_seed(store: &Store) -> Result<u64> {
    if let Some(v) = setting_get(store, KEY_INSTALL_SEED)?
        && let Ok(seed) = v.parse::<u64>()
    {
        return Ok(seed);
    }
    let seed: u64 = rand::random();
    setting_put(store, KEY_INSTALL_SEED, &seed.to_string())?;
    Ok(seed)
}

/// Política remota em cache e o instante da última consulta.
pub fn cached_policy(store: &Store) -> Result<(RemotePolicy, Option<DateTime<Utc>>)> {
    let policy = setting_get(store, KEY_POLICY)?
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();
    let at = setting_get(store, KEY_POLICY_FETCHED_AT)?
        .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
        .map(|t| t.with_timezone(&Utc));
    Ok((policy, at))
}

pub fn save_policy(store: &Store, policy: &RemotePolicy, at: DateTime<Utc>) -> Result<()> {
    let json = serde_json::to_string(policy).map_err(|e| Error::Store(e.to_string()))?;
    setting_put(store, KEY_POLICY, &json)?;
    setting_put(store, KEY_POLICY_FETCHED_AT, &ts(at))
}

pub fn set_last_sync(store: &Store, account_id: Id, at: DateTime<Utc>) -> Result<()> {
    db(store.conn().execute(
        "UPDATE account SET last_sync_at = ?2 WHERE id = ?1",
        params![account_id, ts(at)],
    ))
    .map(|_| ())
}

pub fn set_last_renewal(store: &Store, account_id: Id, at: DateTime<Utc>) -> Result<()> {
    db(store.conn().execute(
        "UPDATE account SET last_renewal_at = ?2 WHERE id = ?1",
        params![account_id, ts(at)],
    ))
    .map(|_| ())
}

pub fn set_next_renewal(store: &Store, account_id: Id, at: Option<DateTime<Utc>>) -> Result<()> {
    db(store.conn().execute(
        "UPDATE account SET next_renewal_at = ?2 WHERE id = ?1",
        params![account_id, at.map(ts)],
    ))
    .map(|_| ())
}

/// Requisições HTTP registradas pelo gate nesta sessão (sem o marcador de início).
pub fn requests_in_session(store: &Store, session_id: &str) -> Result<u32> {
    let n: i64 = db(store.conn().query_row(
        "SELECT count(*) FROM request_log WHERE session_id = ?1 \
         AND kind NOT LIKE 'session_start:%'",
        [session_id],
        |r| r.get(0),
    ))?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Preferências de planejamento dos Ajustes; usa o padrão se a leitura falhar.
pub fn plan_prefs(store: &Store) -> crate::planner::Prefs {
    store
        .settings_get()
        .map(|s| crate::planner::Prefs::from(&s))
        .unwrap_or_default()
}
