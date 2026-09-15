#!/usr/bin/env node
/**
 * Nicle IDE - Discord Rich Presence Companion Plugin
 * Zero-dependency native Node.js / Bun companion service.
 * Connects to Discord via local IPC socket/named pipe and receives
 * editor activity from Nicle IDE.
 */

import http from "node:http";
import net from "node:net";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const PORT = Number(process.env.PORT) || 3070;
const HOST = "127.0.0.1";
const startTime = Date.now();
const sessionStartSeconds = Math.floor(startTime / 1000);

// Discord RPC Opcodes
const OPCODES = {
  HANDSHAKE: 0,
  FRAME: 1,
  CLOSE: 2,
  PING: 3,
  PONG: 4,
};

// Known language icon / asset map
const LANGUAGE_ASSETS = {
  ts: { icon: "typescript", name: "TypeScript" },
  tsx: { icon: "typescript", name: "TypeScript React" },
  js: { icon: "javascript", name: "JavaScript" },
  jsx: { icon: "javascript", name: "JavaScript React" },
  mjs: { icon: "javascript", name: "JavaScript" },
  cjs: { icon: "javascript", name: "JavaScript" },
  rs: { icon: "rust", name: "Rust" },
  py: { icon: "python", name: "Python" },
  go: { icon: "go", name: "Go" },
  c: { icon: "c", name: "C" },
  cpp: { icon: "cpp", name: "C++" },
  cc: { icon: "cpp", name: "C++" },
  cxx: { icon: "cpp", name: "C++" },
  h: { icon: "c", name: "C Header" },
  hpp: { icon: "cpp", name: "C++ Header" },
  html: { icon: "html", name: "HTML" },
  htm: { icon: "html", name: "HTML" },
  css: { icon: "css", name: "CSS" },
  scss: { icon: "sass", name: "SCSS" },
  sass: { icon: "sass", name: "Sass" },
  less: { icon: "less", name: "Less" },
  json: { icon: "json", name: "JSON" },
  md: { icon: "markdown", name: "Markdown" },
  markdown: { icon: "markdown", name: "Markdown" },
  svelte: { icon: "svelte", name: "Svelte" },
  vue: { icon: "vue", name: "Vue" },
  toml: { icon: "toml", name: "TOML" },
  yaml: { icon: "yaml", name: "YAML" },
  yml: { icon: "yaml", name: "YAML" },
  xml: { icon: "xml", name: "XML" },
  sh: { icon: "shell", name: "Shell Script" },
  bash: { icon: "shell", name: "Bash" },
  zsh: { icon: "shell", name: "Zsh" },
  sql: { icon: "database", name: "SQL" },
  dockerfile: { icon: "docker", name: "Docker" },
};

// Default Discord Application Client ID for Nicle IDE
const DEFAULT_CLIENT_ID = process.env.DISCORD_CLIENT_ID || "1347000000000000000";

// Plugin state & configurable settings
const settings = {
  clientId: DEFAULT_CLIENT_ID,
  hideWorkspace: false,
  hideFile: false,
  idle: false,
  largeImageKey: "nicle_logo",
  largeImageText: "Nicle IDE",
};

let currentActivity = {
  workspace: null,
  file: null,
  timestamp: Date.now(),
};

function encodePacket(opcode, payload) {
  const json = JSON.stringify(payload);
  const dataBuf = Buffer.from(json, "utf-8");
  const headerBuf = Buffer.alloc(8);
  headerBuf.writeInt32LE(opcode, 0);
  headerBuf.writeInt32LE(dataBuf.length, 4);
  return Buffer.concat([headerBuf, dataBuf]);
}

