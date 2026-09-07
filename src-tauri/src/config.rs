//! Persisted app settings.
//!
//! Lives in the OS config directory, which is never gated behind macOS file-access
//! permission — so the app can always remember where the vault is, even when it currently
//! cannot read the vault itself.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Explicit vault location chosen by the user. `None` means "work it out".
    #[serde(default)]
    pub vault_root: Option<PathBuf>,
}

fn config_file(config_dir: &Path) -> PathBuf {
    config_dir.join("config.json")
}

pub fn load(config_dir: &Path) -> AppConfig {
    std::fs::read_to_string(config_file(config_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, config: &AppConfig) -> Result<()> {
    std::fs::create_dir_all(config_dir)
        .with_context(|| format!("creating {}", config_dir.display()))?;
    let path = config_file(config_dir);
    std::fs::write(&path, serde_json::to_string_pretty(config)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
