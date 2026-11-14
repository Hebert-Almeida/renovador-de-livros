//! Implementação de [`GuardPersistence`] sobre o `Store` (tabelas `request_log`
//! e `breaker_state`).
//!
//! Convenção de sessões: o gate registra o início de cada sessão como uma linha
//! de `request_log` com `kind = session_start_kind(trigger)` (ex.:
//! `session_start:manual`). As contagens de sessão usam apenas essas linhas,
//! por isso reiniciar o app não zera as cotas. Logins distintos são contados
//! pelas linhas com `kind = "login"` (por host + conta).

use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};

use super::Store;
use super::convert::*;
use crate::guard::{BreakerState, GuardPersistence, RequestOutcome, RequestRecord};
use crate::model::{Id, Trigger};

const SESSION_START_PREFIX: &str = "session_start:";

/// Valor de `request_log.kind` que marca o início de uma sessão.
pub fn session_start_kind(trigger: Trigger) -> String {
    format!("{SESSION_START_PREFIX}{}", trigger.as_str())
}

fn outcome_str(o: RequestOutcome) -> &'static str {
    match o {
        RequestOutcome::Ok => "ok",
        RequestOutcome::NetworkError => "network_error",
        RequestOutcome::ServerError => "server_error",
        RequestOutcome::UnexpectedHtml => "unexpected_html",
        RequestOutcome::RateLimited => "rate_limited",
        RequestOutcome::HumanVerification => "human_verification",
        RequestOutcome::InvalidCredentials => "invalid_credentials",
    }
}

impl GuardPersistence for Store {
    fn record_request(&self, rec: &RequestRecord) -> crate::Result<()> {
        self.conn()
            .execute(
                "INSERT INTO request_log (account_id, host, at, kind, session_id, http_status, \
                 outcome) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    rec.account_id,
                    rec.host,
                    ts(rec.at),
                    rec.kind,
                    rec.session_id,
                    rec.http_status,
                    outcome_str(rec.outcome)
                ],
            )
            .db()
            .map(|_| ())
    }

    fn count_sessions(
        &self,
        account_id: Option<Id>,
        trigger: Option<Trigger>,
        since: DateTime<Utc>,
    ) -> crate::Result<u32> {
        let exact = trigger.map(session_start_kind);
        self.conn()
            .query_row(
                "SELECT count(DISTINCT session_id) FROM request_log \
                 WHERE (CASE WHEN ?1 IS NULL THEN substr(kind, 1, ?4) = ?5 ELSE kind = ?1 END) \
                   AND at >= ?2 AND (?3 IS NULL OR account_id = ?3)",
                params![
                    exact,
                    ts(since),
                    account_id,
                    SESSION_START_PREFIX.len() as i64,
                    SESSION_START_PREFIX
                ],
                |r| r.get(0),
            )
            .db()
    }

    fn last_session_start(
        &self,
        account_id: Id,
        trigger: Trigger,
    ) -> crate::Result<Option<DateTime<Utc>>> {
        self.conn()
            .query_row(
                "SELECT at FROM request_log WHERE account_id = ?1 AND kind = ?2 \
                 ORDER BY at DESC LIMIT 1",
                params![account_id, session_start_kind(trigger)],
                |r| get_ts(r, 0),
            )
            .optional()
            .db()
    }

    fn distinct_logins_since(&self, since: DateTime<Utc>) -> crate::Result<u32> {
        self.conn()
            .query_row(
                "SELECT count(*) FROM (SELECT DISTINCT host, account_id FROM request_log \
                 WHERE kind = 'login' AND at >= ?1)",
                [ts(since)],
                |r| r.get(0),
            )
            .db()
    }

    fn breaker(&self, scope: &str) -> crate::Result<BreakerState> {
        Ok(self
            .conn()
            .query_row(
                "SELECT consecutive_failures, open_until, reason FROM breaker_state \
                 WHERE scope = ?1",
                [scope],
                |r| {
                    Ok(BreakerState {
                        consecutive_failures: r.get(0)?,
                        open_until: get_ts_opt(r, 1)?,
                        reason: r.get(2)?,
                    })
                },
            )
            .optional()
            .db()?
            .unwrap_or_default())
    }

    fn set_breaker(&self, scope: &str, state: &BreakerState) -> crate::Result<()> {
        self.conn()
            .execute(
                "INSERT INTO breaker_state (scope, consecutive_failures, open_until, reason) \
                 VALUES (?1, ?2, ?3, ?4) ON CONFLICT (scope) DO UPDATE SET \
                 consecutive_failures = excluded.consecutive_failures, \
                 open_until = excluded.open_until, reason = excluded.reason",
                params![
                    scope,
                    state.consecutive_failures,
                    ts_opt(state.open_until),
                    state.reason
                ],
            )
            .db()
            .map(|_| ())
    }
}
