pub mod adapters;
pub mod catalog;
pub mod install;
pub mod manifest;
pub mod runtime;
pub mod storage;
pub mod supervisor;

use crate::error::{message, Result};
use catalog::{get_catalog, CatalogPlugin};
use runtime::{check_bun_runtime, check_node_runtime, BunRuntimeStatus, RuntimeStatus};
use storage::{
    get_installed_manifest, link_dev_plugin, load_installed, unlink_dev_plugin,
    update_app_settings, AppSettingsPayload, InstalledRecord,
};
pub use supervisor::{Phase, PluginRuntimeStatus, PluginSupervisor};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredPluginCommand {
    pub plugin_id: String,
    pub plugin_name: String,
    pub command_id: String,
    pub title: String,
    pub description: Option<String>,
    pub action_type: String,
    pub command: String,
    pub args: Vec<String>,
}

pub struct PluginState(pub PluginSupervisor);

impl Default for PluginState {
    fn default() -> Self {
        Self(PluginSupervisor::default())
    }
}

impl PluginState {
    pub fn stop_all(&self) {
        self.0.stop_all();
    }
}

#[tauri::command]
pub async fn plugins_get_catalog() -> Result<Vec<CatalogPlugin>> {
    Ok(get_catalog())
}

#[tauri::command]
pub async fn plugins_get_installed(app: AppHandle, state: State<'_, PluginState>) -> Result<Vec<InstalledRecord>> {
    let mut list = load_installed(&app)?;
    for item in &mut list {
        if item.manifest.is_none() {
            if let Ok(Some(m)) = get_installed_manifest(&app, &item.id) {
                item.manifest = Some(m);
            }
        }
        state.0.sync_installed_state(&item.id, true, item.port);
    }
    Ok(list)
}

#[tauri::command]
pub async fn plugins_check_runtime() -> Result<RuntimeStatus> {
    Ok(check_node_runtime())
}

#[tauri::command]
pub async fn plugins_get_status(app_id: String, state: State<'_, PluginState>) -> Result<PluginRuntimeStatus> {
    Ok(state.0.get_runtime_status(&app_id))
}

#[tauri::command]
pub async fn plugins_install(app_id: String, app: AppHandle, state: State<'_, PluginState>) -> Result<()> {
    install::install_plugin(&app, &app_id, &state.0).await
}

#[tauri::command]
pub async fn plugins_start(app_id: String, app: AppHandle, state: State<'_, PluginState>) -> Result<()> {
    state.0.start_app(&app, &app_id).await
}

#[tauri::command]
pub async fn plugins_stop(app_id: String, state: State<'_, PluginState>) -> Result<()> {
    state.0.stop_app(&app_id).await
}

#[tauri::command]
pub async fn plugins_restart(app_id: String, app: AppHandle, state: State<'_, PluginState>) -> Result<()> {
    state.0.restart_app(&app, &app_id).await
}

#[tauri::command]
pub async fn plugins_update(app_id: String, app: AppHandle, state: State<'_, PluginState>) -> Result<()> {
    install::update_plugin(&app, &app_id, &state.0).await
}

#[tauri::command]
pub async fn plugins_uninstall(
    app_id: String,
    delete_data: bool,
    app: AppHandle,
    state: State<'_, PluginState>,
) -> Result<()> {
    install::uninstall_plugin(&app, &app_id, delete_data, &state.0).await
}

#[tauri::command]
pub async fn plugins_save_settings(
    app_id: String,
    settings: AppSettingsPayload,
    app: AppHandle,
    state: State<'_, PluginState>,
) -> Result<InstalledRecord> {
    let updated = update_app_settings(&app, &app_id, settings)?;
    state.0.sync_installed_state(&app_id, true, updated.port);
    Ok(updated)
}

#[tauri::command]
pub async fn plugins_get_logs(app_id: String, state: State<'_, PluginState>) -> Result<String> {
    Ok(state.0.get_logs(&app_id))
}

#[tauri::command]
pub async fn plugins_clear_logs(app_id: String, state: State<'_, PluginState>) -> Result<()> {
    state.0.clear_logs(&app_id);
    Ok(())
}

#[tauri::command]
pub async fn plugins_open_dashboard(app_id: String, state: State<'_, PluginState>) -> Result<()> {
    let status = state.0.get_runtime_status(&app_id);
    let url = status.dashboard_url.ok_or_else(|| message("App is not running"))?;

    let opener_result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&url).spawn()
    } else if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(&url).spawn()
    };

    opener_result.map_err(|e| message(format!("Failed to open system browser: {e}")))?;
    Ok(())
}

