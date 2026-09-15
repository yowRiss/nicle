# Claude Code Plugin for Nicle

Integrates Anthropic's [Claude Code](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/overview) CLI assistant into Nicle IDE with Bun runtime supervision.

## Features
- **Direct Terminal Integration**: Launches Claude Code inside Nicle's built-in PTY terminal with full interactive ANSI support.
- **Command Palette Actions**: Exposes Claude commands directly in Nicle (`Ctrl+Shift+P`):
  - `Claude Code: Open in Terminal` (`claude`)
  - `Claude Code: Review Git Diffs`
  - `Claude Code: Explain Workspace`
  - `Claude Code: Compact Conversation`
- **Bun-Native**: Uses Bun for package installation and fast runtime execution.

## Getting Started

### 1. Prerequisites
- **Bun** (v1.0.0 or later): `curl -fsSL https://bun.sh/install | bash`
- **Anthropic API Key**: `export ANTHROPIC_API_KEY="sk-..."`

### 2. Install Claude Code CLI
```bash
bun install -g @anthropic-ai/claude-code
```

### 3. Link into Nicle IDE
1. Open Nicle.
2. Press `Ctrl+Shift+X` (or click **Plugins** in the sidebar).
3. Click **Link Local Plugin**.
4. Select this directory containing `nicle-plugin.json`.
5. The plugin appears under **Installed** with a `[DEV]` badge.
6. Open the Command Palette (`Ctrl+Shift+P`) and type `claude` to run actions immediately!
