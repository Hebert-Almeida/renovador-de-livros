//! Ponto de entrada para o segundo plano no mobile (WorkManager/BGTask). W2-01.

use std::path::Path;
use std::sync::Arc;

use crate::clock::{Clock, SystemClock};
use crate::engine::notes::{self, Note};
use crate::engine::{CycleOutcome, Engine};
use crate::guard::DefaultGate;
use crate::sophia::client::HttpSophiaClient;
use crate::store::{DbKey, Store};

fn error_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

/// Executa um ciclo em segundo plano e devolve o `CycleReport` em JSON.
/// Nunca entra em pânico: erros viram `{"error": "..."}`.
///
/// Formato de sucesso: `{"outcomes": [CycleOutcome, ...]}`, com um item por
/// conta que tinha tarefa vencida (`report` é o `CycleReport`; `notifications`
/// diz o que o app deve notificar). Sem tarefa vencida, `outcomes` vem vazio e
/// nenhuma requisição é feita.
///
/// `notes` traz o texto pronto (`{title, body}`) das notificações a exibir, o
/// mesmo do desktop; vem vazio se o usuário desligou as notificações.
pub fn run_background_cycle(db_path: &str, key: &DbKey) -> String {
    match std::panic::catch_unwind(|| run(db_path, key)) {
        Ok(Ok((outcomes, notes))) => {
            serde_json::json!({ "outcomes": outcomes, "notes": notes }).to_string()
        }
        Ok(Err(e)) => error_json(&e.to_string()),
        Err(_) => error_json("pânico no ciclo em segundo plano"),
    }
}

fn run(db_path: &str, key: &DbKey) -> crate::Result<(Vec<CycleOutcome>, Vec<Note>)> {
    let store = Arc::new(Store::open(Path::new(db_path), key)?);
    let clock = Arc::new(SystemClock);
    let gate = Arc::new(DefaultGate::new(store.clone(), clock.clone()));
    let client = Arc::new(HttpSophiaClient::with_reqwest(gate.clone())?);
    let engine = Engine::new(gate, client, clock.clone());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| crate::Error::Invalid(format!("runtime: {e}")))?;
    let outcomes = rt.block_on(engine.run_due(&store))?;
    let notes = render_notes(&store, &outcomes, clock.now());
    Ok((outcomes, notes))
}

/// Texto das notificações do ciclo, respeitando a opção "notificações" dos Ajustes.
fn render_notes(
    store: &Store,
    outcomes: &[CycleOutcome],
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<Note> {
    if store.settings_get().is_ok_and(|s| !s.notifications) {
        return Vec::new();
    }
    let today = crate::planner::local_date(now);
    let library = |id| store.account_get(id).ok().map(|a| a.library_name);
    outcomes
        .iter()
        .flat_map(|o| &o.notifications)
        .map(|n| notes::from_engine(n, today, library))
        .collect()
}
