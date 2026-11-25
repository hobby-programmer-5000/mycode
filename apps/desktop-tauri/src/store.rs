use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStoreData {
    pub client_settings: Option<Value>,
    pub connection_catalog: Option<String>,
    pub local_environment_enabled: Option<bool>,
    pub theme: Option<String>,
}

pub struct DesktopStore {
    path: PathBuf,
    data: Mutex<DesktopStoreData>,
}

impl DesktopStore {
    pub fn open() -> Self {
        let base = dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("MyCode");
        let _ = fs::create_dir_all(&base);
        let path = base.join("tauri-desktop-store.json");
        let data = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<DesktopStoreData>(&raw).ok())
            .unwrap_or_default();
        Self {
            path,
            data: Mutex::new(data),
        }
    }

    fn persist_locked(&self, data: &DesktopStoreData) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let serialized = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serialized).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }

    pub fn get_client_settings(&self) -> Option<Value> {
        self.data.lock().client_settings.clone()
    }

    pub fn set_client_settings(&self, value: Value) -> Result<(), String> {
        let mut guard = self.data.lock();
        guard.client_settings = Some(value);
        self.persist_locked(&guard)
    }

    pub fn get_connection_catalog(&self) -> Option<String> {
        self.data.lock().connection_catalog.clone()
    }

    pub fn set_connection_catalog(&self, catalog: String) -> Result<(), String> {
        let mut guard = self.data.lock();
        guard.connection_catalog = Some(catalog);
        self.persist_locked(&guard)
    }

    pub fn get_local_environment_enabled(&self) -> bool {
        self.data.lock().local_environment_enabled.unwrap_or(true)
    }

    pub fn set_local_environment_enabled(&self, enabled: bool) -> Result<(), String> {
        let mut guard = self.data.lock();
        guard.local_environment_enabled = Some(enabled);
        self.persist_locked(&guard)
    }

    pub fn set_theme(&self, theme: String) -> Result<(), String> {
        let mut guard = self.data.lock();
        guard.theme = Some(theme);
        self.persist_locked(&guard)
    }
}
