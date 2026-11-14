//! Eventos tipados (contrato congelado).
//! Nomes no frontend: `cycle-progress-event`, `cycle-finished-event`, `automation-state-event`
//! (equivalentes a `cycle://progress`, `cycle://finished`, `automation://state` do plano).

use renovador_core::model::{AutomationState, CycleProgress, CycleReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct CycleProgressEvent(pub CycleProgress);

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct CycleFinishedEvent(pub CycleReport);

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct AutomationStateEvent(pub AutomationState);
