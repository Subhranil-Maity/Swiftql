mod config;
mod connect;
mod drivers;
mod introspect;
mod query;
mod session;

use std::sync::Arc;

use config_store::ConfigManager;
use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

/// Where the generated TS bindings go, relative to `src-tauri/`.
const BINDINGS_PATH: &str = "../src/bindings.ts";

/// Every command, event and extra type the frontend can see.
fn specta_builder() -> Builder<tauri::Wry> {
    use driver_core::{notice, types, value};

    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
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
            drivers::list_drivers,
            connect::test_connection,
            connect::check_connection,
            connect::connect,
            connect::disconnect,
            connect::list_sessions,
            query::run_statements,
            query::cancel_run,
            introspect::introspect,
        ])
        .events(collect_events![config::SettingsChanged])
        // Not used by a command yet, exported so the whole data model is in TS.
        .typ::<value::Value>()
        .typ::<types::ColumnMeta>()
        .typ::<notice::Notice>()
        .typ::<query::RunEvent>()
        .typ::<query::RunSummary>()
        .typ::<driver_core::error::DbError>()
        .typ::<config_store::ConfigError>()
        // i64 appears in Value::Int, sortOrder and Interval.microseconds. The
        // driver already sends values over 2^53 as Value::BigInt(string).
        .dangerously_cast_bigints_to_number()
}

/// Writes `src/bindings.ts`.
fn export_bindings(builder: &Builder<tauri::Wry>) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(BINDINGS_PATH);
    builder
        .export(
            specta_typescript::Typescript::default().header("// @ts-nocheck\n/* eslint-disable */"),
            path,
        )
        .expect("failed to export typescript bindings");
}

/// Paints the window in the app's background colour while the webview
/// starts, instead of white. Follows the OS theme: the app theme lives in the
/// webview's localStorage, which isn't readable here yet.
// TODO: use Settings.general.theme once the theme is stored there (stores/theme.ts).
fn match_os_theme(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    // --background in src/index.css: oklch(1 0 0) and oklch(0.145 0 0).
    let color = match window.theme() {
        Ok(tauri::Theme::Dark) => tauri::window::Color(10, 10, 10, 255),
        _ => tauri::window::Color(255, 255, 255, 255),
    };
    if let Err(err) = window.set_background_color(Some(color)) {
        tracing::warn!(error = %err, "failed to set the window background colour");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            match_os_theme(app);
            let root = app.path().app_config_dir()?;
            let manager: config::Config = Arc::new(ConfigManager::open(root)?);
            app.manage(manager);
            app.manage(drivers::DriverRegistry::new());
            app.manage(session::SessionManager::default());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    /// `bun run gen:bindings` runs this to regenerate `src/bindings.ts`.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
