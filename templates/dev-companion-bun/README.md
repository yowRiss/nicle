# Dev Companion Service (Bun) for Nicle

A template demonstrating a background companion service supervised by Nicle with:
- Bun runtime (`bun run server.ts`)
- Configurable port injection via `PORT` environment variable
- `/health` readiness probe for clean zero-downtime startup detection
- Web dashboard accessible via Nicle's **Open Dashboard** button
- Terminal commands registered in Nicle's Command Palette (`Ctrl+Shift+P`)

## Testing in Nicle:
1. Open Nicle.
2. Go to **Plugins** (`Ctrl+Shift+X`).
3. Click **Link Local Plugin** and select this directory.
4. Click **Start** to launch the Bun server.
5. Click **Open Dashboard** or **Logs** to observe live output.
