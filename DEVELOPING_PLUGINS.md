# Developing Plugins for Nicle

This guide provides the complete developer reference and workflow for authoring, testing, and distributing plugins for Nicle IDE.

---

## 1. Plugin Architecture Overview

Nicle plugins allow developers to extend the IDE without recompiling Nicle or compromising the stability of the core editor.

### Core Tenets
1. **Process Isolation**: Plugins run as managed external processes or inside Nicle's built-in PTY terminal. They never execute arbitrary, unverified JavaScript in the editor's UI thread.
2. **First-Class Runtime Support**: Native support for **Bun**, **Node.js**, **Python**, and compiled **Binaries**.
3. **Local Dev Linking**: Point Nicle to any local directory containing a `nicle-plugin.json` manifest. No publishing or packaging step is needed while iterating.
4. **Command Palette Integration**: Plugins can expose commands directly to Nicle's Command Palette (`Ctrl+Shift+P`) and trigger interactive terminal sessions or background tasks.

---

## 2. Plugin Types

| Type | Manifest `pluginType` | Purpose | Key Features |
|---|---|---|---|
| **CLI Assistant** | `cli` | Interactive terminal tools (e.g. Claude Code) | Launches in Nicle PTY terminal, ANSI color support, quick Command Palette actions |
| **Companion Service** | `service` | Background daemons & microservices | Supervised lifecycle (Start/Stop), loopback port binding, `/health` readiness probes, web dashboard |
| **Command Suite** | `command` | Action-oriented tool runner | Registers utility scripts or commands in the Command Palette |

---

## 3. The `nicle-plugin.json` Manifest Specification

Every plugin directory must have a `nicle-plugin.json` file in its root.

```json
{
  "$schema": "https://nicle.dev/schema/plugin.json",
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "Brief description of what the plugin does",
  "author": "Your Name",
  "homepage": "https://github.com/your-username/my-plugin",
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
  "service": {
    "entry": "server.ts",
    "defaultPort": 3050,
    "readinessPath": "/health",
    "dashboardPath": "/"
  },
  "commands": [
    {
      "id": "myplugin.run",
      "title": "My Plugin: Run Action",
      "description": "Executes command in the terminal",
      "actionType": "terminal",
      "command": "echo 'Hello from Nicle!'"
    }
  ],
  "environment": {
    "FORCE_COLOR": "1"
  }
}
```

### Schema Properties Breakdown

#### Top-level Metadata
- **`id`** (`string`, required): Unique slug (letters, numbers, `-`, `_`, `.`). Example: `"claude-code"`.
- **`name`** (`string`, required): User-facing display title.
- **`version`** (`string`, required): Semantic version string (`x.y.z`).
- **`description`** (`string`, optional): Summary of the plugin's capabilities.
- **`pluginType`** (`"service" | "cli" | "command"`, required): Type of plugin.
- **`author`** (`string`, optional): Author or organization.
- **`homepage`** (`string`, optional): Documentation or project URL.

#### `runtime`
- **`type`** (`"bun" | "node" | "python" | "binary" | "system"`, required): Runtime used to execute the plugin.
- **`minVersion`** (`string`, optional): Minimum required runtime version (e.g. `"1.0.0"`).

#### `service` (Required for `pluginType: "service"`)
- **`entry`** (`string`, required): Main entry file relative to plugin folder (e.g. `"server.ts"` or `"main.py"`).
- **`defaultPort`** (`number`, optional): Default TCP loopback port (defaults to `3000`).
- **`readinessPath`** (`string`, optional): HTTP GET endpoint Nicle polls on `127.0.0.1:<port>` to verify startup (e.g. `"/health"`).
- **`dashboardPath`** (`string`, optional): Relative URL path opened in the browser when clicking **Open Dashboard** (e.g. `"/"`).

