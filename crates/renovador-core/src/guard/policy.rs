//! Política remota assinada (Ed25519) e merge "só aperta".

use base64::Engine as _;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, VerifyingKey};

use super::RemotePolicy;
use super::limits;

impl RemotePolicy {
    /// Verifica a assinatura de `policy.json` (`sig` em bytes crus de 64 bytes
    /// ou em base64) e só então interpreta o JSON. Assinatura inválida = erro.
    pub fn verify_and_parse(
        json: &[u8],
        sig: &[u8],
        public_key: &[u8; 32],
    ) -> crate::Result<RemotePolicy> {
        let key = VerifyingKey::from_bytes(public_key)
            .map_err(|e| crate::Error::Invalid(format!("chave pública inválida: {e}")))?;
        let raw = if sig.len() == 64 {
            sig.to_vec()
        } else {
            let text = std::str::from_utf8(sig)
                .map_err(|_| crate::Error::Invalid("assinatura ilegível".into()))?;
            base64::engine::general_purpose::STANDARD
                .decode(text.trim())
                .map_err(|_| crate::Error::Invalid("assinatura não é base64".into()))?
        };
        let sig = Signature::from_slice(&raw)
            .map_err(|_| crate::Error::Invalid("assinatura com tamanho inválido".into()))?;
        key.verify_strict(json, &sig)
            .map_err(|_| crate::Error::Invalid("assinatura da política inválida".into()))?;
        serde_json::from_slice(json)
            .map_err(|e| crate::Error::Invalid(format!("policy.json inválido: {e}")))
    }

    /// Combina duas políticas apertando sempre: mínimos, `E` lógico e união.
    pub fn merge(&self, other: &RemotePolicy) -> RemotePolicy {
        fn min_opt<T: Ord + Copy>(a: Option<T>, b: Option<T>) -> Option<T> {
            match (a, b) {
                (Some(x), Some(y)) => Some(x.min(y)),
                (x, None) | (None, x) => x,
            }
        }
        fn max_opt<T: Ord + Copy>(a: Option<T>, b: Option<T>) -> Option<T> {
            match (a, b) {
                (Some(x), Some(y)) => Some(x.max(y)),
                (x, None) | (None, x) => x,
            }
        }
        let mut blocked = self.blocked_hosts.clone();
        for h in &other.blocked_hosts {
            if !blocked.iter().any(|b| b.eq_ignore_ascii_case(h)) {
                blocked.push(h.clone());
            }
        }
        RemotePolicy {
            automation_enabled: match (self.automation_enabled, other.automation_enabled) {
                (Some(a), Some(b)) => Some(a && b),
                (x, None) | (None, x) => x,
            },
            max_auto_sessions_per_account_day: min_opt(
                self.max_auto_sessions_per_account_day,
                other.max_auto_sessions_per_account_day,
            ),
            max_manual_sessions_per_account_day: min_opt(
                self.max_manual_sessions_per_account_day,
                other.max_manual_sessions_per_account_day,
            ),
            max_sessions_per_install_day: min_opt(
                self.max_sessions_per_install_day,
                other.max_sessions_per_install_day,
            ),
            max_requests_per_session: min_opt(
                self.max_requests_per_session,
                other.max_requests_per_session,
            ),
            // Para intervalo, apertar significa aumentar.
            min_request_interval_secs: max_opt(
                self.min_request_interval_secs,
                other.min_request_interval_secs,
            ),
            blocked_hosts: blocked,
            message: other.message.clone().or_else(|| self.message.clone()),
        }
    }

    /// Limites efetivos: embutidos apertados pela política (nunca afrouxados).
    pub fn effective(&self) -> EffectiveLimits {
        let min = |builtin: u32, remote: Option<u32>| remote.map_or(builtin, |r| r.min(builtin));
        let interval = self
            .min_request_interval_secs
            .map_or(limits::MIN_REQUEST_INTERVAL, std::time::Duration::from_secs)
            .max(limits::MIN_REQUEST_INTERVAL);
        EffectiveLimits {
            automation_enabled: self.automation_enabled.unwrap_or(true),
            max_auto_sessions_per_account_day: min(
                limits::MAX_AUTO_SESSIONS_PER_ACCOUNT_DAY,
                self.max_auto_sessions_per_account_day,
            ),
            max_manual_sessions_per_account_day: min(
                limits::MAX_MANUAL_SESSIONS_PER_ACCOUNT_DAY,
                self.max_manual_sessions_per_account_day,
            ),
            max_sessions_per_install_day: min(
                limits::MAX_SESSIONS_PER_INSTALL_DAY,
                self.max_sessions_per_install_day,
            ),
            max_requests_per_session: min(
                limits::MAX_REQUESTS_PER_SESSION,
                self.max_requests_per_session,
            ),
            min_request_interval: interval,
            blocked_hosts: self
                .blocked_hosts
                .iter()
                .map(|h| h.to_ascii_lowercase())
                .collect(),
        }
    }
}

/// Limites em vigor após aplicar a política remota.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveLimits {
    pub automation_enabled: bool,
    pub max_auto_sessions_per_account_day: u32,
    pub max_manual_sessions_per_account_day: u32,
    pub max_sessions_per_install_day: u32,
    pub max_requests_per_session: u32,
    pub min_request_interval: std::time::Duration,
    pub blocked_hosts: Vec<String>,
}

/// A política remota pode ser consultada de novo? (no máximo 1×/dia)
pub fn policy_fetch_due(last_fetch: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match last_fetch {
        None => true,
        Some(t) => {
            now.signed_duration_since(t).to_std().unwrap_or_default()
                >= limits::REMOTE_POLICY_MIN_INTERVAL
        }
    }
}
