# AI Harness Plugin for Nicle IDE

Integrates Nicle's **Multi-Agent AI Harness** directly into the Nicle IDE workbench, exposing commands in the Command Palette (`Ctrl+Shift+P`) to dispatch specialized tasks to **Agent A (Coder)**, **Agent B (Reviewer)**, and **Agent C (Verifier)**.

## Features

- **Command Palette Integration**:
  - `AI Harness: Run Reviewer Audit (Agent B)`: Runs an adversarial static review on workspace diffs, checking for panics, `.unwrap()`, memory buffer bounds, and Svelte runes.
  - `AI Harness: Run Full Verification Pipeline (Agent C)`: Runs complete type checking (`svelte-check`), plugin schema tests, and review checks.
  - `AI Harness: Dispatch Coder Agent (Agent A)`: Prints a scoped prompt envelope for code implementation.
  - `AI Harness: Dispatch Reviewer Agent (Agent B)`: Prints a scoped prompt envelope for adversarial review.
- **Cross-Model Compatibility**: Designed to work with **Google Antigravity / Gemini**, **Anthropic Claude Code**, and **OpenAI Codex**.

## How to Link into Nicle IDE

1. Open Nicle.
2. Press `Ctrl+Shift+X` (or click **Plugins** in the sidebar).
3. Click **Link Local Plugin**.
4. Select this directory (`examples/sample-ai-harness`).
5. Open the Command Palette (`Ctrl+Shift+P`) and type `AI Harness` to run any agent command!
