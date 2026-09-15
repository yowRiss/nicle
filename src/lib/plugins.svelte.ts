import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type Phase =
  | 'not-installed'
  | 'installing'
  | 'installed-off'
  | 'starting'
  | 'running'
  | 'stopping'
  | 'failed'
  | 'updating'
  | 'uninstalling';

export interface PluginCommand {
  id: string;
  title: string;
  description?: string;
  actionType: 'terminal' | 'run' | 'service';
  command: string;
  args?: string[];
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  homepage?: string;
  pluginType?: 'service' | 'cli' | 'command';
  runtime?: {
    type: 'node' | 'bun' | 'python' | 'binary' | 'system';
    minVersion?: string;
  };
  commands?: PluginCommand[];
  service?: {
    entry: string;
    defaultPort: number;
    readinessPath?: string;
    dashboardPath?: string;
    activityPath?: string;
  };
}

export interface RegisteredPluginCommand {
  pluginId: string;
  pluginName: string;
  commandId: string;
  title: string;
  description?: string;
  actionType: 'terminal' | 'run' | 'service';
  command: string;
  args: string[];
}

export interface CatalogPlugin {
  id: string;
  name: string;
  tagline: string;
  description: string;
  publisher: string;
  npmPackage: string;
  pinnedVersion: string;
  runtimeRequirement: string;
  defaultPort: number;
  homepage: string;
  documentation: string;
  permissionsDisclosure: string;
  installScriptsRequired: boolean;
  isDev?: boolean;
  pluginType?: 'service' | 'cli' | 'command';
  commands?: PluginCommand[];
  installPath?: string;
}

export interface InstalledRecord {
  id: string;
  version: string;
  installPath: string;
  installedAt: number;
  port: number;
  startWithNicle: boolean;
  isDev?: boolean;
  manifest?: PluginManifest;
}

export interface RuntimeStatus {
  available: boolean;
  nodeVersion: string | null;
  npmVersion: string | null;
  error: string | null;
}

export interface BunRuntimeStatus {
  available: boolean;
  bunVersion: string | null;
  error: string | null;
}

export interface PluginRuntimeStatus {
  id: string;
  phase: Phase;
  pid: number | null;
  port: number;
  dashboardUrl: string | null;
  lastError: string | null;
  transitionActive: boolean;
}

export interface AppSettingsPayload {
  port: number;
  startWithNicle: boolean;
}

class PluginsStore {
  catalog = $state<CatalogPlugin[]>([]);
  installed = $state<InstalledRecord[]>([]);
  runtimeStatus = $state<RuntimeStatus | null>(null);
  bunStatus = $state<BunRuntimeStatus | null>(null);
  statuses = $state<Record<string, PluginRuntimeStatus>>({});
  logs = $state<Record<string, string>>({});
  commands = $state<RegisteredPluginCommand[]>([]);
  activeTab = $state<'discover' | 'installed'>('discover');
  searchQuery = $state('');
  selectedId = $state<string | null>(null);
  showCreateModal = $state(false);
  loading = $state(false);
  errorMessage = $state<string | null>(null);

  installedMap = $derived(new Map(this.installed.map(i => [i.id, i])));

  allPlugins = $derived.by(() => {
    const list: CatalogPlugin[] = [...this.catalog];
    for (const inst of this.installed) {
      if (!list.some(c => c.id === inst.id)) {
        const m = inst.manifest;
        list.push({
          id: inst.id,
          name: m?.name ?? inst.id,
          tagline: m?.description ?? (inst.isDev ? 'Local dev plugin' : 'Installed plugin'),
          description: m?.description ?? 'Local custom plugin',
          publisher: m?.author ?? (inst.isDev ? 'Local Dev' : 'Custom'),
          npmPackage: inst.id,
          pinnedVersion: inst.version,
          runtimeRequirement: m?.runtime?.type ? `Runtime: ${m.runtime.type}` : 'Node.js or Bun',
          defaultPort: inst.port,
          homepage: m?.homepage ?? '',
          documentation: m?.homepage ?? '',
          permissionsDisclosure: 'Executes locally with user permissions under Nicle supervision.',
          installScriptsRequired: false,
          isDev: inst.isDev,
          pluginType: m?.pluginType ?? 'service',
          commands: m?.commands ?? [],
          installPath: inst.installPath,
        });
      } else {
        const idx = list.findIndex(c => c.id === inst.id);
        const existing = list[idx];
        if (existing && inst.isDev) {
          list[idx] = { ...existing, isDev: true, installPath: inst.installPath };
        }
      }
    }
    return list;
  });

