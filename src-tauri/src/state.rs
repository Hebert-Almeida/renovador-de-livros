//! Estado compartilhado do app (W2-02 preenche).

use std::sync::Mutex;

use renovador_core::store::Store;

#[derive(Default)]
#[allow(dead_code)] // usado a partir da W2-02
pub struct AppState {
    /// Banco aberto após obter a chave do cofre. `None` até o setup concluir.
    pub store: Mutex<Option<Store>>,
}
