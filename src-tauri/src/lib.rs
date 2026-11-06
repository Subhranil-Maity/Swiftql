mod config;
mod connect;
mod drivers;
mod export;
mod introspect;
mod notify;
mod query;
mod results;
mod session;
mod ssh;

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
            connect::connect::<tauri::Wry>,
            connect::disconnect,
            connect::list_sessions,
            query::run_statements,
            query::cancel_run,
            query::end_transaction,
            results::fetch_rows,
            results::sort_result,
            results::release_results,
            results::copy_result,
            results::export_result,
            introspect::introspect,
            notify::listen_channel,
            notify::unlisten_channel,
            notify::list_listen_channels,
            notify::send_notify,
            ssh::list_ssh_hosts,
            ssh::trust_ssh_host,
            ssh::clear_trusted_ssh_hosts,
        ])
        .events(collect_events![
            config::SettingsChanged,
            notify::NotificationReceived
        ])
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
/// starts, instead of white. Follows the saved theme, or the OS for "system".
fn match_theme(app: &tauri::App, theme: config_store::settings::Theme) {
    use config_store::settings::Theme;
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let dark = match theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => matches!(window.theme(), Ok(tauri::Theme::Dark)),
    };
    // --background in src/index.css: oklch(1 0 0) and oklch(0.145 0 0).
    let color = if dark {
        tauri::window::Color(10, 10, 10, 255)
    } else {
        tauri::window::Color(255, 255, 255, 255)
    };
    if let Err(err) = window.set_background_color(Some(color)) {
        tracing::warn!(error = %err, "failed to set the window background colour");
    }
}

/// Logs to stderr. `RUST_LOG` overrides the default: `info` in release
/// builds, `debug` for SwiftQL's own crates in dev builds.
fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let default = if cfg!(debug_assertions) {
        "info,swiftql_lib=debug,driver_postgres=debug,config_store=debug,ssh_tunnel=debug"
    } else {
        "info"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    export_bindings(&builder);

    let specta_handler = builder.invoke_handler();
    // Commands that take a raw body can't be specta commands.
    let raw_handler: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
        Box::new(tauri::generate_handler![export::save_export]);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(move |invoke| match invoke.message.command() {
            "save_export" => raw_handler(invoke),
            _ => specta_handler(invoke),
        })
        .setup(move |app| {
            builder.mount_events(app);
            let root = app.path().app_config_dir()?;
            let manager: config::Config = Arc::new(ConfigManager::open(root)?);
            match_theme(app, manager.settings().get().general.theme);
            app.manage(manager);
            app.manage(drivers::DriverRegistry::new());
            app.manage(session::SessionManager::default());
            app.manage(notify::Listeners::default());
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
