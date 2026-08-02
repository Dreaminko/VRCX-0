use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tracing::Level;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use crate::deep_link::{parse_deep_link, queue_deep_link_action, DEEP_LINK_ARRIVED_EVENT};
use crate::error::AppError;
use crate::state::{AppState, BACKGROUND_MODE_RESUME_ROUTE_STORAGE_KEY};

use super::adapters::{
    start_host_services, start_mcp_server_if_enabled, TauriDesktopNotifier, TauriUpdaterPort,
};
use super::autostart::{apply_autostart_window_state_if_needed, sync_autostart_from_db};
use super::shared::app_language;
use super::window::{configure_tray, create_main_window, disable_windows_default_context_menu};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn should_capture_gui_error(level: &Level, target: &str) -> bool {
    level == &Level::ERROR
        && (target == "vrcx_0" || target.starts_with("vrcx_0::") || target.starts_with("vrcx_0_"))
}

pub fn init_error_logging(app_data: Option<PathBuf>) {
    let Some(app_data) = app_data.or_else(vrcx_0_host::error_log::default_app_data_dir) else {
        return;
    };

    let default_panic_hook = std::panic::take_hook();
    let panic_app_data = app_data.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        vrcx_0_host::error_log::append_panic_error_log_with_version(
            &panic_app_data,
            panic_info,
            APP_VERSION,
        );
        default_panic_hook(panic_info);
    }));

    let tracing_app_data = app_data;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "vrcx_0=info".into()),
            ),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(move || {
                    vrcx_0_host::error_log::ErrorLogWriter::new_with_version(
                        tracing_app_data.clone(),
                        APP_VERSION,
                    )
                })
                .with_filter(filter_fn(|metadata| {
                    should_capture_gui_error(metadata.level(), metadata.target())
                })),
        )
        .init();
}

pub fn init_tls_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

pub fn updater_public_key() -> String {
    match option_env!("TAURI_UPDATER_PUBLIC_KEY") {
        Some(value) if !value.trim().is_empty() => value.to_string(),
        _ => "TAURI_UPDATER_PUBLIC_KEY_NOT_CONFIGURED".to_string(),
    }
}

pub fn app_update_build_label() -> String {
    option_env!("VRCX_0_BUILD_LABEL")
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

pub fn app_update_build_badge() -> String {
    option_env!("VRCX_0_BUILD_BADGE")
        .unwrap_or("")
        .trim()
        .to_string()
}

pub fn app_update_check_disabled() -> bool {
    option_env!("VRCX_0_DISABLE_UPDATE_CHECK") == Some("1")
}

pub fn apply_linux_webkit_workaround() {
    #[cfg(target_os = "linux")]
    {
        use webkit2gtk_nvidia_quirk::{apply_workaround_with_options, ApplyWorkaroundOptions};

        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            tracing::info!("disabling WebKitGTK DMABUF renderer on Linux");
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }

        apply_workaround_with_options(ApplyWorkaroundOptions::default());
    }
}

fn initialize_app_state(
    app: &tauri::App,
    app_data_dir: vrcx_0_host::app_paths::AppDataDirResolution,
    updater_port: Arc<TauriUpdaterPort>,
) -> AppState {
    let error = match AppState::new(app_data_dir.clone(), updater_port.clone()) {
        Ok(state) => return state,
        Err(error) => error,
    };

    if is_database_corruption_error(&error) {
        match quarantine_corrupt_database(&app_data_dir.current_dir) {
            Ok(quarantined) => {
                tracing::error!(
                    error = %error,
                    quarantined = %quarantined.display(),
                    "local database is corrupted; quarantined it to recreate a fresh database"
                );
                match AppState::new(app_data_dir, updater_port) {
                    Ok(state) => {
                        show_blocking_dialog(
                            app,
                            MessageDialogKind::Warning,
                            &format!(
                                "The local database was corrupted and could not be opened.\n\n\
                                 It was moved to:\n{}\n\n\
                                 VRCX-0 created a fresh database; please sign in again.",
                                quarantined.display()
                            ),
                        );
                        return state;
                    }
                    Err(retry_error) => exit_with_startup_error(app, &retry_error),
                }
            }
            Err(quarantine_error) => {
                tracing::error!(
                    error = %quarantine_error,
                    "failed to quarantine the corrupted local database"
                );
            }
        }
    }

    exit_with_startup_error(app, &error)
}

fn is_database_corruption_error(error: &AppError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("malformed") || message.contains("not a database")
}

fn quarantine_corrupt_database(app_data: &std::path::Path) -> std::io::Result<PathBuf> {
    let db_file = vrcx_0_host::app_paths::AppPaths::from_app_data(app_data.to_path_buf()).db_file;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let quarantined = appended_path(&db_file, &format!(".corrupt-{timestamp}"));
    std::fs::rename(&db_file, &quarantined)?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = appended_path(&db_file, suffix);
        if sidecar.exists() {
            let _ = std::fs::rename(&sidecar, appended_path(&quarantined, suffix));
        }
    }
    Ok(quarantined)
}

fn appended_path(path: &std::path::Path, suffix: &str) -> PathBuf {
    let mut appended = path.as_os_str().to_os_string();
    appended.push(suffix);
    PathBuf::from(appended)
}

