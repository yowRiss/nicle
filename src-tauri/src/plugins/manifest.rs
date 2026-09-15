use crate::error::{message, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PluginType {
    Service,
    Cli,
    Command,
}

impl Default for PluginType {
    fn default() -> Self {
        PluginType::Service
    }
}

impl PluginType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginType::Service => "service",
            PluginType::Cli => "cli",
            PluginType::Command => "command",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeType {
    Node,
    Bun,
    Python,
    Binary,
    System,
}

impl Default for RuntimeType {
    fn default() -> Self {
        RuntimeType::Node
    }
}

impl RuntimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::Node => "node",
            RuntimeType::Bun => "bun",
            RuntimeType::Python => "python",
            RuntimeType::Binary => "binary",
            RuntimeType::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSpec {
    #[serde(rename = "type")]
    pub runtime_type: RuntimeType,
    #[serde(default)]
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstallSpec {
    #[serde(default)]
    pub manager: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSpec {
    pub entry: String,
    #[serde(default = "default_service_port")]
    pub default_port: u16,
    #[serde(default)]
    pub readiness_path: Option<String>,
    #[serde(default)]
    pub dashboard_path: Option<String>,
    #[serde(default)]
    pub activity_path: Option<String>,
}

fn default_service_port() -> u16 {
    3000
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ActionType {
    Terminal,
    Run,
    Service,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::Terminal => "terminal",
            ActionType::Run => "run",
            ActionType::Service => "service",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub action_type: ActionType,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub plugin_type: PluginType,
    #[serde(default)]
    pub runtime: Option<RuntimeSpec>,
    #[serde(default)]
    pub install: Option<InstallSpec>,
    #[serde(default)]
    pub service: Option<ServiceSpec>,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

pub fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    if manifest.id.trim().is_empty() {
        return Err(message("Plugin manifest id cannot be empty"));
    }

    if !manifest
        .id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(message(format!(
            "Plugin id '{}' contains invalid characters. Use only letters, numbers, '-', '_', or '.'",
            manifest.id
        )));
    }

    if manifest.name.trim().is_empty() {
        return Err(message("Plugin manifest name cannot be empty"));
    }

    if manifest.version.trim().is_empty() {
        return Err(message("Plugin manifest version cannot be empty"));
    }

    if manifest.plugin_type == PluginType::Service && manifest.service.is_none() {
        return Err(message(
            "Service plugins must declare a 'service' configuration with an 'entry' point",
        ));
    }

    if let Some(ref s) = manifest.service {
        if let Some(ref rp) = s.readiness_path {
            if !rp.starts_with('/') {
                return Err(message(
                    "service.readinessPath must start with '/' (e.g. '/health')",
                ));
            }
        }
        if let Some(ref dp) = s.dashboard_path {
            if !dp.starts_with('/') {
                return Err(message(
                    "service.dashboardPath must start with '/' (e.g. '/')",
                ));
            }
        }
        if let Some(ref act) = s.activity_path {
            if !act.starts_with('/') {
                return Err(message(
                    "service.activityPath must start with '/' (e.g. '/activity')",
                ));
            }
        }
    }

    for cmd in &manifest.commands {
        if cmd.id.trim().is_empty() {
            return Err(message("Command id cannot be empty"));
        }
        if cmd.title.trim().is_empty() {
            return Err(message(format!(
                "Command title for '{}' cannot be empty",
                cmd.id
            )));
        }
        if cmd.command.trim().is_empty() {
            return Err(message(format!(
                "Command string for '{}' cannot be empty",
                cmd.id
            )));
        }
    }

    Ok(())
}

pub fn load_manifest_from_dir(dir: &Path) -> Result<PluginManifest> {
    let manifest_path = dir.join("nicle-plugin.json");
    if !manifest_path.exists() {
        return Err(message(format!(
            "Manifest file not found: {}",
            manifest_path.display()
        )));
    }

    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest: PluginManifest = serde_json::from_str(&content).map_err(|e| {
        message(format!(
            "Failed to parse nicle-plugin.json at {}: {e}",
            manifest_path.display()
        ))
    })?;

    validate_manifest(&manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest_service() {
        let json = r#"{
            "id": "test-service",
            "name": "Test Service",
            "version": "1.0.0",
            "description": "A companion service",
            "pluginType": "service",
            "runtime": { "type": "bun" },
            "service": {
                "entry": "server.ts",
                "defaultPort": 4000,
                "readinessPath": "/health",
                "dashboardPath": "/",
                "activityPath": "/activity"
            },
            "commands": [
                {
                    "id": "test.ping",
                    "title": "Ping Service",
                    "actionType": "terminal",
                    "command": "curl http://127.0.0.1:4000/health"
                }
            ]
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).expect("valid json");
        assert!(validate_manifest(&manifest).is_ok());
        assert_eq!(manifest.id, "test-service");
        assert_eq!(manifest.plugin_type, PluginType::Service);
        assert_eq!(
            manifest.runtime.as_ref().map(|r| &r.runtime_type),
            Some(&RuntimeType::Bun)
        );
        assert_eq!(
            manifest.service.as_ref().and_then(|s| s.activity_path.as_deref()),
            Some("/activity")
        );
        assert_eq!(manifest.commands.len(), 1);
        assert_eq!(manifest.commands[0].action_type, ActionType::Terminal);
    }

    #[test]
    fn test_valid_manifest_cli_claude_code() {
        let json = r#"{
            "id": "claude-code",
            "name": "Claude Code",
            "version": "0.2.29",
            "description": "Anthropic agentic assistant",
            "pluginType": "cli",
            "runtime": { "type": "bun", "minVersion": "1.0.0" },
            "install": {
                "manager": "bun",
                "package": "@anthropic-ai/claude-code"
            },
            "commands": [
                {
                    "id": "claude.open",
                    "title": "Claude Code: Open in Terminal",
                    "actionType": "terminal",
                    "command": "claude"
                },
                {
                    "id": "claude.review",
                    "title": "Claude Code: Review Diffs",
                    "actionType": "terminal",
                    "command": "claude 'review git diff'"
                }
            ]
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).expect("valid json");
        assert!(validate_manifest(&manifest).is_ok());
        assert_eq!(manifest.plugin_type, PluginType::Cli);
        assert_eq!(manifest.commands.len(), 2);
    }

    #[test]
    fn test_invalid_manifest_id() {
        let json = r#"{
            "id": "bad name with spaces!",
            "name": "Invalid",
            "version": "1.0.0",
            "description": "invalid"
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).expect("valid json");
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_service_without_spec_fails() {
        let json = r#"{
            "id": "service-missing",
            "name": "Missing",
            "version": "1.0.0",
            "description": "invalid",
            "pluginType": "service"
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).expect("valid json");
        assert!(validate_manifest(&manifest).is_err());
    }
}
