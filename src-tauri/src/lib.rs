pub mod agy;
pub mod commands;
pub mod config;
pub mod generate;
pub mod vault;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use vault::paths::{resolve_vault_root, Vault};

/// Compiled in rather than read from disk: a packaged app must not depend on files sitting
/// next to the binary. Black on transparent, so macOS recolours it for the menu bar.
const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");

/// A menu-bar item carrying the same graduation cap as the app icon, so the two are
/// recognisably one application.
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Відкрити grind-test", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Вийти", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(TRAY_ICON)?)
        .icon_as_template(true)
        .tooltip("grind-test")
        .menu(&menu)
        // Left click reveals the window; the menu belongs on the right button, which is
        // where every other menu-bar item puts it.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => reveal(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn reveal(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_tray(app)?;
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let config = config::load(&config_dir);
            // Before anything can call a model: every tier lookup reads this.
            agy::configure(config.models.clone());
            let chosen = config.vault_root;
            // Packaged builds have no repo next to them, so fall back to the app data dir.
            let fallback = app.path().app_data_dir().ok().map(|dir| dir.join("vault"));
            let root = resolve_vault_root(chosen, fallback);
            app.manage(commands::AppState::new(Vault::new(root), config_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_info,
            commands::get_models,
            commands::set_models,
            commands::list_models,
            commands::choose_vault,
            commands::create_vault,
            commands::use_default_vault,
            commands::list_subjects,
            commands::get_subject,
            commands::get_knowledge,
            commands::generate_knowledge,
            commands::generate_quiz,
            commands::list_quizzes,
            commands::get_quiz,
            commands::submit_quiz,
            commands::get_hint,
            commands::list_attempts,
            commands::plan_study_session,
            commands::prepare_session_questions,
            commands::unfinished_sessions,
            commands::resume_study_session,
            commands::finish_study_session,
            commands::mark_topic_read,
            commands::plan_reading,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
