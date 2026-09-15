# Discord Rich Presence Plugin for Nicle IDE

Companion service plugin that displays your active workspace, current file, line count, and elapsed coding time on Discord via native local IPC sockets and named pipes.

## Features

- **Zero npm dependencies**: Uses only native Node.js / Bun `net` and `http` modules.
- **Native Discord IPC**: Automatically locates and connects to Discord via Unix sockets (`/run/user/<uid>/discord-ipc-0`, `$XDG_RUNTIME_DIR/discord-ipc-0`, `/tmp/discord-ipc-0`) or Windows named pipes (`\\\\?\\pipe\\discord-ipc-0`).
- **Live Activity Updates**: Debounced activity synchronization with Nicle IDE editor events.
- **Rich Status**: Displays active workspace name, file being edited, cursor line/col, language icon, and session timer.
- **Live Dashboard**: Embedded graphite-themed web dashboard at `http://127.0.0.1:3070/` featuring real-time preview and privacy controls.
- **Privacy Controls**: Easily hide workspace or file names, or toggle idle status anytime.

## Quick Start

1. Start Nicle IDE.
2. Open Plugins (`Ctrl+Shift+X`).
3. Under Discover or Installed, activate **Discord Rich Presence**.
4. Open the live dashboard at `http://127.0.0.1:3070/` or use the Command Palette:
   - `Discord Presence: Reconnect`
   - `Discord Presence: Open Dashboard`
   - `Discord Presence: Toggle Idle Status`