#### `commands` (Array of IDE Actions)
- **`id`** (`string`, required): Unique command ID (e.g. `"claude.open"`).
- **`title`** (`string`, required): Title displayed in the Command Palette (`Ctrl+Shift+P`).
- **`description`** (`string`, optional): Brief hint shown below the command title.
- **`actionType`** (`"terminal" | "run" | "service"`, required):
  - `"terminal"`: Sends the command into Nicle's active PTY shell and switches to the Terminal panel.
  - `"run"`: Executes the command in a background child process.
  - `"service"`: Triggers a service lifecycle transition.
- **`command`** (`string`, required): Shell command line to execute.
- **`args`** (`string[]`, optional): Command line arguments.

---

## 4. Quick Start: Scaffolding a Plugin

Nicle supports two complementary workflows to dev-create a plugin: **In-IDE Visual Scaffolding** and **Terminal CLI Scaffolding**.

### Workflow A: In-IDE Visual Wizard (Recommended)

1. Launch Nicle IDE (`nicle`).
2. Open Plugins (`Ctrl+Shift+X` or click the puzzle icon in the activity bar).
3. Click the cyan **New Plugin** button in the header bar *(or press `Ctrl+Shift+P` and choose `Plugins: Create New Plugin…`)*.
4. Fill in the modal options:
   - **Plugin Architecture**: Choose between **CLI Assistant** (terminal agent) or **Companion Service** (background daemon).
   - **Plugin Display Name**: e.g. `My Claude Helper` or `Workspace Companion`.
   - **Plugin Identifier**: Unique slug, e.g. `my-claude-helper`.
   - **Destination Directory**: Choose a project subfolder or click **Browse** to pick any folder.
   - **Auto-link**: Leave checked to link immediately for live local development.
5. Click **Create Plugin**.
6. Your new plugin immediately appears in the **Installed** tab with a `[DEV]` badge, and its actions are registered in the Command Palette (`Ctrl+Shift+P`).

---

### Workflow B: Terminal Scaffolder & Interactive CLI

Run the scaffolder from your terminal:

```bash
# 1. Interactive wizard (prompts for name, template, author, directory, auto-link)
npm run create-plugin

# 2. Or pass flags directly:
npm run create-plugin -- my-assistant --template claude-code-bun --link

# 3. Create a background companion service:
npm run create-plugin -- my-daemon --template dev-companion-bun --dir ./plugins/my-daemon --link

# 4. Create a workspace command suite:
npm run create-plugin -- my-suite --template command-suite-bun --link
```

#### Available Templates
| Template Name | Plugin Type | Description |
|---|---|---|
| **`claude-code-bun`** | `cli` | Interactive Claude Code / terminal assistant running inside Nicle's PTY with Bun. |
| **`dev-companion-bun`** | `service` | Background Bun HTTP microservice with loopback port binding, `/health` readiness check, and browser dashboard. |
| **`command-suite-bun`** | `command` | Action-oriented script runner registering developer shortcuts in Nicle's Command Palette. |

#### CLI Options
- `--template, -t <name>`: Template name (`claude-code-bun`, `dev-companion-bun`, `command-suite-bun`).
- `--dir, -d <path>`: Destination directory (defaults to `./<slug>`).
- `--link, -l`: Automatically register the plugin in Nicle's installed registry (`isDev: true`).
- `--desc <text>`: Short plugin summary description.
- `--author, -a <name>`: Author or organization name.
- `--interactive, -i`: Force interactive question prompts.

---

## 5. Development, Validation & Testing Workflow

### 1. Live Dev Linking & Hot Iteration
When linked, Nicle reads your plugin files directly from disk without packaging or bundling:
- For **CLI & Command plugins**: Changes to `nicle-plugin.json` or your scripts take effect immediately the next time the command is triggered from `Ctrl+Shift+P`.
- For **Service plugins**: In the Plugins panel (`Ctrl+Shift+X`), toggle **Off** and then **On** (or click **Restart**) to load code updates in your service.