  filteredCatalog = $derived.by(() => {
    const q = this.searchQuery.trim().toLowerCase();
    if (!q) return this.catalog;
    return this.catalog.filter(
      p =>
        p.name.toLowerCase().includes(q) ||
        p.tagline.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q) ||
        p.npmPackage.toLowerCase().includes(q) ||
        p.publisher.toLowerCase().includes(q)
    );
  });

  filteredInstalled = $derived.by(() => {
    const q = this.searchQuery.trim().toLowerCase();
    const installedIds = new Set(this.installed.map(i => i.id));
    const items = this.allPlugins.filter(c => installedIds.has(c.id));
    if (!q) return items;
    return items.filter(
      p =>
        p.name.toLowerCase().includes(q) ||
        p.tagline.toLowerCase().includes(q) ||
        p.npmPackage.toLowerCase().includes(q)
    );
  });

  getStatus(appId: string): PluginRuntimeStatus {
    if (this.statuses[appId]) return this.statuses[appId];
    const isInst = this.installedMap.has(appId);
    const rec = this.installedMap.get(appId);
    return {
      id: appId,
      phase: isInst ? 'installed-off' : 'not-installed',
      pid: null,
      port: rec?.port ?? 20128,
      dashboardUrl: null,
      lastError: null,
      transitionActive: false,
    };
  }

  getLogs(appId: string): string {
    return this.logs[appId] ?? '';
  }
}

export const plugins = new PluginsStore();

export async function initPlugins() {
  if (!isTauri()) {
    // Browser fallback
    plugins.catalog = [
      {
        id: '9router',
        name: '9Router',
        tagline: 'Local AI routing and gateway companion app',
        description: 'A local companion service that routes LLM requests across multiple AI providers, manages credentials, and tracks usage.',
        publisher: 'decolua',
        npmPackage: '9router',
        pinnedVersion: '0.5.69',
        runtimeRequirement: 'Node.js >= 18.0.0, npm >= 9.0.0',
        defaultPort: 20128,
        homepage: 'https://github.com/decolua/9router',
        documentation: 'https://github.com/decolua/9router/blob/master/cli/README.md',
        permissionsDisclosure: 'Executes as a supervised local process with user permissions. Binds exclusively to the loopback interface (127.0.0.1). Provider keys and dashboard settings remain in 9Router\'s own local data storage.',
        installScriptsRequired: false,
      },
      {
        id: 'discord-presence',
        name: 'Discord Rich Presence',
        tagline: 'Live Discord status for active file, workspace, line count, and coding time',
        description: 'A supervised local companion service connecting to your local Discord desktop client via Discord IPC. Displays your active workspace, current file, language icons, cursor coordinates, and elapsed coding time on your Discord profile with zero external dependencies and full privacy controls.',
        publisher: 'Nicle Community',
        npmPackage: 'discord-presence',
        pinnedVersion: '1.0.0',
        runtimeRequirement: 'Node.js >= 18.0.0 or Bun >= 1.0.0',
        defaultPort: 3070,
        homepage: 'https://github.com/nicle-ide/nicle',
        documentation: 'https://github.com/nicle-ide/nicle/tree/main/plugins/discord-presence',
        permissionsDisclosure: 'Connects to local Discord client via IPC socket/pipe (127.0.0.1 loopback only). Never transmits code content or connects to external servers.',
        installScriptsRequired: false,
      },
    ];
    plugins.runtimeStatus = {
      available: true,
      nodeVersion: 'v22.23.1',
      npmVersion: '10.9.8',
      error: null,
    };
    if (!plugins.selectedId && plugins.catalog[0]) {
      plugins.selectedId = plugins.catalog[0].id;
    }
    return;
  }

  plugins.loading = true;
  plugins.errorMessage = null;

  try {
    const [cat, inst, rt, bunRt, cmds] = await Promise.all([
      invoke<CatalogPlugin[]>('plugins_get_catalog'),
      invoke<InstalledRecord[]>('plugins_get_installed'),
      invoke<RuntimeStatus>('plugins_check_runtime'),
      invoke<BunRuntimeStatus>('plugins_check_bun_runtime').catch(() => ({ available: false, bunVersion: null, error: null })),
      invoke<RegisteredPluginCommand[]>('plugins_get_commands').catch(() => []),
    ]);
    plugins.catalog = cat;
    plugins.installed = inst;
    plugins.runtimeStatus = rt;
    plugins.bunStatus = bunRt;
    plugins.commands = cmds;

    // Fetch runtime statuses for all plugins
    for (const item of plugins.allPlugins) {
      try {
        const st = await invoke<PluginRuntimeStatus>('plugins_get_status', { appId: item.id });
        plugins.statuses[item.id] = st;
      } catch {
        // Ignored
      }
    }

    if (!plugins.selectedId && plugins.allPlugins[0]) {
      plugins.selectedId = plugins.allPlugins[0].id;
    }
  } catch (err) {
    plugins.errorMessage = err instanceof Error ? err.message : String(err);
  } finally {
    plugins.loading = false;
  }
}