function getSocketCandidates() {
  const list = [];
  if (process.platform === "win32") {
    for (let i = 0; i < 10; i++) {
      list.push(`\\\\?\\pipe\\discord-ipc-${i}`);
      list.push(`\\\\.\\pipe\\discord-ipc-${i}`);
    }
  } else {
    const uid = typeof process.getuid === "function" ? process.getuid() : 1000;
    const xdg = process.env.XDG_RUNTIME_DIR;
    const tmp = process.env.TMPDIR || process.env.TMP || process.env.TEMP || "/tmp";

    for (let i = 0; i < 10; i++) {
      if (xdg) {
        list.push(path.join(xdg, `discord-ipc-${i}`));
      }
      list.push(`/run/user/${uid}/discord-ipc-${i}`);
      list.push(path.join(tmp, `discord-ipc-${i}`));
    }
  }
  return [...new Set(list)];
}

class DiscordRpcClient {
  constructor() {
    this.socket = null;
    this.connected = false;
    this.connecting = false;
    this.currentSocketPath = null;
    this.reconnectTimer = null;
    this.reconnectDelay = 5000;
    this.buffer = Buffer.alloc(0);
  }

  connect() {
    if (this.connecting || this.connected) return;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    this.connecting = true;
    const candidates = getSocketCandidates();
    this._tryConnectNext(candidates, 0);
  }

  _tryConnectNext(candidates, index) {
    if (index >= candidates.length) {
      this.connecting = false;
      this._scheduleReconnect();
      return;
    }

    const socketPath = candidates[index];
    const sock = net.createConnection(socketPath);

    const onError = () => {
      sock.destroy();
      this._tryConnectNext(candidates, index + 1);
    };

    sock.once("error", onError);

    sock.once("connect", () => {
      sock.removeListener("error", onError);
      this.socket = sock;
      this.currentSocketPath = socketPath;
      this.connected = true;
      this.connecting = false;
      this.reconnectDelay = 5000;
      this.buffer = Buffer.alloc(0);

      console.log(`[DISCORD] Connected to Discord IPC socket: ${socketPath}`);

      sock.on("data", (chunk) => this._onData(chunk));
      sock.on("close", () => this._onClose());
      sock.on("error", (err) => this._onError(err));

      this._sendHandshake();
    });
  }

  _sendHandshake() {
    if (!this.socket || !this.connected) return;
    const packet = encodePacket(OPCODES.HANDSHAKE, {
      v: 1,
      client_id: settings.clientId,
    });
    this.socket.write(packet);
  }

  _onData(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);

