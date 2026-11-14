//! Persistência em memória do gate (testes e uso sem banco).

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use chrono::{DateTime, Utc};

use super::{BreakerState, GuardPersistence, LOGIN_KIND, RequestRecord, session_trigger};
use crate::model::{Id, Trigger};

/// Implementação em memória de [`GuardPersistence`]. Compartilhe via `Arc`
/// entre gates para simular um reinício do app sem perder as cotas.
#[derive(Debug, Default)]
pub struct MemoryGuardPersistence {
    requests: Mutex<Vec<RequestRecord>>,
    breakers: Mutex<HashMap<String, BreakerState>>,
}

impl MemoryGuardPersistence {
    pub fn new() -> Self {
        Self::default()
    }
    /// Cópia de todos os registros (para inspeção em testes).
    pub fn records(&self) -> Vec<RequestRecord> {
        self.requests.lock().unwrap().clone()
    }
}

impl GuardPersistence for MemoryGuardPersistence {
    fn record_request(&self, rec: &RequestRecord) -> crate::Result<()> {
        self.requests.lock().unwrap().push(rec.clone());
        Ok(())
    }

    fn count_sessions(
        &self,
        account_id: Option<Id>,
        trigger: Option<Trigger>,
        since: DateTime<Utc>,
    ) -> crate::Result<u32> {
        let n = self
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.at >= since)
            .filter(|r| account_id.is_none() || r.account_id == account_id)
            .filter(|r| match session_trigger(&r.kind) {
                Some(t) => trigger.is_none_or(|want| want == t),
                None => false,
            })
            .count();
        Ok(n as u32)
    }

    fn last_session_start(
        &self,
        account_id: Id,
        trigger: Trigger,
    ) -> crate::Result<Option<DateTime<Utc>>> {
        Ok(self
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.account_id == Some(account_id))
            .filter(|r| session_trigger(&r.kind) == Some(trigger))
            .map(|r| r.at)
            .max())
    }

    fn distinct_logins_since(&self, since: DateTime<Utc>) -> crate::Result<u32> {
        // Mesma semântica do `Store`: pares distintos (host, conta) em linhas `login`.
        let set: HashSet<(String, Id)> = self
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.at >= since && r.kind == LOGIN_KIND)
            .filter_map(|r| r.account_id.map(|a| (r.host.clone(), a)))
            .collect();
        Ok(set.len() as u32)
    }

    fn breaker(&self, scope: &str) -> crate::Result<BreakerState> {
        Ok(self
            .breakers
            .lock()
            .unwrap()
            .get(scope)
            .cloned()
            .unwrap_or_default())
    }

    fn set_breaker(&self, scope: &str, state: &BreakerState) -> crate::Result<()> {
        self.breakers
            .lock()
            .unwrap()
            .insert(scope.to_owned(), state.clone());
        Ok(())
    }
}
