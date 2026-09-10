//! Which model each tier names — the only part of model use the student configures.

use serde::Serialize;
use tauri::State;

use super::{fail, AppState, CmdResult};
use crate::agy::ModelSettings;

#[derive(Debug, Serialize)]
pub struct ModelChoice {
    pub models: ModelSettings,
    /// What the app ships with, so the settings screen can offer "back to defaults" without
    /// duplicating the ids.
    pub defaults: ModelSettings,
}

#[tauri::command]
pub fn get_models(_state: State<'_, AppState>) -> ModelChoice {
    ModelChoice {
        models: crate::agy::models(),
        defaults: ModelSettings::default(),
    }
}

/// Persist the tier→model mapping and apply it to the running process.
///
/// Which tier a job runs at is not negotiable — knowledge and hints are smart, quizzes and
/// sessions fast. Only the model each tier names changes here.
#[tauri::command]
pub fn set_models(state: State<'_, AppState>, models: ModelSettings) -> CmdResult<ModelChoice> {
    if models.smart.trim().is_empty() || models.fast.trim().is_empty() {
        return Err("оберіть модель для обох рівнів".to_string());
    }
    let models = ModelSettings {
        smart: models.smart.trim().to_string(),
        fast: models.fast.trim().to_string(),
    };

    let mut config = crate::config::load(&state.config_dir);
    config.models = models.clone();
    crate::config::save(&state.config_dir, &config).map_err(fail)?;
    crate::agy::configure(models);

    Ok(get_models(state))
}

/// What `agy` can run. Asked of the CLI rather than hardcoded, so the list stays in step
/// with Antigravity without this app tracking providers.
#[tauri::command]
pub async fn list_models() -> CmdResult<Vec<crate::agy::ModelInfo>> {
    crate::agy::list_models().await.map_err(fail)
}
