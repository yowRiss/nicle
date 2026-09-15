use crate::error::{message, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

use crate::plugins::manifest::{load_manifest_from_dir, PluginManifest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledRecord {
    pub id: String,
    pub version: String,
    pub install_path: String,
    pub installed_at: u64,
    pub port: u16,
    pub start_with_nicle: bool,
    #[serde(default)]
    pub is_dev: bool,
    #[serde(default)]
    pub manifest: Option<PluginManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPayload {
    pub port: u16,
    pub start_with_nicle: bool,
}

pub fn get_plugins_base_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    let base = app.path().app_data_dir().map_err(message)?.join("plugins");
    if !base.exists() {
        std::fs::create_dir_all(&base)?;
    }
    Ok(base)
}

pub fn get_apps_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    let dir = get_plugins_base_dir(app)?.join("apps");
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

pub fn get_staging_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    let dir = get_plugins_base_dir(app)?.join("staging");
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

pub fn get_app_install_dir(app: &tauri::AppHandle, app_id: &str) -> Result<PathBuf> {
    Ok(get_apps_dir(app)?.join(app_id))
}

pub fn load_installed(app: &tauri::AppHandle) -> Result<Vec<InstalledRecord>> {
    let file = get_plugins_base_dir(app)?.join("installed.json");
    if !file.exists() {
        return Ok(Vec::new());
    }
    let data = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let records: Vec<InstalledRecord> = serde_json::from_slice(&data).map_err(message)?;
    Ok(records)
}

pub fn save_installed(app: &tauri::AppHandle, records: &[InstalledRecord]) -> Result<()> {
    let base = get_plugins_base_dir(app)?;
    let target = base.join("installed.json");
    let temp = base.join(format!("installed.json.tmp.{}", std::process::id()));
    let json_bytes = serde_json::to_vec_pretty(records).map_err(message)?;
    std::fs::write(&temp, json_bytes)?;
    std::fs::rename(&temp, &target)?;
    Ok(())
}

pub fn get_installed_record(app: &tauri::AppHandle, app_id: &str) -> Result<Option<InstalledRecord>> {
    let list = load_installed(app)?;
    Ok(list.into_iter().find(|r| r.id == app_id))
}

pub fn upsert_installed_record(app: &tauri::AppHandle, record: InstalledRecord) -> Result<()> {
    let mut list = load_installed(app)?;
    if let Some(pos) = list.iter().position(|r| r.id == record.id) {
        list[pos] = record;
    } else {
        list.push(record);
    }
    save_installed(app, &list)
}

pub fn remove_installed_record(app: &tauri::AppHandle, app_id: &str) -> Result<()> {
    let mut list = load_installed(app)?;
    list.retain(|r| r.id != app_id);
    save_installed(app, &list)
}

pub fn update_app_settings(
    app: &tauri::AppHandle,
    app_id: &str,
    settings: AppSettingsPayload,
) -> Result<InstalledRecord> {
    if settings.port < 1024 {
        return Err(message("Port must be 1024 or higher"));
    }
    let mut list = load_installed(app)?;
    let pos = list
        .iter()
        .position(|r| r.id == app_id)
        .ok_or_else(|| message(format!("App '{app_id}' is not installed")))?;

    list[pos].port = settings.port;
    list[pos].start_with_nicle = settings.start_with_nicle;
    let updated = list[pos].clone();
    save_installed(app, &list)?;
    Ok(updated)
}

pub fn link_dev_plugin(app: &tauri::AppHandle, dir_path: &std::path::Path) -> Result<InstalledRecord> {
    let manifest = load_manifest_from_dir(dir_path)?;
    let port = manifest.service.as_ref().map(|s| s.default_port).unwrap_or(3000);

    let record = InstalledRecord {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        install_path: dir_path.to_string_lossy().to_string(),
        installed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        port,
        start_with_nicle: false,
        is_dev: true,
        manifest: Some(manifest),
    };

    upsert_installed_record(app, record.clone())?;
    Ok(record)
}

pub fn unlink_dev_plugin(app: &tauri::AppHandle, app_id: &str) -> Result<()> {
    let record = get_installed_record(app, app_id)?
        .ok_or_else(|| message(format!("Plugin '{app_id}' is not registered")))?;

    if !record.is_dev {
        return Err(message(format!("Plugin '{app_id}' is not a linked dev plugin. Use uninstall instead.")));
    }

    remove_installed_record(app, app_id)
}

pub fn get_installed_manifest(app: &tauri::AppHandle, app_id: &str) -> Result<Option<PluginManifest>> {
    let record = match get_installed_record(app, app_id)? {
        Some(r) => r,
        None => return Ok(None),
    };

    if let Some(m) = record.manifest {
        return Ok(Some(m));
    }

    let manifest_file = std::path::Path::new(&record.install_path).join("nicle-plugin.json");
    if manifest_file.exists() {
        if let Ok(m) = load_manifest_from_dir(std::path::Path::new(&record.install_path)) {
            return Ok(Some(m));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_record_roundtrip() {
        let record = InstalledRecord {
            id: "9router".to_string(),
            version: "0.5.69".to_string(),
            install_path: "/tmp/fake/apps/9router".to_string(),
            installed_at: 1726000000,
            port: 20128,
            start_with_nicle: false,
            is_dev: false,
            manifest: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let decoded: InstalledRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "9router");
        assert_eq!(decoded.version, "0.5.69");
        assert_eq!(decoded.port, 20128);
        assert!(!decoded.start_with_nicle);
    }
}
