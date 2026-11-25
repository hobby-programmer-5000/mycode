use crate::backend::{LocalBackendManager, loopback_http_request_raw};
use crate::store::DesktopStore;
use base64::Engine as _;
use mycode_core::firewall;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, Theme};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

pub struct AppState {
    pub backend: Arc<LocalBackendManager>,
    pub store: Arc<DesktopStore>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopMediaRead {
    pub data_url: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedThemeFile {
    pub file_name: String,
    pub contents: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PickFolderOptions {
    pub initial_path: Option<String>,
}

#[tauri::command]
pub fn get_local_environment_bearer_token(state: State<'_, AppState>) -> Result<String, String> {
    state.backend.exchange_bearer_token()
}

#[tauri::command]
pub fn get_host_environment_id(state: State<'_, AppState>) -> Option<String> {
    state.backend.read_host_environment_id()
}

#[tauri::command]
pub fn get_client_settings(state: State<'_, AppState>) -> Option<Value> {
    state.store.get_client_settings()
}

#[tauri::command]
pub fn set_client_settings(state: State<'_, AppState>, settings: Value) -> Result<(), String> {
    state.store.set_client_settings(settings)
}

#[tauri::command]
pub fn get_connection_catalog(state: State<'_, AppState>) -> Option<String> {
    state.store.get_connection_catalog()
}

#[tauri::command]
pub fn set_connection_catalog(
    state: State<'_, AppState>,
    catalog: String,
) -> Result<bool, String> {
    state.store.set_connection_catalog(catalog)?;
    Ok(true)
}

#[tauri::command]
pub fn set_local_environment_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.store.set_local_environment_enabled(enabled)
}

#[tauri::command]
pub fn reveal_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub fn set_theme(app: AppHandle, state: State<'_, AppState>, theme: String) -> Result<(), String> {
    let _ = state.store.set_theme(theme.clone());
    if let Some(win) = app.get_webview_window("main") {
        let next = match theme.as_str() {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            _ => None,
        };
        let _ = win.set_theme(next);
    }
    Ok(())
}

#[tauri::command]
pub async fn pick_folder(
    app: AppHandle,
    options: Option<PickFolderOptions>,
) -> Result<Option<String>, String> {
    let initial = options.and_then(|o| o.initial_path);
    tokio::task::spawn_blocking(move || {
        let mut dialog = app.dialog().file();
        if let Some(path) = initial {
            let pb = PathBuf::from(&path);
            if pb.exists() {
                dialog = dialog.set_directory(pb);
            }
        }
        dialog
            .blocking_pick_folder()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_project_favicon(
    app: AppHandle,
    initial_path: Option<String>,
) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        let mut dialog = app.dialog().file().add_filter(
            "Images",
            &["png", "jpg", "jpeg", "svg", "ico", "webp", "gif"],
        );
        if let Some(path) = initial_path {
            let pb = PathBuf::from(&path);
            if pb.exists() {
                dialog = dialog.set_directory(pb);
            }
        }
        dialog
            .blocking_pick_file()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_theme_files(app: AppHandle) -> Result<Option<Vec<PickedThemeFile>>, String> {
    tokio::task::spawn_blocking(move || {
        let picked = app
            .dialog()
            .file()
            .add_filter("Theme JSON", &["json", "jsonc"])
            .blocking_pick_files()?;
        let mut results = Vec::new();
        for fp in picked {
            if let Ok(path) = fp.into_path()
                && let Ok(contents) = fs::read_to_string(&path)
            {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "theme.json".to_string());
                results.push(PickedThemeFile {
                    file_name,
                    contents,
                });
            }
        }
        Some(results)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> Result<bool, String> {
    if !firewall::is_safe_external_open_url(&url) {
        return Ok(false);
    }
    match app.opener().open_url(&url, None::<&str>) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Strictly loopback-only media reader (blocks every external/LAN URL).
#[tauri::command]
pub async fn read_media(url: String) -> Result<Option<DesktopMediaRead>, String> {
    if !firewall::is_loopback_http_url(&url) {
        return Ok(None);
    }
    let Ok(parsed) = url::Url::parse(&url) else {
        return Ok(None);
    };
    let port = parsed.port_or_known_default().unwrap_or(80);
    let path_and_query = match parsed.query() {
        Some(q) => format!("{}?{q}", parsed.path()),
        None => parsed.path().to_string(),
    };

    tokio::task::spawn_blocking(move || {
        match loopback_http_request_raw(port, "GET", &path_and_query, None, None) {
            Ok((200, mime_type, bytes)) => {
                let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                Some(DesktopMediaRead {
                    data_url: format!("data:{mime_type};base64,{encoded}"),
                    mime_type,
                })
            }
            _ => None,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn to_local_backend_path(backend_id: String, path: String) -> Option<String> {
    if backend_id == "primary" {
        Some(path)
    } else {
        None
    }
}

#[tauri::command]
pub fn get_server_exposure_state() -> Value {
    json!({
        "mode": "local-only",
        "tailscaleServeEnabled": false,
        "tailscaleServePort": 443,
        "tailscaleAvailable": false,
        "tailscaleHttpsUrl": null,
        "lanAddress": null
    })
}

#[tauri::command]
pub fn get_update_state() -> Value {
    json!({
        "enabled": false,
        "status": "disabled",
        "channel": "latest",
        "currentVersion": env!("CARGO_PKG_VERSION"),
        "hostArch": if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" },
        "appArch": if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" },
        "runningUnderArm64Translation": false,
        "availableVersion": null,
        "downloadedVersion": null,
        "releaseNotes": [],
        "omittedReleaseCount": 0,
        "downloadPercent": null,
        "checkedAt": null,
        "message": "Automatic updates are disabled in offline MyCode builds.",
        "errorContext": null,
        "canRetry": false
    })
}

#[tauri::command]
pub fn get_wsl_state() -> Value {
    json!({
        "supported": false,
        "available": false,
        "enabled": false,
        "wslOnly": false,
        "distro": null,
        "distros": [],
        "runningDistro": null
    })
}

#[tauri::command]
pub fn get_cli_command_state() -> Value {
    json!({
        "supported": false,
        "installedPath": null,
        "onPath": false,
        "launcherPath": null
    })
}

#[tauri::command]
pub fn get_snap_shot_state() -> Value {
    json!({
        "mode": "unavailable",
        "windows": cfg!(target_os = "windows"),
        "shortcut": "double-shift",
        "shortcutRegistered": false,
        "shortcutMessage": null,
        "message": "SnapShot is disabled in this lightweight Rust build."
    })
}

#[tauri::command]
pub fn rust_engine_info() -> Value {
    json!({
        "shell": "tauri-2-rust",
        "version": env!("CARGO_PKG_VERSION"),
        "offlineLocked": true,
        "allowedProviders": ["claude-code", "codex", "antigravity"]
    })
}
