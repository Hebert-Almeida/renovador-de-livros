//! Ciclo de renovação (W2-01): política → gate → login → lista → sincronizar →
//! planejar → renovar e confirmar → gravar → logout → relatório.

use std::sync::Arc;

use crate::clock::Clock;
use crate::guard::RequestGate;
pub use crate::model::CycleReport;
use crate::model::{CycleKind, CycleProgress, Id, Trigger};
use crate::sophia::SophiaClient;
use crate::store::Store;
use crate::{Error, Result};

pub struct Engine {
    pub gate: Arc<dyn RequestGate>,
    pub client: Arc<dyn SophiaClient>,
    pub clock: Arc<dyn Clock>,
}

pub type ProgressFn<'a> = &'a (dyn Fn(CycleProgress) + Send + Sync);

impl Engine {
    pub async fn run_cycle(
        &self,
        _store: &Store,
        _account_id: Id,
        _kind: CycleKind,
        _trigger: Trigger,
        _progress: ProgressFn<'_>,
    ) -> Result<CycleReport> {
        Err(Error::todo("engine::run_cycle"))
    }
}
