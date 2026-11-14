//! Ponto de entrada para o segundo plano no mobile (WorkManager/BGTask). W2-01.

use crate::store::DbKey;

/// Executa um ciclo em segundo plano e devolve o `CycleReport` em JSON.
/// Nunca entra em pânico: erros viram `{"error": "..."}`.
pub fn run_background_cycle(_db_path: &str, _key: &DbKey) -> String {
    serde_json::json!({ "error": "não implementado" }).to_string()
}
