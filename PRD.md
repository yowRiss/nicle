# Nicle MVP

## Goal
A fast, lightweight desktop editor for basic coding, using Rust, Tauri 2, Svelte/TypeScript, CodeMirror 6, and xterm.js. No Electron.

## Scope
- Open folders; lazy, paged file explorer; create, rename, delete files/folders.
- Multiple tabs, edit/save/save all, dirty markers, undo/redo, highlighting, indentation, brackets, wrapping, find/replace.
- Real workspace PTY terminal with resize, interrupt, clipboard, and scrollback.
- Run/stop/restart installed JavaScript, TypeScript, Python, Go, Rust, C, and C++ tools with streamed output.
- Loopback-only HTML preview with full refresh on HTML/CSS/JS changes.
- Quick Open, command palette, keyboard shortcuts, status bar, persistent basic settings and dark/light themes.
- Plugins: Managed local companion apps marketplace (per plugin.md). Discovery, isolated private installation, process lifecycle supervision (On/Off/Restart/Logs), verified loopback dashboard launcher, and clean uninstall. Initial adapter for 9Router.
- Built-in Git Source Control: async status detection, syntax-highlighted diff modal, stage/unstage/discard, commit, push, pull, branch switching/creation, repository initialization, and explorer file tree status decorations.

## Architecture
Rust owns filesystem, workspace search, Git command execution/status parsing, PTY/process lifecycle, runners, preview HTTP server, settings persistence, and plugin companion-app supervision/installation. Svelte owns UI, CodeMirror documents, xterm rendering, tabs, dialogs, preview, Plugins page/details, and Source Control panel/modals. Keep editor text out of reactive global state.

## Performance
Opening a workspace reads only its root. Expand directories on demand with bounded results. Search only on request, off the UI thread, skipping `.git`, `node_modules`, `target`, `dist`, `build`, `.cache`. Bound terminal/output history, diff text (max 1 MiB), and file sizes. Retain at most 1 MiB in-memory log buffer per plugin app. Target idle RAM below 150 MB and warm launch below 2 seconds; measure before claiming these targets.

## Exclusions
No in-editor JS extension injection or arbitrary third-party code execution in the editor UI thread, no LSP, IntelliSense, autocomplete server, debugger, in-editor AI code completion, SSH, Docker, collaboration, cloud sync, accounts, or placeholder controls.

## Acceptance
Launch the desktop app, open a project, browse and mutate files, edit/save multiple tabs, use an interactive terminal, run and stop code, preview HTML with automatic refresh, use both palettes and shortcuts, switch theme, and exit with all owned processes cleaned up. Build/type checks must pass; record actual runtime verification and limitations.
