# Discord Rich Presence Plugin (Bun)

Discord Rich Presence companion service for Nicle IDE powered by Bun or Node.js.

## Features

- **Zero dependencies**: Uses native sockets/pipes and HTTP server.
- **Auto-connecting**: Hooks into Discord IPC socket/pipe across Linux, macOS, and Windows.
- **Debounced Activity Updates**: Receives workspace, file, and cursor location from Nicle IDE.
- **Embedded Dashboard**: Live preview card and privacy toggles at `http://127.0.0.1:3070/`.

## Running in Dev

```bash
bun run server.js
```
