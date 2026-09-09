pub mod agy;
pub mod commands;
pub mod config;
pub mod generate;
pub mod vault;

use vault::paths::{resolve_vault_root, Vault};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            use tauri::Manager;
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let chosen = config::load(&config_dir).vault_root;
            // Packaged builds have no repo next to them, so fall back to the app data dir.
            let fallback = app.path().app_data_dir().ok().map(|dir| dir.join("vault"));
            let root = resolve_vault_root(chosen, fallback);
            app.manage(commands::AppState::new(Vault::new(root), config_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_info,
            commands::choose_vault,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
