# Nicle

A small desktop code editor built with Rust, Tauri 2, Svelte 5, CodeMirror 6 and xterm.js. No Electron.

## Run

Install Node.js 22+, Rust, and the [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/). On Fedora, the development packages include `gtk3-devel` and `webkit2gtk4.1-devel`.

```sh
npm ci
npm run tauri dev
```

Create a standalone executable:

```sh
npm run tauri build
```

The executable is `src-tauri/target/release/nicle` (Windows: `nicle.exe`). Packaging installers is disabled; the executable uses the operating system's webview. `npm run dev` alone shows the UI in a browser; filesystem and process features require the desktop app.

## Use

Open Folder accepts a path or opens the native folder picker through Browse. Expand folders to load their contents. Select a folder before creating a file inside it; select the workspace name to create at the root. Each explorer row has an actions menu for rename and permanent deletion.

CodeMirror preserves undo history and selection per tab. A dot marks unsaved changes. Closing a dirty tab, changing folders, or closing Nicle offers Save / Discard / Cancel. Save All is in the command palette.

| Shortcut | Action |
|---|---|
| Ctrl / Command + S | Save |
| Ctrl / Command + Shift + S | Save all |
| Ctrl / Command + P | Quick Open |
| Ctrl / Command + Shift + P | Command palette |
| Ctrl / Command + backtick | Toggle terminal |
| Ctrl / Command + F or H | Find / replace |
| Ctrl / Command + W | Close tab |
| Ctrl / Command + Tab | Next tab |

Terminal runs your system shell in the workspace. Ctrl+C interrupts commands. Ctrl+Shift+C/V copies/pastes; normal platform paste also works. Hiding the panel preserves the shell; its trash action ends the process.

Run saves the current file before starting it. Output streams into the Output panel, which has Stop and Restart actions. Tools must already be installed:

| Files | Command |
|---|---|
| JS | `node file` |
| TS / TSX | installed `tsx`, or `npx --no-install tsx file` |
| Python | `python3 -u file` |
| Go | `go run file` |
| Rust | `cargo run` in the nearest Cargo project inside the workspace |
| C / C++ | `gcc` / `g++`, then a temporary executable |

HTML Preview serves the workspace at an ephemeral `127.0.0.1` port. Changes to the loaded HTML, CSS and JavaScript trigger a full refresh, including changes made outside Nicle. Files must be saved to appear in preview. The server stops when the preview closes or the workspace changes.

Settings persist locally: theme, font size, indentation and word wrap.

## Plugins & Extensions

Nicle supports managed companion services, CLI assistants (such as Claude Code powered by Bun), and Command Palette extensions via process isolation. Local plugin development is supported out of the box with zero compilation steps.

See [DEVELOPING_PLUGINS.md](DEVELOPING_PLUGINS.md) for the manifest specification, CLI scaffolder, and guide.

## Multi-Agent AI Harness

Nicle IDE includes a first-class **Multi-Agent AI Harness** designed to coordinate specialized task subagents across diverse LLM systems (**Google Antigravity / Gemini**, **Anthropic Claude Code**, and **OpenAI Codex**):

- **Agent A (Coder / Implementer)**: Writes minimal, idiomatic, high-performance code adhering to `PRD.md` and `rust-skills`.
- **Agent B (Reviewer / Auditor)**: Performs rigorous adversarial audits on diffs, checking for panics, `.unwrap()`, memory buffer bounds, and Svelte runes.
- **Agent C (Verifier / QA)**: Runs automated type checks (`npm run check`), plugin validation, and build suites.
- **Agent D (Orchestrator)**: Coordinates task planning, handoffs, and fix cycles.

### Agent Quickstart Commands
- **Run AI Reviewer Audit**: `npm run harness:review`
- **Run Full Verification Pipeline**: `npm run harness:verify`
- **Generate Agent Task Envelope**: `npm run harness -- dispatch --role coder --task "<description>"`
- **Harness Skill Reference**: [`.agents/skills/ai-harness/SKILL.md`](.agents/skills/ai-harness/SKILL.md)
- **Universal Agent Guide**: [`AGENTS.md`](AGENTS.md) | **Claude Guide**: [`CLAUDE.md`](CLAUDE.md)


## Deliberate limits

UTF-8 text files up to 8 MB; 200 entries per explorer page; Quick Open returns up to 100 matches and limits each traversal to 100,000 entries / 1.5 seconds. Search ignores `.git`, `node_modules`, `target`, `dist`, `build`, `.cache`. Output keeps the most recent 200,000 characters and terminal scrollback keeps 1,000 lines. Preview tracks up to 2,048 loaded source files. No whole-project scan at startup.

## Checks

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

See PRD.md for scope and WORK_LOG.md for verification evidence. Linux is the development/verification platform; Windows and macOS need native verification.