    while (this.buffer.length >= 8) {
      const opcode = this.buffer.readInt32LE(0);
      const length = this.buffer.readInt32LE(4);

      if (this.buffer.length < 8 + length) {
        break; // Wait for full packet payload
      }

      const payloadBuf = this.buffer.subarray(8, 8 + length);
      this.buffer = this.buffer.subarray(8 + length);

      try {
        const payload = JSON.parse(payloadBuf.toString("utf-8"));
        this._handlePacket(opcode, payload);
      } catch (err) {
        console.warn("[DISCORD] Failed to parse IPC frame payload:", err);
      }
    }
  }

  _handlePacket(opcode, payload) {
    if (opcode === OPCODES.FRAME) {
      if (payload.cmd === "DISPATCH" && payload.evt === "READY") {
        console.log(`[DISCORD] IPC Handshake acknowledged. Authenticated as: ${payload.data?.user?.username ?? "Nicle User"}`);
        this.updatePresence();
      }
    } else if (opcode === OPCODES.PING) {
      if (this.socket && this.connected) {
        this.socket.write(encodePacket(OPCODES.PONG, payload));
      }
    } else if (opcode === OPCODES.CLOSE) {
      console.warn("[DISCORD] IPC socket closed by remote Discord client");
      this._disconnect();
    }
  }

  _onClose() {
    this._disconnect();
  }

  _onError(err) {
    console.warn(`[DISCORD] Socket error: ${err.message}`);
    this._disconnect();
  }

  _disconnect() {
    if (this.socket) {
      try {
        this.socket.destroy();
      } catch {}
      this.socket = null;
    }
    const wasConnected = this.connected;
    this.connected = false;
    this.connecting = false;
    this.currentSocketPath = null;
    this.buffer = Buffer.alloc(0);

    if (wasConnected) {
      console.log("[DISCORD] Disconnected from Discord IPC. Will attempt to reconnect...");
    }
    this._scheduleReconnect();
  }

  _scheduleReconnect() {
    if (this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.reconnectDelay = Math.min(this.reconnectDelay + 5000, 30000);
      this.connect();
    }, this.reconnectDelay);
  }

  buildActivity() {
    if (settings.idle) {
      return {
        details: "Idling in Nicle IDE",
        state: settings.hideWorkspace || !currentActivity.workspace
          ? undefined
          : `Workspace: ${currentActivity.workspace.name}`,
        timestamps: { start: sessionStartSeconds },
        assets: {
          large_image: settings.largeImageKey,
          large_text: settings.largeImageText,
        },
      };
    }

    if (!currentActivity.file) {
      return {
        details: "Exploring Codebase",
        state: settings.hideWorkspace || !currentActivity.workspace
          ? "No Workspace Open"
          : `Workspace: ${currentActivity.workspace.name}`,
        timestamps: { start: sessionStartSeconds },
        assets: {
          large_image: settings.largeImageKey,
          large_text: settings.largeImageText,
        },
      };
    }

    const file = currentActivity.file;
    const ext = (file.extension || "").toLowerCase();
    const lang = LANGUAGE_ASSETS[ext] || {
      icon: "code",
      name: ext ? ext.toUpperCase() : "Plain Text",
    };

    const details = settings.hideFile ? "Editing Code" : `Editing ${file.name}`;

    let state = undefined;
    const workspacePart = settings.hideWorkspace || !currentActivity.workspace
      ? ""
      : `Workspace: ${currentActivity.workspace.name}`;

    let cursorPart = "";
    if (file.line && file.column) {
      cursorPart = `(Ln ${file.line}, Col ${file.column})`;
    } else if (file.line) {
      cursorPart = `(Ln ${file.line})`;
    }

    if (workspacePart && cursorPart) {
      state = `${workspacePart} ${cursorPart}`;
    } else if (workspacePart) {
      state = workspacePart;
    } else if (cursorPart) {
      state = cursorPart;
    }

    return {
      details,
      state,
      timestamps: { start: sessionStartSeconds },
      assets: {
        large_image: settings.largeImageKey,
        large_text: settings.largeImageText,
        small_image: lang.icon,
        small_text: `Editing ${lang.name}`,
      },
    };
  }

  updatePresence() {
    if (!this.socket || !this.connected) return;

    const activity = this.buildActivity();
    const packet = encodePacket(OPCODES.FRAME, {
      cmd: "SET_ACTIVITY",
      args: {
        pid: process.pid,
        activity,
      },
      nonce: crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2),
    });

    try {
      this.socket.write(packet);
    } catch (err) {
      console.warn("[DISCORD] Failed to write activity packet:", err);
    }
  }

  destroy() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.socket) {
      try {
        this.socket.destroy();
      } catch {}
      this.socket = null;
    }
    this.connected = false;
  }
}

const rpc = new DiscordRpcClient();
rpc.connect();

