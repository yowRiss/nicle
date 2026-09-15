<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ArrowLeft,
    ExternalLink,
    Play,
    Square,
    RotateCw,
    Download,
    Trash2,
    Copy,
    Check,
    Settings,
    FileText,
    Info,
    AlertCircle,
    MoreVertical,
    RefreshCw,
    ShieldCheck,
  } from 'lucide-svelte';
  import {
    plugins,
    installApp,
    startApp,
    stopApp,
    restartApp,
    updateApp,
    uninstallApp,
    saveAppSettings,
    openDashboard,
    fetchLogs,
    clearLogs,
    unlinkDevPlugin,
    type CatalogPlugin,
    type InstalledRecord,
    type PluginRuntimeStatus,
    type PluginCommand,
  } from '../../lib/plugins.svelte';
  import { sendTerminalCommand } from '../../lib/tools.svelte';
  import Modal from '../Modal.svelte';

  interface Props {
    appId: string;
    onBack?: () => void;
  }

  const { appId, onBack }: Props = $props();

  let activeTab = $state<'overview' | 'settings' | 'logs'>('overview');
  let overflowOpen = $state(false);
  let uninstallModal = $state(false);
  let deleteDataChecked = $state(false);
  let actionError = $state<string | null>(null);
  let copied = $state(false);

  // Settings form state
  let configPort = $state<number>(20128);
  let startWithNicle = $state<boolean>(false);
  let settingsSaved = $state(false);

  const plugin = $derived(plugins.allPlugins.find(c => c.id === appId) || plugins.catalog.find(c => c.id === appId));
  const installedRecord = $derived(plugins.installed.find(i => i.id === appId));
  const status = $derived(plugins.getStatus(appId));
  const isInstalled = $derived(Boolean(installedRecord));
  const isRunning = $derived(status.phase === 'running');
  const isTransitioning = $derived(
    status.transitionActive ||
    status.phase === 'starting' ||
    status.phase === 'stopping' ||
    status.phase === 'installing' ||
    status.phase === 'updating' ||
    status.phase === 'uninstalling'
  );

  // Sync settings inputs when installed record changes
  $effect(() => {
    if (installedRecord) {
      configPort = installedRecord.port;
      startWithNicle = installedRecord.startWithNicle;
    } else if (plugin) {
      configPort = plugin.defaultPort;
      startWithNicle = false;
    }
  });

  // Refresh logs when logs tab is active
  $effect(() => {
    if (activeTab === 'logs' && appId) {
      void fetchLogs(appId);
    }
  });

  async function handleToggleSwitch() {
    actionError = null;
    try {
      if (isRunning) {
        await stopApp(appId);
      } else {
        await startApp(appId);
      }
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleInstall() {
    actionError = null;
    try {
      await installApp(appId);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleRestart() {
    actionError = null;
    try {
      await restartApp(appId);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleOpenDashboard() {
    actionError = null;
    try {
      await openDashboard(appId);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleUpdate() {
    overflowOpen = false;
    actionError = null;
    try {
      await updateApp(appId);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function confirmUninstall() {
    uninstallModal = false;
    actionError = null;
    try {
      await uninstallApp(appId, deleteDataChecked);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleUnlinkDev() {
    actionError = null;
    try {
      await unlinkDevPlugin(appId);
      if (onBack) onBack();
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleRunCommand(cmd: PluginCommand) {
    actionError = null;
    try {
      if (cmd.actionType === 'terminal') {
        await sendTerminalCommand(cmd.command);
      } else if (cmd.actionType === 'service') {
        if (!isRunning) {
          await startApp(appId);
        }
      }
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleSaveSettings() {
    if (!installedRecord) return;
    actionError = null;
    try {
      await saveAppSettings(appId, {
        port: Number(configPort),
        startWithNicle,
      });
      settingsSaved = true;
      setTimeout(() => (settingsSaved = false), 2500);
    } catch (err) {
      actionError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleCopyLogs() {
    const text = plugins.getLogs(appId);
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    }
  }

  function getStatusBadge(phase: string): { label: string; cls: string } {
    switch (phase) {
      case 'running':
        return { label: 'Running', cls: 'status-running' };
      case 'starting':
        return { label: 'Starting…', cls: 'status-starting' };
      case 'stopping':
        return { label: 'Stopping…', cls: 'status-stopping' };
      case 'installing':
        return { label: 'Installing…', cls: 'status-installing' };
      case 'updating':
        return { label: 'Updating…', cls: 'status-updating' };
      case 'uninstalling':
        return { label: 'Uninstalling…', cls: 'status-uninstalling' };
      case 'failed':
        return { label: 'Failed', cls: 'status-failed' };
      case 'installed-off':
        return { label: 'Off', cls: 'status-off' };
      default:
        return { label: 'Not Installed', cls: 'status-none' };
    }
  }

  const badge = $derived(getStatusBadge(status.phase));
</script>

{#if !plugin}
  <div class="plugin-details-empty">
    <Info size={28} style="color: var(--muted);" />
    <p>Select a companion app to view details and controls.</p>
  </div>
{:else}
  <div class="plugin-details-wrapper">
    <!-- Top Bar: Back button (if available) + Identity + Primary Actions -->
    <header class="plugin-details-header">
      <div class="header-left">
        {#if onBack}
          <button class="icon-btn back-btn" onclick={onBack} title="Back to app list" aria-label="Back to app list">
            <ArrowLeft size={16} />
          </button>
        {/if}
        <div class="app-title-group">
          <div class="title-row">
            <h2 class="app-name">{plugin.name}</h2>
            <span class="status-pill {badge.cls}" role="status" aria-live="polite">
              {badge.label}
            </span>
          </div>
          <div class="meta-row">
            <span class="meta-publisher">
              <ShieldCheck size={13} style="color: var(--accent); vertical-align: -2px;" />
              {plugin.publisher}
            </span>
            <span class="meta-sep">·</span>
            <span class="meta-pkg">{plugin.npmPackage}@{plugin.pinnedVersion}</span>
            {#if isInstalled && installedRecord}
              <span class="meta-sep">·</span>
              <span class="meta-port">Port {installedRecord.port}</span>
            {/if}
          </div>
        </div>
      </div>

      <!-- Action Area -->
      <div class="header-actions">
        {#if !isInstalled}
          <button
            class="btn-primary install-action-btn"
            disabled={isTransitioning || !plugins.runtimeStatus?.available}
            onclick={handleInstall}
            aria-label={`Install ${plugin.name}`}
          >
            <Download size={14} />
            <span>{status.phase === 'installing' ? 'Installing…' : 'Install'}</span>
          </button>
        {:else}
          <!-- Off/On Switch Toggle with Accessible 44px min Target -->
          <div class="switch-container">
            <span class="switch-label">{isRunning ? 'On' : 'Off'}</span>
            <button
              type="button"
              role="switch"
              aria-checked={isRunning}
              aria-label={`Toggle ${plugin.name} on or off`}
              disabled={isTransitioning}
              class="switch-control"
              class:checked={isRunning}
              class:busy={isTransitioning}
              onclick={handleToggleSwitch}
            >
              <span class="switch-thumb"></span>
            </button>
          </div>

          <!-- Open Dashboard -->
          <button
            class="btn-secondary"
            disabled={!isRunning}
            onclick={handleOpenDashboard}
            title="Open local web dashboard in default browser"
            aria-label="Open dashboard"
          >
            <ExternalLink size={14} />
            <span>Open dashboard</span>
          </button>

          <!-- Restart Button -->
          <button
            class="icon-btn"
            disabled={!isRunning || isTransitioning}
            onclick={handleRestart}
            title="Restart companion service"
            aria-label="Restart service"
          >
            <RotateCw size={15} />
          </button>

          <!-- Overflow Menu -->
          <div class="overflow-container">
            <button
              class="icon-btn"
              disabled={isTransitioning}
              onclick={() => (overflowOpen = !overflowOpen)}
              title="More actions"
              aria-label="More actions"
              aria-expanded={overflowOpen}
            >
              <MoreVertical size={16} />
            </button>
            {#if overflowOpen}
              <div class="overflow-dropdown">
                <button
                  class="dropdown-item"
                  onclick={handleUpdate}
                  disabled={isTransitioning}
                >
                  <RefreshCw size={14} />
                  <span>Check for update</span>
                </button>
                <div class="dropdown-sep"></div>
                <button
                  class="dropdown-item destructive"
                  onclick={() => {
                    overflowOpen = false;
                    uninstallModal = true;
                  }}
                  disabled={isTransitioning}
                >
                  <Trash2 size={14} />
                  <span>Uninstall…</span>
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </header>

    <!-- Error Banner if any -->
    {#if actionError || status.lastError}
      <div class="action-error-banner" role="alert">
        <AlertCircle size={15} style="color: var(--error); flex-shrink: 0;" />
        <span class="error-msg">{actionError || status.lastError}</span>
        <button class="icon-btn close-err-btn" onclick={() => { actionError = null; status.lastError = null; }} aria-label="Dismiss error">
          ×
        </button>
      </div>
    {/if}

    <!-- Tabs Navigation Strip -->
    <nav class="details-tab-nav" aria-label="Plugin details tabs">
      <button
        class="details-tab-btn"
        class:active={activeTab === 'overview'}
        onclick={() => (activeTab = 'overview')}
      >
        <Info size={14} />
        <span>Overview</span>
      </button>
      <button
        class="details-tab-btn"
        class:active={activeTab === 'settings'}
        onclick={() => (activeTab = 'settings')}
      >
        <Settings size={14} />
        <span>Settings</span>
      </button>
      <button
        class="details-tab-btn"
        class:active={activeTab === 'logs'}
        onclick={() => (activeTab = 'logs')}
      >
        <FileText size={14} />
        <span>Logs</span>
      </button>
    </nav>

    <!-- Tab Viewport -->
    <div class="tab-viewport">
      {#if activeTab === 'overview'}
        <section class="overview-pane">
          <div class="desc-card">
            <h3 class="section-title">About</h3>
            <p class="desc-text">{plugin.description}</p>
          </div>

          {#if installedRecord?.isDev}
            <div class="desc-card dev-banner-card">
              <div class="dev-banner-header">
                <span class="badge-dev">LOCAL DEV PLUGIN</span>
                <button class="btn-secondary mini-btn" onclick={handleUnlinkDev}>Unlink from IDE</button>
              </div>
              <div class="dev-path-display">
                <span class="label">Local Directory:</span>
                <code class="code-path">{installedRecord.installPath}</code>
              </div>
            </div>
          {/if}

          {#if plugin.commands && plugin.commands.length > 0}
            <div class="desc-card commands-card">
              <h3 class="section-title">IDE Commands & Terminal Actions</h3>
              <p class="desc-sub">Execute directly from the Command Palette (<code>Ctrl+Shift+P</code>) or launch below:</p>
              <div class="commands-grid">
                {#each plugin.commands as cmd}
                  <div class="cmd-item">
                    <div class="cmd-info">
                      <strong class="cmd-title">{cmd.title}</strong>
                      {#if cmd.description}
                        <span class="cmd-desc">{cmd.description}</span>
                      {/if}
                      <code class="cmd-snippet">{cmd.command}</code>
                    </div>
                    <button class="btn-primary mini-btn" onclick={() => handleRunCommand(cmd)}>
                      <Play size={12} />
                      <span>{cmd.actionType === 'terminal' ? 'Terminal' : 'Run'}</span>
                    </button>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- App Access and Boundaries Disclosure -->
          <div class="access-disclosure-card">
            <h3 class="section-title">
              <ShieldCheck size={16} style="color: var(--accent); vertical-align: -2px;" />
              App access & boundaries
            </h3>
            <p class="disclosure-body">{plugin.permissionsDisclosure}</p>
            <div class="disclosure-checklist">
              <div class="check-item">
                <span class="check-bullet">✓</span>
                <span>Supervised process execution with user permissions</span>
              </div>
              <div class="check-item">
                <span class="check-bullet">✓</span>
                <span>No arbitrary JS eval in editor UI thread (isolated process tree)</span>
              </div>
              <div class="check-item">
                <span class="check-bullet">✓</span>
                <span>Companion services bind exclusively to loopback <code>127.0.0.1</code></span>
              </div>
            </div>
          </div>

          <div class="details-grid">
            <div class="grid-item">
              <span class="grid-label">Runtime requirement</span>
              <span class="grid-val">{plugin.runtimeRequirement}</span>
            </div>
            <div class="grid-item">
              <span class="grid-label">System runtime status</span>
              <span class="grid-val">
                {#if plugin.runtimeRequirement.toLowerCase().includes('bun')}
                  {#if plugins.bunStatus?.available}
                    <span class="text-success">Detected (Bun {plugins.bunStatus.bunVersion})</span>
                  {:else}
                    <span class="text-error">{plugins.bunStatus?.error || 'Bun missing'}</span>
                  {/if}
                {:else if plugins.runtimeStatus?.available}
                  <span class="text-success">Detected (Node {plugins.runtimeStatus.nodeVersion}, npm {plugins.runtimeStatus.npmVersion})</span>
                {:else}
                  <span class="text-error">{plugins.runtimeStatus?.error || 'Node.js / npm missing'}</span>
                {/if}
              </span>
            </div>
            <div class="grid-item">
              <span class="grid-label">Package source</span>
              <span class="grid-val">npm registry ({plugin.npmPackage})</span>
            </div>
            <div class="grid-item">
              <span class="grid-label">Installed version</span>
              <span class="grid-val">{installedRecord?.version || 'Not installed'}</span>
            </div>
            {#if installedRecord}
              <div class="grid-item full-width">
                <span class="grid-label">Storage location</span>
                <span class="grid-val mono-path">{installedRecord.installPath}</span>
              </div>
            {/if}
            <div class="grid-item full-width">
              <span class="grid-label">Official links</span>
              <div class="links-row">
                <a href={plugin.homepage} target="_blank" rel="noreferrer" class="link-item">
                  <ExternalLink size={13} />
                  <span>GitHub Repository</span>
                </a>
                <a href={plugin.documentation} target="_blank" rel="noreferrer" class="link-item">
                  <ExternalLink size={13} />
                  <span>Documentation</span>
                </a>
              </div>
            </div>
          </div>
        </section>

      {:else}
        {#if activeTab === 'settings'}
          <section class="settings-pane">
            <form onsubmit={(e) => { e.preventDefault(); void handleSaveSettings(); }}>
              <div class="setting-row">
                <div class="setting-text">
                  <label for="plugin-port-input" class="setting-label">Service Port</label>
                  <p class="setting-desc">Local TCP port for the loopback server and dashboard. Must be between 1024 and 65535.</p>
                </div>
                <div class="setting-control">
                  <input
                    id="plugin-port-input"
                    type="number"
                    min="1024"
                    max="65535"
                    class="port-input"
                    bind:value={configPort}
                    disabled={!isInstalled || isTransitioning}
                  />
                </div>
              </div>

              <div class="setting-row">
                <div class="setting-text">
                  <label for="plugin-autostart-toggle" class="setting-label">Start with Nicle</label>
                  <p class="setting-desc">Automatically start this companion app in the background whenever Nicle opens.</p>
                </div>
                <div class="setting-control">
                  <input
                    id="plugin-autostart-toggle"
                    type="checkbox"
                    class="checkbox-input"
                    bind:checked={startWithNicle}
                    disabled={!isInstalled || isTransitioning}
                  />
                </div>
              </div>

              {#if isInstalled}
                <div class="settings-actions-bar">
                  <button
                    type="submit"
                    class="btn-primary"
                    disabled={isTransitioning}
                  >
                    Save settings
                  </button>
                  {#if settingsSaved}
                    <span class="save-indicator">
                      <Check size={14} style="color: var(--success); vertical-align: -2px;" />
                      Settings saved!
                    </span>
                  {/if}
                </div>
              {:else}
                <div class="notice-box">
                  <span>Install this companion app to configure its port and startup preferences.</span>
                </div>
              {/if}
            </form>
          </section>

        {:else}
          {#if activeTab === 'logs'}
            <section class="logs-pane">
              <div class="logs-toolbar">
                <div class="logs-title">
                  <span>Process output & installation events</span>
                </div>
                <div class="logs-actions">
                  <button
                    class="icon-btn"
                    onclick={() => void fetchLogs(appId)}
                    title="Refresh logs"
                    aria-label="Refresh logs"
                  >
                    <RefreshCw size={14} />
                  </button>
                  <button
                    class="btn-secondary copy-btn"
                    onclick={handleCopyLogs}
                    title="Copy full logs to clipboard"
                    aria-label="Copy logs"
                  >
                    {#if copied}
                      <Check size={13} style="color: var(--success);" />
                      <span>Copied!</span>
                    {:else}
                      <Copy size={13} />
                      <span>Copy</span>
                    {/if}
                  </button>
                  <button
                    class="btn-secondary"
                    onclick={() => void clearLogs(appId)}
                    title="Clear current log view"
                    aria-label="Clear log view"
                  >
                    <span>Clear view</span>
                  </button>
                </div>
              </div>

              <div class="logs-container" role="region" aria-label="App log viewer">
                <pre class="logs-code"><code>{plugins.getLogs(appId) || 'No logs recorded for this session yet.'}</code></pre>
              </div>
            </section>
          {/if}
        {/if}
      {/if}
    </div>
  </div>
{/if}

<!-- Uninstall Confirmation Modal -->
{#if uninstallModal && plugin}
  <Modal
    title={`Uninstall ${plugin.name}`}
    onclose={() => (uninstallModal = false)}
  >
    <div class="modal-body-content">
      <p>Are you sure you want to uninstall <strong>{plugin.name}</strong>? This will stop any active process and delete its managed package files.</p>
      
      <label class="modal-checkbox-row">
        <input type="checkbox" bind:checked={deleteDataChecked} />
        <span>Also delete companion app local data and settings</span>
      </label>

      <div class="modal-actions-bar">
        <button class="btn-secondary" onclick={() => (uninstallModal = false)}>
          Cancel
        </button>
        <button class="btn-destructive" onclick={confirmUninstall}>
          Uninstall app
        </button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  .plugin-details-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background-color: var(--canvas);
    color: var(--text);
  }

  .plugin-details-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--muted);
    padding: var(--space-6);
    text-align: center;
    gap: var(--space-3);
  }

  .plugin-details-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    border-bottom: 1px solid var(--border);
    background-color: var(--sidebar);
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .app-name {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .meta-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 12px;
    color: var(--muted);
    margin-top: 2px;
  }

  .meta-sep {
    opacity: 0.5;
  }

  .status-pill {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-radius: var(--radius-control);
  }

  .status-running {
    background-color: var(--raised);
    color: var(--success);
    border: 1px solid var(--success);
  }

  .status-starting, .status-stopping, .status-installing, .status-updating {
    background-color: var(--selection);
    color: var(--accent);
    border: 1px solid var(--control-border);
  }

  .status-failed {
    background-color: var(--raised);
    color: var(--error);
    border: 1px solid var(--error);
  }

  .status-off {
    background-color: var(--raised);
    color: var(--muted);
    border: 1px solid var(--border);
  }

  .status-none {
    background-color: transparent;
    color: var(--muted);
    border: 1px solid var(--border);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  /* Accessible 44px min toggle switch */
  .switch-container {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 44px;
  }

  .switch-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
    min-width: 24px;
  }

  .switch-control {
    position: relative;
    width: 44px;
    height: 24px;
    background-color: var(--border);
    border-radius: 999px;
    border: 1px solid var(--control-border);
    cursor: pointer;
    transition: background-color 0.2s, border-color 0.2s;
    padding: 0;
  }

  .switch-control:hover {
    border-color: var(--text);
  }

  .switch-control.checked {
    background-color: var(--accent);
    border-color: var(--accent);
  }

  .switch-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background-color: var(--text);
    transition: transform 0.2s;
  }

  .switch-control.checked .switch-thumb {
    transform: translateX(20px);
    background-color: var(--on-accent);
  }

  .switch-control:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .overflow-container {
    position: relative;
  }

  .overflow-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background-color: var(--raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px var(--shadow);
    z-index: 50;
    min-width: 150px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 12px;
    color: var(--text);
    text-align: left;
    width: 100%;
    justify-content: flex-start;
  }

  .dropdown-item:hover:not(:disabled) {
    background-color: var(--hover);
  }

  .dropdown-item.destructive {
    color: var(--error);
  }

  .dropdown-item.destructive:hover:not(:disabled) {
    background-color: var(--hover);
  }

  .dropdown-sep {
    height: 1px;
    background-color: var(--border);
    margin: 3px 0;
  }

  .action-error-banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background-color: var(--raised);
    border-bottom: 1px solid var(--error);
    border-left: 4px solid var(--error);
    padding: 8px 16px;
    font-size: 12px;
    color: var(--error);
  }

  .error-msg {
    flex: 1;
  }

  .close-err-btn {
    font-size: 16px;
    line-height: 1;
  }

  .details-tab-nav {
    display: flex;
    border-bottom: 1px solid var(--border);
    background-color: var(--sidebar);
    padding: 0 var(--space-6);
    gap: var(--space-2);
  }

  .details-tab-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 8px 14px;
    font-size: 13px;
    color: var(--muted);
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    transition: color 0.15s, border-color 0.15s;
  }

  .details-tab-btn:hover {
    color: var(--text);
  }

  .details-tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .tab-viewport {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
  }

  .section-title {
    margin: 0 0 var(--space-2) 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .desc-card {
    margin-bottom: var(--space-6);
  }

  .desc-text {
    margin: 0;
    line-height: 1.6;
    color: var(--text);
  }

  .access-disclosure-card {
    background-color: var(--raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: var(--space-4);
    margin-bottom: var(--space-6);
  }

  .disclosure-body {
    margin: 0 0 var(--space-3) 0;
    line-height: 1.5;
    color: var(--muted);
    font-size: 12.5px;
  }

  .disclosure-checklist {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }

  .check-item {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .check-bullet {
    color: var(--accent);
    font-weight: bold;
  }

  .details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
  }

  .grid-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .grid-item.full-width {
    grid-column: 1 / -1;
  }

  .grid-label {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .grid-val {
    font-size: 13px;
    color: var(--text);
  }

  .mono-path {
    font-family: var(--font-mono);
    font-size: 12px;
    word-break: break-all;
    background-color: var(--raised);
    padding: 4px 8px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  .links-row {
    display: flex;
    gap: var(--space-4);
  }

  .link-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--accent);
    text-decoration: none;
    font-size: 13px;
  }

  .link-item:hover {
    text-decoration: underline;
  }

  /* Settings pane */
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--border);
    gap: var(--space-4);
  }

  .setting-label {
    display: block;
    font-weight: 600;
    font-size: 13px;
    margin-bottom: 2px;
  }

  .setting-desc {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .port-input {
    width: 110px;
    padding: 6px 10px;
    background-color: var(--raised);
    border: 1px solid var(--control-border);
    border-radius: 4px;
    color: var(--text);
    font-family: var(--font-mono);
  }

  .checkbox-input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }

  .settings-actions-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-6);
  }

  .save-indicator {
    font-size: 12px;
    color: var(--success);
  }

  .notice-box {
    padding: var(--space-4);
    background-color: var(--raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--muted);
    font-size: 12.5px;
  }

  /* Logs pane */
  .logs-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: var(--space-3);
  }

  .logs-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .logs-title {
    font-size: 12px;
    color: var(--muted);
  }

  .logs-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .logs-container {
    flex: 1;
    min-height: 320px;
    background-color: var(--canvas);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    padding: var(--space-3);
    overflow: auto;
  }

  .logs-code {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-all;
  }

  /* Modal contents */
  .modal-body-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .modal-checkbox-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }

  .modal-actions-bar {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .btn-destructive {
    background-color: var(--error);
    color: var(--on-accent);
    border: none;
    border-radius: var(--radius-control);
    padding: 6px 12px;
    font-weight: 500;
    cursor: pointer;
  }

  .text-success { color: var(--success); }
  .text-error { color: var(--error); }

  /* Dev banner & Commands styles */
  .dev-banner-card {
    border-left: 3px solid var(--accent);
    background-color: var(--raised);
    border-radius: var(--radius-control);
  }

  .dev-banner-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }

  .badge-dev {
    display: inline-block;
    padding: 2px 6px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: var(--text);
    background-color: var(--selection);
    border: 1px solid var(--control-border);
    border-radius: var(--radius-control);
  }

  .dev-path-display {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 12px;
  }

  .dev-path-display .label {
    color: var(--muted);
  }

  .code-path {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--text);
    background-color: var(--raised);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  .commands-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .desc-sub {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .commands-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .cmd-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background-color: var(--raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    gap: var(--space-3);
  }

  .cmd-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .cmd-title {
    font-size: 12.5px;
    color: var(--text);
  }

  .cmd-desc {
    font-size: 11.5px;
    color: var(--muted);
    margin: 0;
  }

  .cmd-snippet {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent);
    background-color: rgba(0, 0, 0, 0.2);
    padding: 1px 4px;
    border-radius: 3px;
    align-self: flex-start;
  }

  @media (max-width: 600px) {
    .details-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