export async function installApp(appId: string): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_install', { appId });
    await refreshInstalled();
  } catch (err) {
    throw err;
  }
}

export async function startApp(appId: string): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_start', { appId });
    await refreshStatus(appId);
  } catch (err) {
    throw err;
  }
}

export async function stopApp(appId: string): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_stop', { appId });
    await refreshStatus(appId);
  } catch (err) {
    throw err;
  }
}

export async function restartApp(appId: string): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_restart', { appId });
    await refreshStatus(appId);
  } catch (err) {
    throw err;
  }
}

export async function updateApp(appId: string): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_update', { appId });
    await refreshInstalled();
  } catch (err) {
    throw err;
  }
}

export async function uninstallApp(appId: string, deleteData = false): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_uninstall', { appId, deleteData });
    await refreshInstalled();
  } catch (err) {
    throw err;
  }
}

export async function saveAppSettings(appId: string, settings: AppSettingsPayload): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_save_settings', { appId, settings });
    await refreshInstalled();
  } catch (err) {
    throw err;
  }
}

export async function openDashboard(appId: string): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke('plugins_open_dashboard', { appId });
  } catch (err) {
    throw err;
  }
}

export const FRONTEND_MAX_LOG_BYTES_PER_PLUGIN = 1024 * 1024; // 1 MiB UTF-8
export const FRONTEND_MAX_TOTAL_LOG_BYTES = 4 * 1024 * 1024; // 4 MiB aggregate
export const FRONTEND_MAX_PENDING_BATCH_BYTES = 256 * 1024; // 256 KiB
export const LOG_BATCH_INTERVAL_MS = 80;
export const LOG_TRUNCATION_NOTICE = '[NOTICE: Prior log output truncated to retain 1 MiB buffer]\n';

interface PluginLogBuffer {
  chunks: string[];
  totalBytes: number;
  truncated: boolean;
}

const logBuffers = new Map<string, PluginLogBuffer>();
let pendingChunks: { id: string; chunk: string }[] = [];
let pendingBytes = 0;
let batchTimer: ReturnType<typeof setTimeout> | null = null;

function assembleLogString(buf: PluginLogBuffer): string {
  if (buf.truncated) {
    return LOG_TRUNCATION_NOTICE + buf.chunks.join('');
  }
  return buf.chunks.join('');
}