### 2. Validating Your Plugin Manifest
Run Nicle's built-in validator to catch manifest schema errors, missing entry files, or invalid command definitions:

```bash
npm run validate-plugin -- ./path/to/plugin
```

The validator checks:
- `nicle-plugin.json` presence and valid JSON syntax
- Slug format (`id`) matching `^[a-zA-Z0-9._-]+$`
- Required `name`, `version`, and `pluginType`
- Valid runtime (`bun`, `node`, `python`, `binary`, `system`)
- For `service` plugins: ensures `service.entry` file exists on disk and `defaultPort` is non-privileged (>= 1024)
- Command structure and action types (`terminal`, `run`, `service`)

### 3. Running Automated Plugin Workflow Tests
To verify all templates and scaffolding mechanisms across the repository:

```bash
npm run test:plugins
```

### 4. Unlinking
- From the IDE: In `Ctrl+Shift+X` -> select the plugin -> click **Unlink from IDE**.
- Or remove the entry from `~/.local/share/dev.nicle.editor/plugins/installed.json`.

---

## 6. Case Study: Integrating Claude Code with Bun

The Claude Code plugin connects Anthropic's interactive agentic CLI to Nicle's terminal.

### Manifest Configuration:
```json
{
  "$schema": "https://nicle.dev/schema/plugin.json",
  "id": "claude-code",
  "name": "Claude Code",
  "version": "1.0.0",
  "description": "Anthropic Claude Code CLI assistant integrated into Nicle IDE with Bun runtime",
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
      "description": "Ask Claude Code to review workspace git diffs and report findings",
      "actionType": "terminal",
      "command": "claude 'review git diffs and suggest improvements'"
    },
    {
      "id": "claude.explain",
      "title": "Claude Code: Explain Workspace",
      "description": "Ask Claude Code to explain project architecture and entry points",
      "actionType": "terminal",
      "command": "claude 'explain the workspace architecture and key files'"
    }
  ]
}
```

### Usage:
1. Ensure Bun is installed: `bun --version`.
2. Install Claude Code CLI: `bun install -g @anthropic-ai/claude-code`.
3. Set your API key: `export ANTHROPIC_API_KEY="sk-..."`.
4. Link the folder in Nicle.
5. Hit `Ctrl+Shift+P` -> `Claude Code: Open in Terminal`.

---

## 7. Case Study: Authoring a Bun Companion Service

A background companion service can run an API server, documentation viewer, or local inference engine.

### `server.ts`:
```typescript
const port = Number(process.env.PORT) || 3050;
const hostname = "127.0.0.1";

console.log(`[COMPANION] Starting service on http://${hostname}:${port}`);

Bun.serve({
  port,
  hostname,
  fetch(req) {
    const url = new URL(req.url);

    // Health probe queried by Nicle supervisor
    if (url.pathname === "/health") {
      return new Response(JSON.stringify({ status: "healthy", timestamp: Date.now() }), {
        headers: { "Content-Type": "application/json" }
      });
    }

    // Web Dashboard
    return new Response(`
      <!DOCTYPE html>
      <html>
        <head><title>Companion Dashboard</title></head>
        <body style="background:#0f1115;color:#e6edf3;font-family:sans-serif;padding:2rem;">
          <h1 style="color:#22d3ee;">Companion Service</h1>
          <p>Supervised by Nicle on 127.0.0.1:${port}</p>
        </body>
      </html>
    `, {
      headers: { "Content-Type": "text/html" }
    });
  }
});
```

### Important Runtime Rules:
1. **Always read `process.env.PORT`**: Nicle dynamically injects the assigned loopback port.
2. **Loopback only (`127.0.0.1`)**: Never bind to `0.0.0.0` or expose public network interfaces.
3. **Graceful shutdown**: Listen for `SIGTERM` / `SIGINT` to flush state and cleanly exit when the user toggles Off in Nicle.
