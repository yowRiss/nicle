use crate::error::{message, Result};
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: Theme,
    pub font_size: u8,
    pub tab_size: u8,
    pub word_wrap: bool,
    #[serde(default)]
    pub auto_save: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            font_size: 13,
            tab_size: 2,
            word_wrap: false,
            auto_save: false,
        }
    }
}

#[tauri::command]
pub async fn load_settings(app: tauri::AppHandle) -> Result<Settings> {
    let file = app.path().app_config_dir().map_err(message)?.join("settings.json");
    match tokio::fs::read(file).await {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(message),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn save_settings(settings: Settings, app: tauri::AppHandle) -> Result<()> {
    if !(10..=24).contains(&settings.font_size) || ![2, 4, 8].contains(&settings.tab_size) {
        return Err(message("Invalid editor settings"));
    }
    let dir = app.path().app_config_dir().map_err(message)?;
    tokio::fs::create_dir_all(&dir).await?;
    let data = serde_json::to_vec_pretty(&settings).map_err(message)?;
    tokio::fs::write(dir.join("settings.json"), data).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_auto_save_defaults_to_false_when_missing() {
        let json = r#"{"theme":"dark","fontSize":14,"tabSize":2,"wordWrap":true}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(!s.auto_save);
        assert!(s.word_wrap);
    }

    #[test]
    fn test_settings_auto_save_roundtrip() {
        let s = Settings {
            theme: Theme::Light,
            font_size: 16,
            tab_size: 4,
            word_wrap: true,
            auto_save: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, s);
    }
}
