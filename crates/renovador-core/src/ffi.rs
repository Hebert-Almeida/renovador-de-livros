//! Ponto de entrada para o segundo plano no mobile (WorkManager/BGTask). W2-01.

use std::path::Path;
use std::sync::Arc;

use crate::clock::SystemClock;
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
pub fn run_background_cycle(db_path: &str, key: &DbKey) -> String {
    match std::panic::catch_unwind(|| run(db_path, key)) {
        Ok(Ok(outcomes)) => serde_json::json!({ "outcomes": outcomes }).to_string(),
        Ok(Err(e)) => error_json(&e.to_string()),
        Err(_) => error_json("pânico no ciclo em segundo plano"),
    }
}

fn run(db_path: &str, key: &DbKey) -> crate::Result<Vec<CycleOutcome>> {
    let store = Arc::new(Store::open(Path::new(db_path), key)?);
    let clock = Arc::new(SystemClock);
    let gate = Arc::new(DefaultGate::new(store.clone(), clock.clone()));
    let client = Arc::new(HttpSophiaClient::with_reqwest(gate.clone())?);
    let engine = Engine::new(gate, client, clock);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| crate::Error::Invalid(format!("runtime: {e}")))?;
    rt.block_on(engine.run_due(&store))
}
