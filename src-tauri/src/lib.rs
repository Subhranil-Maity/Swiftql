mod config;

use std::sync::Arc;

use config_store::ConfigManager;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = app.path().app_config_dir()?;
            let manager: config::Config = Arc::new(ConfigManager::open(root)?);
            app.manage(manager);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            config::get_settings,
            config::update_settings,
            config::reset_settings,
            config::list_connections,
            config::recent_connections,
            config::get_connection,
            config::save_connection,
            config::delete_connection,
            config::touch_connection,
            config::set_connection_status,
            config::clear_connection_secret,
            config::keyring_status,
            config::get_app_state,
            config::set_app_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
