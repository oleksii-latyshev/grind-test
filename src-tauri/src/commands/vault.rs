//! Where the vault is: setup, the folder picker, and what the app can see there.

use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};

use super::{AppState, CmdResult};
use crate::vault::paths;

#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub root: String,
    /// The student has been through setup and said where the vault is. Until then the app
    /// must not read the vault path at all — see `describe`.
    pub configured: bool,
    /// The app can actually list the syllabus directory right now.
    pub readable: bool,
    pub subject_count: usize,
    /// Why it is not readable, phrased for the user.
    pub error: Option<String>,
    pub agy_binary: Option<String>,
}

fn agy_binary() -> Option<String> {
    crate::agy::resolve_binary()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
}

/// Describe the vault, probing the filesystem only once the student has pointed at one.
///
/// The probe is what raises a macOS folder-access prompt, so before setup it must not
/// happen: the first system panel anyone sees should be the folder picker they asked for.
fn describe(state: &AppState) -> VaultInfo {
    let vault = state.vault();
    let configured = state.configured();
    if !configured {
        return VaultInfo {
            root: vault.root.to_string_lossy().to_string(),
            configured: false,
            readable: false,
            subject_count: 0,
            error: None,
            agy_binary: agy_binary(),
        };
    }
    let (readable, subject_count, error) = match paths::describe_vault(&vault.root) {
        Ok(count) => (true, count, None),
        Err(message) => (false, 0, Some(message)),
    };
    VaultInfo {
        root: vault.root.to_string_lossy().to_string(),
        configured: true,
        readable,
        subject_count,
        error,
        agy_binary: agy_binary(),
    }
}

#[tauri::command]
pub fn vault_info(state: State<'_, AppState>) -> VaultInfo {
    describe(&state)
}

/// Create a vault in a folder the student picks, with the directory skeleton and — when
/// there is no syllabus anywhere in it — one sample file showing the format.
#[tauri::command]
pub async fn create_vault(app: tauri::AppHandle) -> CmdResult<VaultInfo> {
    let state = app.state::<AppState>();
    let Some(parent) = pick_folder(&app, "Оберіть теку, де створити сховище").await?
    else {
        return Ok(describe(&state));
    };

    // Picking a folder that is already a vault means "use this one", not "nest another".
    let root = if parent.join("syllabus").is_dir() {
        parent
    } else {
        parent.join("grind-vault")
    };
    paths::create_vault(&root).map_err(|error| {
        format!(
            "не вдалося створити сховище в «{}»: {error}",
            root.display()
        )
    })?;
    state.adopt_vault(root)?;
    Ok(describe(&state))
}

/// Accept the folder the app already resolved — the app data dir in a packaged build, the
/// repository vault in development — without opening a picker.
#[tauri::command]
pub fn use_default_vault(state: State<'_, AppState>) -> CmdResult<VaultInfo> {
    let root = state.vault().root;
    paths::create_vault(&root)
        .map_err(|error| format!("не вдалося підготувати «{}»: {error}", root.display()))?;
    state.adopt_vault(root)?;
    Ok(describe(&state))
}

/// Let the user point the app at the vault through the system folder picker.
///
/// Beyond being the obvious way to move a vault, on macOS this is also the way *back* from a
/// denied folder-access prompt: choosing a folder in the system panel is an explicit grant,
/// so the user can see exactly what they are allowing.
#[tauri::command]
pub async fn choose_vault(app: tauri::AppHandle) -> CmdResult<VaultInfo> {
    let state = app.state::<AppState>();
    let Some(picked) = pick_folder(&app, "Оберіть теку сховища").await? else {
        // Cancelled: report what is configured now rather than treating it as an error.
        return Ok(describe(&state));
    };
    state.adopt_vault(paths::normalise_vault_choice(&picked))?;
    Ok(describe(&state))
}

/// The system folder panel. Picking a folder here is the macOS grant, which is why it is the
/// only place the app ever reaches outside its own data directory.
async fn pick_folder(app: &tauri::AppHandle, title: &str) -> CmdResult<Option<PathBuf>> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
        .pick_folder(move |picked| {
            let _ = tx.send(picked);
        });

    let picked = rx
        .await
        .map_err(|_| "вікно вибору теки закрилося".to_string())?;
    picked
        .map(|path| path.into_path().map_err(|error| error.to_string()))
        .transpose()
}