fn exit_with_startup_error(app: &tauri::App, error: &AppError) -> ! {
    tracing::error!(error = %error, "failed to initialize app state");
    show_blocking_dialog(
        app,
        MessageDialogKind::Error,
        &format!("VRCX-0 failed to start.\n\n{error}"),
    );
    std::process::exit(1);
}

fn show_blocking_dialog(app: &tauri::App, kind: MessageDialogKind, message: &str) {
    app.dialog()
        .message(message)
        .kind(kind)
        .title("VRCX-0")
        .blocking_show();
}

pub fn setup_app_with_data_dir(
    app: &mut tauri::App,
    app_data_dir: vrcx_0_host::app_paths::AppDataDirResolution,
) -> Result<(), Box<dyn std::error::Error>> {
    let updater_port = Arc::new(TauriUpdaterPort::new(app.handle().clone()));
    let app_state = initialize_app_state(app, app_data_dir, updater_port);
    let language = app_language(&app_state);
    app.manage(app_state);

    let state = app.state::<AppState>();
    state
        .desktop
        .services
        .set_notification_desktop_notifier(Arc::new(TauriDesktopNotifier::new(
            app.handle().clone(),
        )));
    let _ = state
        .storage
        .remove(BACKGROUND_MODE_RESUME_ROUTE_STORAGE_KEY);
    state.runtime_context.runtime.record_phase(
        "appState",
        "completed",
        "Backend AppState initialized.",
    );
    state.runtime_context.sync.record(
        "startup",
        "running",
        "Tauri setup is wiring runtime services.",
        0,
    );
    create_main_window(app.handle(), state.web.proxy_url())?;
    state.runtime_context.runtime.record_phase(
        "mainWindow",
        "completed",
        "Main webview window created.",
    );

    disable_windows_default_context_menu(app.handle());

    let state = app.state::<AppState>();
    configure_tray(app, &state)?;
    state
        .runtime_context
        .runtime
        .record_phase("tray", "completed", "System tray configured.");
    #[cfg(target_os = "macos")]
    crate::macos_menu::configure_macos_app_menu(app.handle(), &language)?;
    #[cfg(not(target_os = "macos"))]
    let _ = language;
    sync_autostart_from_db(app, &state);
    apply_autostart_window_state_if_needed(app, &state);
    start_host_services(app.handle(), &state);
    start_mcp_server_if_enabled(app.handle());
    wire_deep_links(app.handle());
    state
        .runtime_context
        .sync
        .record("startup", "ready", "Backend host services are ready.", 0);

    Ok(())
}

fn wire_deep_links(app: &tauri::AppHandle) {
    #[cfg(all(debug_assertions, any(windows, target_os = "linux")))]
    if let Err(error) = app.deep_link().register_all() {
        tracing::warn!(error = %error, "failed to register development deep link schemes");
    }

    match app.deep_link().get_current() {
        Ok(Some(urls)) => {
            for url in urls {
                queue_deep_link_url(app, url.as_str());
            }
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(error = %error, "failed to read launch deep links");
        }
    }

    let app_handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            queue_deep_link_url(&app_handle, url.as_str());
        }
    });
}

fn queue_deep_link_url(app: &tauri::AppHandle, value: &str) {
    let Some(action) = parse_deep_link(value) else {
        tracing::warn!(url = %value, "ignored unsupported deep link");
        return;
    };
    let Some(state) = app.try_state::<AppState>() else {
        tracing::warn!(url = %value, "ignored deep link before app state was ready");
        return;
    };
    queue_deep_link_action(&state.pending_deep_links, action, || {
        let app_handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || {
            show_main_window_for_deep_link(&app_handle);
            emit_deep_link_arrived(&app_handle);
        }) {
            tracing::warn!(error = %error, "failed to schedule deep link window restore");
        }
    });
}

fn show_main_window_for_deep_link(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(error) =
            super::window::restore_foreground_window_from_background_mode(app, &state)
        {
            tracing::warn!(error = %error, "failed to show main window from deep link");
        }
        return;
    }

    if let Err(error) = super::window::ensure_main_window(app) {
        tracing::warn!(error = %error, "failed to show main window from deep link");
    }
}

fn emit_deep_link_arrived(app: &tauri::AppHandle) {
    if let Err(error) = app.emit(DEEP_LINK_ARRIVED_EVENT, serde_json::json!({})) {
        tracing::warn!(error = %error, "failed to emit deep link wake event");
    }
}

#[cfg(test)]
mod tests {
    use super::should_capture_gui_error;
    use tracing::Level;

    #[test]
    fn gui_error_log_captures_only_own_error_targets() {
        for target in [
            "vrcx_0",
            "vrcx_0::bootstrap::adapters",
            "vrcx_0_application",
            "vrcx_0_application::auth",
        ] {
            assert!(should_capture_gui_error(&Level::ERROR, target), "{target}");
        }

        for target in [
            "rustls_platform_verifier::verification::windows",
            "tauri_plugin_updater::updater",
            "tauri_runtime_wry",
            "vrcx_0x",
            "vrcx",
        ] {
            assert!(!should_capture_gui_error(&Level::ERROR, target), "{target}");
        }

        for level in [Level::WARN, Level::INFO, Level::DEBUG, Level::TRACE] {
            assert!(!should_capture_gui_error(
                &level,
                "vrcx_0::bootstrap::adapters"
            ));
        }
    }
}
