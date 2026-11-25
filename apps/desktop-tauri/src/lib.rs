mod backend;
mod bridge;
mod commands;
mod store;

use backend::LocalBackendManager;
use bridge::{BridgeSeed, build_initialization_script};
use commands::AppState;
use mycode_core::firewall;
use std::sync::Arc;
use store::DesktopStore;
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

fn detect_system_locale() -> String {
    std::env::var("LANG")
        .ok()
        .and_then(|s| s.split('.').next().map(|v| v.replace('_', "-")))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "en-US".to_string())
}

fn node_platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

pub fn run() {
    // Enforce C++-level WebView2 air-gapped networking BEFORE Tauri initializes Edge WebView2.
    // SAFETY: Invoked on the single main thread prior to spawning any background threads or runtime.
    unsafe {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            firewall::WEBVIEW2_OFFLINE_BROWSER_ARGS,
        );
    }

    let store = Arc::new(DesktopStore::open());
    let backend_holder: Arc<parking_lot::Mutex<Option<Arc<LocalBackendManager>>>> =
        Arc::new(parking_lot::Mutex::new(None));
    let backend_for_exit = Arc::clone(&backend_holder);
    let store_for_setup = Arc::clone(&store);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_local_environment_bearer_token,
            commands::get_host_environment_id,
            commands::get_client_settings,
            commands::set_client_settings,
            commands::get_connection_catalog,
            commands::set_connection_catalog,
            commands::set_local_environment_enabled,
            commands::reveal_window,
            commands::set_theme,
            commands::pick_folder,
            commands::pick_project_favicon,
            commands::pick_theme_files,
            commands::open_external,
            commands::read_media,
            commands::to_local_backend_path,
            commands::get_server_exposure_state,
            commands::get_update_state,
            commands::get_wsl_state,
            commands::get_cli_command_state,
            commands::get_snap_shot_state,
            commands::rust_engine_info,
        ])
        .setup(move |app| {
            let resource_dir = app.path().resource_dir().ok();
            let backend = Arc::new(
                LocalBackendManager::start(resource_dir)
                    .map_err(|e| std::io::Error::other(e))?,
            );
            *backend_holder.lock() = Some(Arc::clone(&backend));

            let locale = detect_system_locale();
            let seed = BridgeSeed {
                platform: node_platform_name(),
                locale: locale.clone(),
                languages: vec![locale],
                local_environment_enabled: store_for_setup.get_local_environment_enabled(),
                bootstrap: backend.bootstrap_info(),
                version: env!("CARGO_PKG_VERSION"),
            };

            app.manage(AppState {
                backend: Arc::clone(&backend),
                store: Arc::clone(&store_for_setup),
            });

            let init_script = build_initialization_script(&seed);

            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("MyCode")
                .inner_size(1400.0, 900.0)
                .min_inner_size(960.0, 640.0)
                .initialization_script(&init_script)
                .on_navigation(|url| firewall::is_allowed_offline_url(url.as_str()))
                .build()?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building MyCode Tauri desktop application");

    app.run(move |_app_handle, event| {
        if let RunEvent::Exit = event
            && let Some(backend) = backend_for_exit.lock().take()
        {
            backend.stop();
        }
    });
}
