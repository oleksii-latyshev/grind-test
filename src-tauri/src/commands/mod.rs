//! The `#[tauri::command]` surface, grouped by the screen that calls it.

pub mod models;
pub mod quiz;
pub mod study;
pub mod subjects;
pub mod vault;

use std::path::PathBuf;
use std::sync::Mutex;

use crate::vault::Vault;

/// Progress events for a running knowledge batch.
pub const GENERATION_EVENT: &str = "generation://progress";

pub struct AppState {
    /// Swappable at runtime: the user can point the app at a different folder without
    /// restarting it.
    vault: Mutex<Vault>,
    config_dir: PathBuf,
}

impl AppState {
    pub fn new(vault: Vault, config_dir: PathBuf) -> Self {
        Self {
            vault: Mutex::new(vault),
            config_dir,
        }
    }

    /// A lock poisoned by a panic elsewhere should not take the whole app down — the value
    /// behind it is just a path.
    pub fn vault(&self) -> Vault {
        self.vault
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn replace_vault(&self, root: PathBuf) {
        *self
            .vault
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Vault::new(root);
    }

    /// `$GRIND_VAULT` is itself an explicit choice, so a shell that sets it never sees the
    /// setup screen.
    fn configured(&self) -> bool {
        std::env::var_os("GRIND_VAULT").is_some() || crate::config::load(&self.config_dir).onboarded
    }

    /// Point the app at `root` and record that setup is done.
    fn adopt_vault(&self, root: PathBuf) -> CmdResult<()> {
        let mut config = crate::config::load(&self.config_dir);
        config.vault_root = Some(root.clone());
        config.onboarded = true;
        crate::config::save(&self.config_dir, &config).map_err(fail)?;
        self.replace_vault(root);
        Ok(())
    }
}

type CmdResult<T> = Result<T, String>;

fn fail(error: anyhow::Error) -> String {
    format!("{error:#}")
}