#[tauri::command]
pub async fn plugins_check_bun_runtime() -> Result<BunRuntimeStatus> {
    Ok(check_bun_runtime())
}

#[tauri::command]
pub async fn plugins_link_dev(
    dir_path: String,
    app: AppHandle,
    state: State<'_, PluginState>,
) -> Result<InstalledRecord> {
    let p = Path::new(&dir_path);
    let record = link_dev_plugin(&app, p)?;
    state.0.sync_installed_state(&record.id, true, record.port);
    Ok(record)
}

#[tauri::command]
pub async fn plugins_unlink_dev(
    app_id: String,
    app: AppHandle,
    state: State<'_, PluginState>,
) -> Result<()> {
    if state.0.is_running(&app_id) {
        state.0.stop_app(&app_id).await?;
    }
    unlink_dev_plugin(&app, &app_id)?;
    state.0.finish_transition(&app_id, Phase::NotInstalled);
    Ok(())
}

#[tauri::command]
pub async fn plugins_get_commands(app: AppHandle) -> Result<Vec<RegisteredPluginCommand>> {
    let installed = load_installed(&app)?;
    let mut commands = Vec::new();

    for record in installed {
        if let Ok(Some(manifest)) = get_installed_manifest(&app, &record.id) {
            for cmd in manifest.commands {
                commands.push(RegisteredPluginCommand {
                    plugin_id: manifest.id.clone(),
                    plugin_name: manifest.name.clone(),
                    command_id: cmd.id,
                    title: cmd.title,
                    description: cmd.description,
                    action_type: cmd.action_type.as_str().to_string(),
                    command: cmd.command,
                    args: cmd.args,
                });
            }
        }
    }

    Ok(commands)
}

#[tauri::command]
pub async fn plugins_scaffold(target_dir: String, template: String) -> Result<String> {
    let target = Path::new(&target_dir);
    if !target.exists() {
        std::fs::create_dir_all(target)?;
    }

    if template == "claude-code-bun" {
        let manifest_json = r#"{
  "$schema": "https://nicle.dev/schema/plugin.json",
  "id": "claude-code",
  "name": "Claude Code",
  "version": "1.0.0",
  "description": "Anthropic agentic assistant for terminal and coding powered by Bun",
  "author": "Anthropic / Community",
  "homepage": "https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/overview",
  "pluginType": "cli",
  "runtime": {
    "type": "bun",
    "minVersion": "1.0.0"
  },
  "install": {
    "manager": "bun",
    "package": "@anthropic-ai/claude-code",
    "args": ["install", "-g", "@anthropic-ai/claude-code"]
  },
  "commands": [
    {
      "id": "claude.open",
      "title": "Claude Code: Open in Terminal",
      "description": "Start interactive Claude Code session in Nicle terminal",
      "actionType": "terminal",
      "command": "claude"
    },
    {
      "id": "claude.review",
      "title": "Claude Code: Review Git Diffs",
      "description": "Ask Claude Code to review workspace git diffs",
      "actionType": "terminal",
      "command": "claude 'review git diffs and suggest improvements'"
    },
    {
      "id": "claude.explain",
      "title": "Claude Code: Explain Workspace",
      "description": "Ask Claude Code to explain project architecture",
      "actionType": "terminal",
      "command": "claude 'explain the workspace architecture and key files'"
    }
  ]
}"#;
        std::fs::write(target.join("nicle-plugin.json"), manifest_json)?;

        let readme = r#"# Claude Code Plugin for Nicle

This plugin integrates Claude Code into Nicle using Bun.

## Commands Available in Nicle
- **Claude Code: Open in Terminal**: Opens interactive Claude session in the IDE terminal.
- **Claude Code: Review Git Diffs**: Runs an automated code review on your active workspace.
- **Claude Code: Explain Workspace**: Summarizes architecture and entry points.

## How to test in Nicle:
1. Ensure Bun is installed (`bun --version`).
2. Install Claude Code: `bun install -g @anthropic-ai/claude-code`.
3. In Nicle, open Plugins (`Ctrl+Shift+X`) -> click "Link Local Plugin" -> select this folder.
4. Open the Command Palette (`Ctrl+Shift+P`) and type `claude` to use your commands!
"#;
        std::fs::write(target.join("README.md"), readme)?;
    } else if template == "ai-harness-agent" {
        let manifest_json = r#"{
  "$schema": "https://nicle.dev/schema/plugin.json",
  "id": "ai-harness-agent",
  "name": "AI Harness Agent",
  "version": "1.0.0",
  "description": "Multi-agent task harness for Nicle IDE: coordinates Agent A (Coder) and Agent B (Reviewer)",
  "author": "Nicle Community",
  "homepage": "https://nicle.dev",
  "pluginType": "command",
  "runtime": {
    "type": "node",
    "minVersion": "18.0.0"
  },
  "commands": [
    {
      "id": "ai-harness.review",
      "title": "AI Harness: Run Reviewer Audit (Agent B)",
      "description": "Audits current git diff for Rust/Svelte safety, memory bounds, and regressions",
      "actionType": "terminal",
      "command": "npm run harness:review"
    },
    {
      "id": "ai-harness.verify",
      "title": "AI Harness: Run Full Verification Pipeline (Agent C)",
      "description": "Executes svelte-check, plugin schema validator, and review suite",
      "actionType": "terminal",
      "command": "npm run harness:verify"
    },
    {
      "id": "ai-harness.coder",
      "title": "AI Harness: Dispatch Coder Agent (Agent A)",
      "description": "Generates a standardized Agent A task envelope with PRD & rust-skills invariants",
      "actionType": "terminal",
      "command": "npm run harness -- dispatch --role coder"
    },
    {
      "id": "ai-harness.reviewer",
      "title": "AI Harness: Dispatch Reviewer Agent (Agent B)",
      "description": "Generates a standardized Agent B review envelope for auditing diffs",
      "actionType": "terminal",
      "command": "npm run harness -- dispatch --role reviewer"
    }
  ],
  "environment": {
    "FORCE_COLOR": "1"
  }
}"#;
        std::fs::write(target.join("nicle-plugin.json"), manifest_json)?;

        let readme = r#"# AI Harness Agent Plugin for Nicle IDE