// HTML Dashboard content conforming to Nicle design.md
function renderDashboardHtml() {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Discord Rich Presence - Nicle IDE</title>
  <style>
    :root {
      --bg-canvas: #0d0f12;
      --bg-surface: #14171d;
      --bg-card: #1a1e26;
      --bg-input: #101216;
      --border-subtle: #242933;
      --border-focus: #06b6d4;
      --text-primary: #f0f6fc;
      --text-muted: #8b949e;
      --text-cyan: #22d3ee;
      --accent-cyan: #06b6d4;
      --accent-cyan-hover: #0891b2;
      --accent-green: #22c55e;
      --accent-amber: #f59e0b;
      --accent-red: #ef4444;
      --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      background-color: var(--bg-canvas);
      color: var(--text-primary);
      font-family: var(--font-sans);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 2.5rem 1rem;
    }

    .container {
      width: 100%;
      max-width: 680px;
      display: flex;
      flex-direction: column;
      gap: 1.5rem;
    }

    .header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      border-bottom: 1px solid var(--border-subtle);
      padding-bottom: 1rem;
    }

    .title-group {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .title-icon {
      width: 32px;
      height: 32px;
      fill: var(--text-cyan);
    }

    h1 {
      font-size: 1.35rem;
      font-weight: 600;
      color: var(--text-primary);
      letter-spacing: -0.02em;
    }

    .badge {
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      font-size: 0.75rem;
      font-weight: 600;
      padding: 0.25rem 0.65rem;
      border-radius: 9999px;
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }

    .badge.connected {
      background: rgba(34, 197, 94, 0.15);
      color: var(--accent-green);
      border: 1px solid rgba(34, 197, 94, 0.3);
    }

    .badge.disconnected {
      background: rgba(245, 158, 11, 0.15);
      color: var(--accent-amber);
      border: 1px solid rgba(245, 158, 11, 0.3);
    }

    .status-dot {
      width: 7px;
      height: 7px;
      border-radius: 50%;
      background: currentColor;
    }

    .card {
      background: var(--bg-surface);
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      padding: 1.25rem;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    }

    .card-title {
      font-size: 0.85rem;
      font-weight: 600;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.05em;
      margin-bottom: 1rem;
    }

    /* Discord Presence Preview Box */
    .discord-preview {
      background: #2b2d31;
      border-radius: 8px;
      padding: 1rem;
      display: flex;
      gap: 1rem;
      position: relative;
      font-family: "gg sans", "Noto Sans", var(--font-sans);
    }

    .discord-avatar-wrap {
      position: relative;
      width: 64px;
      height: 64px;
      flex-shrink: 0;
    }

    .discord-large-img {
      width: 64px;
      height: 64px;
      border-radius: 8px;
      background: #1e1f22;
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-cyan);
      font-weight: 700;
      font-size: 1.2rem;
      border: 1px solid rgba(255,255,255,0.06);
    }

    .discord-small-img {
      position: absolute;
      bottom: -4px;
      right: -4px;
      width: 24px;
      height: 24px;
      border-radius: 50%;
      background: #111214;
      border: 2px solid #2b2d31;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 0.65rem;
      font-weight: bold;
      color: #38bdf8;
    }

    .discord-content {
      display: flex;
      flex-direction: column;
      justify-content: center;
      gap: 0.2rem;
      overflow: hidden;
    }

    .discord-game-title {
      font-size: 0.9rem;
      font-weight: 700;
      color: #ffffff;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .discord-details {
      font-size: 0.85rem;
      color: #dbdee1;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .discord-state {
      font-size: 0.85rem;
      color: #949ba4;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .discord-time {
      font-size: 0.8rem;
      color: #949ba4;
      font-variant-numeric: tabular-nums;
    }

    /* Controls form */
    .controls-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 1rem;
      margin-top: 0.5rem;
    }

    .toggle-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.65rem 0.85rem;
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: 6px;
    }

    .toggle-label {
      font-size: 0.85rem;
      color: var(--text-primary);
    }

    .toggle-switch {
      position: relative;
      display: inline-block;
      width: 38px;
      height: 22px;
    }

    .toggle-switch input {
      opacity: 0;
      width: 0;
      height: 0;
    }

    .slider {
      position: absolute;
      cursor: pointer;
      top: 0; left: 0; right: 0; bottom: 0;
      background-color: #2e3440;
      transition: .2s;
      border-radius: 22px;
    }

    .slider:before {
      position: absolute;
      content: "";
      height: 16px;
      width: 16px;
      left: 3px;
      bottom: 3px;
      background-color: white;
      transition: .2s;
      border-radius: 50%;
    }

    input:checked + .slider {
      background-color: var(--accent-cyan);
    }

    input:checked + .slider:before {
      transform: translateX(16px);
    }

    .actions-bar {
      display: flex;
      justify-content: flex-end;
      gap: 0.75rem;
      margin-top: 1rem;
    }

    button {
      background: var(--accent-cyan);
      color: #000000;
      font-weight: 600;
      font-size: 0.85rem;
      border: none;
      padding: 0.5rem 1rem;
      border-radius: 6px;
      cursor: pointer;
      transition: background 0.15s ease;
    }

    button:hover {
      background: var(--accent-cyan-hover);
    }

    button.secondary {
      background: var(--bg-card);
      color: var(--text-primary);
      border: 1px solid var(--border-subtle);
    }

    button.secondary:hover {
      background: #252b36;
    }

    .meta-info {
      font-size: 0.78rem;
      color: var(--text-muted);
      display: flex;
      justify-content: space-between;
      padding-top: 0.5rem;
      border-top: 1px solid var(--border-subtle);
    }

    code {
      font-family: var(--font-mono);
      color: var(--text-cyan);
      background: var(--bg-input);
      padding: 0.15rem 0.35rem;
      border-radius: 4px;
      font-size: 0.75rem;
    }
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <div class="title-group">
        <svg class="title-icon" viewBox="0 0 24 24">
          <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.929 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.894.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/>
        </svg>
        <h1>Discord Rich Presence</h1>
      </div>
      <div id="statusBadge" class="badge disconnected">
        <span class="status-dot"></span>
        <span id="statusText">CONNECTING...</span>
      </div>
    </div>

    <!-- Live Preview Card -->
    <div class="card">
      <div class="card-title">Live Discord Preview</div>
      <div class="discord-preview">
        <div class="discord-avatar-wrap">
          <div class="discord-large-img">N</div>
          <div class="discord-small-img" id="previewLangIcon">JS</div>
        </div>
        <div class="discord-content">
          <div class="discord-game-title">Nicle IDE</div>
          <div class="discord-details" id="previewDetails">Exploring Codebase</div>
          <div class="discord-state" id="previewState">No Workspace Open</div>
          <div class="discord-time" id="previewTime">00:00 elapsed</div>
        </div>
      </div>
    </div>

    <!-- Privacy & Presence Controls -->
    <div class="card">
      <div class="card-title">Presence Preferences</div>
      <div class="controls-grid">
        <div class="toggle-row">
          <span class="toggle-label">Hide Active File Name</span>
          <label class="toggle-switch">
            <input type="checkbox" id="toggleHideFile" onchange="updateSettings()">
            <span class="slider"></span>
          </label>
        </div>
        <div class="toggle-row">
          <span class="toggle-label">Hide Workspace Name</span>
          <label class="toggle-switch">
            <input type="checkbox" id="toggleHideWorkspace" onchange="updateSettings()">
            <span class="slider"></span>
          </label>
        </div>
        <div class="toggle-row">
          <span class="toggle-label">Set Status to Idle</span>
          <label class="toggle-switch">
            <input type="checkbox" id="toggleIdle" onchange="updateSettings()">
            <span class="slider"></span>
          </label>
        </div>
        <div class="toggle-row">
          <span class="toggle-label">Companion Port</span>
          <code>127.0.0.1:${PORT}</code>
        </div>
      </div>

      <div class="actions-bar">
        <button class="secondary" onclick="reconnectDiscord()">Reconnect Discord</button>
        <button onclick="refreshStatus()">Refresh</button>
      </div>
    </div>

    <div class="meta-info">
      <span>Nicle Companion Service v1.0.0</span>
      <span id="socketInfo">Socket: Searching...</span>
    </div>
  </div>

  <script>
    let elapsedSeconds = 0;
    setInterval(() => {
      elapsedSeconds++;
      const mins = String(Math.floor(elapsedSeconds / 60)).padStart(2, '0');
      const secs = String(elapsedSeconds % 60).padStart(2, '0');
      document.getElementById('previewTime').textContent = mins + ':' + secs + ' elapsed';
    }, 1000);

    async function refreshStatus() {
      try {
        const res = await fetch('/api/status');
        if (!res.ok) return;
        const data = await res.json();

        // Status Badge
        const badge = document.getElementById('statusBadge');
        const statusText = document.getElementById('statusText');
        if (data.discordConnected) {
          badge.className = 'badge connected';
          statusText.textContent = 'CONNECTED';
        } else {
          badge.className = 'badge disconnected';
          statusText.textContent = 'DISCORD OFFLINE';
        }

        // Preview content
        if (data.activity) {
          const act = data.activity;
          document.getElementById('previewDetails').textContent = act.details || 'Idling in Nicle IDE';
          document.getElementById('previewState').textContent = act.state || '';
          const smallIcon = act.assets?.small_image || 'code';
          document.getElementById('previewLangIcon').textContent = smallIcon.slice(0, 3).toUpperCase();
        }

        // Settings checkboxes
        if (data.settings) {
          document.getElementById('toggleHideFile').checked = !!data.settings.hideFile;
          document.getElementById('toggleHideWorkspace').checked = !!data.settings.hideWorkspace;
          document.getElementById('toggleIdle').checked = !!data.settings.idle;
        }

        if (data.socketPath) {
          document.getElementById('socketInfo').textContent = 'IPC: ' + data.socketPath;
        } else {
          document.getElementById('socketInfo').textContent = 'IPC: Disconnected';
        }
      } catch (err) {
        console.error('Status fetch error:', err);
      }
    }

    async function updateSettings() {
      const payload = {
        hideFile: document.getElementById('toggleHideFile').checked,
        hideWorkspace: document.getElementById('toggleHideWorkspace').checked,
        idle: document.getElementById('toggleIdle').checked,
      };

      try {
        await fetch('/api/settings', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload)
        });
        await refreshStatus();
      } catch (err) {
        console.error('Failed to save settings:', err);
      }
    }

    async function reconnectDiscord() {
      try {
        await fetch('/api/reconnect', { method: 'POST' });
        await refreshStatus();
      } catch (err) {
        console.error('Reconnect trigger failed:', err);
      }
    }

    refreshStatus();
    setInterval(refreshStatus, 2500);
  </script>
