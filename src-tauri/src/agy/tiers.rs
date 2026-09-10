//! Which model each tier resolves to, and what `agy` can run.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::RwLock;
use std::time::Duration;
use tokio::process::Command;

use super::{resolve_binary, truncate};

/// Which model a job is allowed to use. Ids live here and nowhere else.
///
/// The *tier* a job runs at is fixed by the job — knowledge and hints are always smart,
/// quizzes and sessions always fast — and only which model each tier names is configurable.
/// That keeps the economy rule enforceable while letting the student pick the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    /// Deep reasoning: knowledge notes and hints. Expensive, results are cached on disk.
    Smart,
    /// Cheap and quick: quiz and session generation, which happens constantly.
    Fast,
}

pub const DEFAULT_SMART: &str = "gemini-3.1-pro-high";
pub const DEFAULT_FAST: &str = "gemini-3.8-flash-medium";

/// The model id each tier resolves to. Anything `agy models` lists is valid, which is how
/// Claude, Gemini and the rest are all reachable without the app knowing about providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSettings {
    pub smart: String,
    pub fast: String,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            smart: DEFAULT_SMART.to_string(),
            fast: DEFAULT_FAST.to_string(),
        }
    }
}

/// Process-wide, so the dozens of call sites that ask for a tier stay unaware of settings.
/// Written once at startup and again whenever the settings screen saves.
static MODELS: RwLock<Option<ModelSettings>> = RwLock::new(None);

pub fn configure(settings: ModelSettings) {
    *MODELS
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(settings);
}

pub fn models() -> ModelSettings {
    MODELS
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .unwrap_or_default()
}

impl ModelTier {
    pub fn model_id(self) -> String {
        let settings = models();
        match self {
            ModelTier::Smart => settings.smart,
            ModelTier::Fast => settings.fast,
        }
    }
}

/// One entry from `agy models`.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
}

/// Ask the CLI what it can run, so the settings screen offers real ids rather than a list
/// this app would have to keep in step with Antigravity by hand.
pub async fn list_models() -> Result<Vec<ModelInfo>> {
    let binary = resolve_binary()?;
    let output = tokio::time::timeout(
        Duration::from_secs(60),
        Command::new(&binary)
            .arg("models")
            .stdin(Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| anyhow!("`agy models` timed out"))?
    .with_context(|| format!("running {} models", binary.display()))?;

    if !output.status.success() {
        bail!(
            "`agy models` exited with {}: {}",
            output.status,
            truncate(String::from_utf8_lossy(&output.stderr).trim(), 300)
        );
    }
    let models = parse_models(&String::from_utf8_lossy(&output.stdout));
    if models.is_empty() {
        bail!("`agy models` listed nothing");
    }
    Ok(models)
}

/// `id<TAB>Human label`, one per line, after a line or two of progress chatter.
fn parse_models(stdout: &str) -> Vec<ModelInfo> {
    stdout
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter_map(|(id, label)| {
            let id = id.trim();
            (!id.is_empty()).then(|| ModelInfo {
                id: id.to_string(),
                label: label.trim().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_model_listing() {
        let listing = "Fetching available models...\ngemini-3.1-pro-high\tGemini 3.1 Pro (High)\nclaude-opus-4-6-thinking\tClaude Opus 4.6 (Thinking)\n";
        let models = parse_models(listing);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gemini-3.1-pro-high");
        assert_eq!(models[1].label, "Claude Opus 4.6 (Thinking)");
    }

    #[test]
    fn tiers_default_to_the_shipped_ids() {
        let settings = ModelSettings::default();
        assert_eq!(settings.smart, DEFAULT_SMART);
        assert_eq!(settings.fast, DEFAULT_FAST);
    }
}