Multi-agent task harness coordinating Agent A (Coder), Agent B (Reviewer), and Agent C (Verifier).

## Usage
Press `Ctrl+Shift+P` and type `AI Harness` to run agent dispatch or review commands directly in Nicle's terminal.
"#;
        std::fs::write(target.join("README.md"), readme)?;
    } else {
        let manifest_json = r#"{
  "$schema": "https://nicle.dev/schema/plugin.json",
  "id": "dev-companion",
  "name": "Dev Companion Service",
  "version": "0.1.0",
  "description": "Local companion service powered by Bun",
  "pluginType": "service",
  "runtime": {
    "type": "bun",
    "minVersion": "1.0.0"
  },
  "service": {
    "entry": "server.ts",
    "defaultPort": 3000,
    "readinessPath": "/health",
    "dashboardPath": "/"
  },
  "commands": [
    {
      "id": "companion.ping",
      "title": "Dev Companion: Ping Health Endpoint",
      "actionType": "terminal",
      "command": "curl http://127.0.0.1:3000/health"
    }
  ]
}"#;
        std::fs::write(target.join("nicle-plugin.json"), manifest_json)?;

        let server_ts = r#"// Minimal loopback companion server running on Bun
const port = Number(process.env.PORT) || 3000;
const hostname = "127.0.0.1";

console.log(`[COMPANION] Starting companion service on http://${hostname}:${port}`);

Bun.serve({
  port,
  hostname,
  fetch(req) {
    const url = new URL(req.url);
    if (url.pathname === "/health") {
      return new Response(JSON.stringify({ status: "healthy", timestamp: Date.now() }), {
        headers: { "Content-Type": "application/json" }
      });
    }
    return new Response(`
      <!DOCTYPE html>
      <html>
        <head><title>Dev Companion</title></head>
        <body style="background:#111;color:#eee;font-family:sans-serif;padding:2rem;">
          <h1 style="color:#22d3ee;">Dev Companion Dashboard</h1>
          <p>Running on 127.0.0.1:${port} supervised by Nicle.</p>
        </body>
      </html>
    `, {
      headers: { "Content-Type": "text/html" }
    });
  }
});
"#;
        std::fs::write(target.join("server.ts"), server_ts)?;
    }

    Ok(target.to_string_lossy().to_string())
}