export function flushPendingLogs() {
  if (batchTimer !== null) {
    clearTimeout(batchTimer);
    batchTimer = null;
  }
  if (pendingChunks.length === 0) return;

  const currentBatch = pendingChunks;
  pendingChunks = [];
  pendingBytes = 0;

  const dirtyPlugins = new Set<string>();
  const encoder = new TextEncoder();

  for (const { id, chunk } of currentBatch) {
    let buf = logBuffers.get(id);
    if (!buf) {
      buf = { chunks: [], totalBytes: 0, truncated: false };
      logBuffers.set(id, buf);
    }

    const chunkBytes = encoder.encode(chunk).length;
    buf.chunks.push(chunk);
    buf.totalBytes += chunkBytes;

    while (buf.totalBytes > FRONTEND_MAX_LOG_BYTES_PER_PLUGIN && buf.chunks.length > 0) {
      const removed = buf.chunks.shift();
      if (removed) {
        const removedBytes = encoder.encode(removed).length;
        buf.totalBytes = Math.max(0, buf.totalBytes - removedBytes);
        buf.truncated = true;
      }
    }
    dirtyPlugins.add(id);
  }

  // Aggregate memory limit across all plugins
  let aggregateBytes = 0;
  for (const b of logBuffers.values()) {
    aggregateBytes += b.totalBytes;
  }

  if (aggregateBytes > FRONTEND_MAX_TOTAL_LOG_BYTES) {
    const sorted = [...logBuffers.entries()].sort((a, b) => b[1].totalBytes - a[1].totalBytes);
    for (const [id, buf] of sorted) {
      while (buf.totalBytes > FRONTEND_MAX_LOG_BYTES_PER_PLUGIN / 2 && buf.chunks.length > 0) {
        const removed = buf.chunks.shift();
        if (removed) {
          const removedBytes = encoder.encode(removed).length;
          buf.totalBytes = Math.max(0, buf.totalBytes - removedBytes);
          buf.truncated = true;
          dirtyPlugins.add(id);
        }
      }
      aggregateBytes = 0;
      for (const b of logBuffers.values()) aggregateBytes += b.totalBytes;
      if (aggregateBytes <= FRONTEND_MAX_TOTAL_LOG_BYTES) break;
    }
  }

  for (const id of dirtyPlugins) {
    const buf = logBuffers.get(id);
    if (buf) {
      plugins.logs[id] = assembleLogString(buf);
    }
  }
}

function scheduleLogFlush() {
  if (batchTimer !== null) return;
  batchTimer = setTimeout(() => {
    batchTimer = null;
    flushPendingLogs();
  }, LOG_BATCH_INTERVAL_MS);
}

export function queueLogChunk(id: string, chunk: string) {
  const chunkLen = chunk.length;
  if (pendingBytes + chunkLen > FRONTEND_MAX_PENDING_BATCH_BYTES) {
    flushPendingLogs();
  }
  pendingChunks.push({ id, chunk });
  pendingBytes += chunkLen;
  scheduleLogFlush();
}

export async function fetchLogs(appId: string): Promise<string> {
  if (!isTauri()) return '';
  try {
    const logs = await invoke<string>('plugins_get_logs', { appId });
    const bytes = new TextEncoder().encode(logs).length;
    logBuffers.set(appId, {
      chunks: [logs],
      totalBytes: bytes,
      truncated: logs.startsWith(LOG_TRUNCATION_NOTICE),
    });
    plugins.logs[appId] = logs;
    return logs;
  } catch {
    return '';
  }
}

export async function clearLogs(appId: string): Promise<void> {
  pendingChunks = pendingChunks.filter(item => item.id !== appId);
  pendingBytes = pendingChunks.reduce((acc, item) => acc + item.chunk.length, 0);
  logBuffers.delete(appId);
  plugins.logs[appId] = '';
  if (!isTauri()) return;
  try {
    await invoke('plugins_clear_logs', { appId });
  } catch {
    // Ignored
  }
}