</body>
</html>`;
}

// HTTP Companion Server
const server = http.createServer((req, res) => {
  const parsedUrl = new URL(req.url, `http://${HOST}:${PORT}`);
  const pathname = parsedUrl.pathname;

  // Helper to validate and return allowed origin for CORS (loopback & Tauri only)
  const getAllowedOrigin = (request) => {
    const origin = request.headers.origin || "";
    if (!origin) return `http://${HOST}:${PORT}`;
    try {
      const u = new URL(origin);
      if (
        u.protocol === "tauri:" ||
        u.hostname === "tauri.localhost" ||
        u.hostname === "127.0.0.1" ||
        u.hostname === "localhost"
      ) {
        return origin;
      }
    } catch {}
    return `http://${HOST}:${PORT}`;
  };

  // Preflight CORS handler
  if (req.method === "OPTIONS") {
    res.writeHead(204, {
      "Access-Control-Allow-Origin": getAllowedOrigin(req),
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type",
    });
    return res.end();
  }

  // JSON helper
  const sendJson = (statusCode, data) => {
    res.writeHead(statusCode, {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": getAllowedOrigin(req),
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type",
    });
    res.end(JSON.stringify(data));
  };

  // Readiness / Health probe
  if (req.method === "GET" && pathname === "/health") {
    return sendJson(200, {
      status: "healthy",
      discordConnected: rpc.connected,
      uptimeSeconds: Math.floor((Date.now() - startTime) / 1000),
      port: PORT,
      activeFile: currentActivity.file?.name ?? null,
      workspace: currentActivity.workspace?.name ?? null,
    });
  }

  // Activity update POST from Nicle IDE
  if (req.method === "POST" && pathname === "/activity") {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
      if (body.length > 65536) {
        req.destroy(); // Protect against oversized payloads
      }
    });

    req.on("end", () => {
      try {
        const payload = JSON.parse(body);
        currentActivity = {
          workspace: payload.workspace ?? null,
          file: payload.file ?? null,
          timestamp: payload.timestamp ?? Date.now(),
        };

        rpc.updatePresence();
        sendJson(200, { ok: true });
      } catch (err) {
        sendJson(400, { ok: false, error: "Invalid JSON" });
      }
    });
    return;
  }

  // Status API
  if (req.method === "GET" && pathname === "/api/status") {
    return sendJson(200, {
      status: "healthy",
      discordConnected: rpc.connected,
      uptimeSeconds: Math.floor((Date.now() - startTime) / 1000),
      socketPath: rpc.currentSocketPath,
      settings,
      activity: rpc.buildActivity(),
    });
  }

  // Settings API
  if (req.method === "POST" && pathname === "/api/settings") {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
      if (body.length > 65536) req.destroy();
    });

    req.on("end", () => {
      try {
        const data = JSON.parse(body);
        if (typeof data.hideWorkspace === "boolean") settings.hideWorkspace = data.hideWorkspace;
        if (typeof data.hideFile === "boolean") settings.hideFile = data.hideFile;
        if (typeof data.idle === "boolean") settings.idle = data.idle;
        if (typeof data.clientId === "string" && data.clientId.trim()) {
          settings.clientId = data.clientId.trim();
          rpc._disconnect();
          rpc.connect();
        }

        rpc.updatePresence();
        sendJson(200, { ok: true, settings });
      } catch (err) {
        sendJson(400, { ok: false, error: "Invalid JSON" });
      }
    });
    return;
  }

  // Reconnect command API
  if ((req.method === "GET" || req.method === "POST") && pathname === "/api/reconnect") {
    rpc._disconnect();
    rpc.connect();
    return sendJson(200, { ok: true, message: "Reconnection triggered" });
  }

  // Toggle idle command API
  if ((req.method === "GET" || req.method === "POST") && pathname === "/api/toggle-idle") {
    settings.idle = !settings.idle;
    rpc.updatePresence();
    return sendJson(200, { ok: true, idle: settings.idle });
  }

  // Open Dashboard command API
  if ((req.method === "GET" || req.method === "POST") && pathname === "/api/open-dashboard") {
    const cmd = process.platform === "win32" ? "start http://127.0.0.1:3070/" : (process.platform === "darwin" ? "open http://127.0.0.1:3070/" : "xdg-open http://127.0.0.1:3070/");
    import("node:child_process").then(cp => cp.exec(cmd)).catch(() => {});
    return sendJson(200, { ok: true, message: "Dashboard opened" });
  }

  // Web Dashboard UI
  if (req.method === "GET" && pathname === "/") {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    return res.end(renderDashboardHtml());
  }

  sendJson(404, { error: "Not found" });
});

server.listen(PORT, HOST, () => {
  console.log(`[DISCORD] Companion server listening on http://${HOST}:${PORT}`);
  console.log(`[DISCORD] Health probe: http://${HOST}:${PORT}/health`);
  console.log(`[DISCORD] Dashboard:    http://${HOST}:${PORT}/`);
});

// Clean shutdown lifecycle
function shutdown() {
  console.log("[DISCORD] Shutting down Discord Presence service...");
  server.close(() => {
    rpc.destroy();
    process.exit(0);
  });

  // Force exit if hanging
  setTimeout(() => {
    rpc.destroy();
    process.exit(0);
  }, 2000).unref();
}

process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);