export async function refreshInstalled() {
  if (!isTauri()) return;
  try {
    plugins.installed = await invoke<InstalledRecord[]>('plugins_get_installed');
    plugins.commands = await invoke<RegisteredPluginCommand[]>('plugins_get_commands').catch(() => []);
    for (const item of plugins.allPlugins) {
      await refreshStatus(item.id);
    }
  } catch {
    // Ignored
  }
}

export async function linkDevPlugin(dirPath: string): Promise<InstalledRecord> {
  if (!isTauri()) throw new Error('Only supported in native Tauri');
  const record = await invoke<InstalledRecord>('plugins_link_dev', { dirPath });
  await refreshInstalled();
  plugins.selectedId = record.id;
  plugins.activeTab = 'installed';
  return record;
}

export async function unlinkDevPlugin(appId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke('plugins_unlink_dev', { appId });
  await refreshInstalled();
}

export async function scaffoldPlugin(targetDir: string, template = 'claude-code-bun'): Promise<string> {
  if (!isTauri()) throw new Error('Only supported in native Tauri');
  return await invoke<string>('plugins_scaffold', { targetDir, template });
}

export async function refreshStatus(appId: string) {
  if (!isTauri()) return;
  try {
    const st = await invoke<PluginRuntimeStatus>('plugins_get_status', { appId });
    plugins.statuses[appId] = st;
  } catch {
    // Ignored
  }
}

export async function listenPluginEvents(): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};

  const unlistenStatus = await listen<PluginRuntimeStatus>('plugin-state-changed', event => {
    const st = event.payload;
    if (st && st.id) {
      plugins.statuses[st.id] = st;
      if (st.phase === 'installed-off' || st.phase === 'running' || st.phase === 'not-installed') {
        void refreshInstalled();
      }
    }
  });

  const unlistenLogs = await listen<{ id: string; chunk: string }>('plugin-log-chunk', event => {
    const { id, chunk } = event.payload;
    if (id && chunk) {
      queueLogChunk(id, chunk);
    }
  });

  return () => {
    unlistenStatus();
    unlistenLogs();
    flushPendingLogs();
  };
}

export interface EditorActivityPayload {
  workspace?: {
    name: string;
    root: string;
  };
  file?: {
    name: string;
    path: string;
    extension: string;
    line: number;
    column: number;
  };
  timestamp: number;
}

let activityDebounceTimer: ReturnType<typeof setTimeout> | null = null;
let lastActivityPayload: EditorActivityPayload | null = null;

export function dispatchEditorActivity(payload: {
  workspace?: { name: string; root: string };
  file?: { name: string; path: string; extension: string; line: number; column: number };
  timestamp: number;
}) {
  // Check if any running plugin needs activity updates before doing debounce work
  const hasSubscribedPlugin = plugins.installed.some(record => {
    return plugins.statuses[record.id]?.phase === 'running' && !!record.manifest?.service?.activityPath;
  });

  if (!hasSubscribedPlugin) {
    if (activityDebounceTimer) {
      clearTimeout(activityDebounceTimer);
      activityDebounceTimer = null;
    }
    return;
  }

  lastActivityPayload = payload;
  if (activityDebounceTimer) {
    clearTimeout(activityDebounceTimer);
  }

  activityDebounceTimer = setTimeout(() => {
    activityDebounceTimer = null;
    const currentPayload = lastActivityPayload;
    if (!currentPayload) return;

    for (const record of plugins.installed) {
      const status = plugins.statuses[record.id];
      if (status?.phase !== 'running') continue;

      const activityPath = record.manifest?.service?.activityPath;
      if (!activityPath) continue;

      const port = record.port || record.manifest?.service?.defaultPort || 3000;
      const targetUrl = `http://127.0.0.1:${port}${activityPath}`;

      try {
        void fetch(targetUrl, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(currentPayload),
          signal: typeof AbortSignal !== 'undefined' && 'timeout' in AbortSignal ? AbortSignal.timeout(2000) : undefined,
        }).catch(() => {
          // Ignored: editor performance must never be impacted by plugin network errors
        });
      } catch {
        // Ignored
      }
    }
  }, 300);
}
