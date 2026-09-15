<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Search,
    X,
    Package,
    AlertCircle,
    RotateCw,
    Download,
    ShieldCheck,
    CheckCircle2,
    Sliders,
    FolderPlus,
    Plus,
  } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    plugins,
    initPlugins,
    installApp,
    startApp,
    stopApp,
    linkDevPlugin,
    type CatalogPlugin,
  } from '../../lib/plugins.svelte';
  import PluginDetails from './PluginDetails.svelte';
  import CreatePluginModal from './CreatePluginModal.svelte';

  interface Props {
    onClose?: () => void;
  }

  const { onClose }: Props = $props();

  let isSmallScreen = $state(false);
  let linkError = $state<string | null>(null);

  function checkWidth() {
    if (typeof window !== 'undefined') {
      isSmallScreen = window.innerWidth < 900;
    }
  }

  onMount(() => {
    checkWidth();
    window.addEventListener('resize', checkWidth);
    void initPlugins();
    return () => {
      window.removeEventListener('resize', checkWidth);
    };
  });

  const displayList = $derived.by(() => {
    return plugins.activeTab === 'discover'
      ? plugins.filteredCatalog
      : plugins.filteredInstalled;
  });

  const selectedApp = $derived(
    plugins.allPlugins.find(c => c.id === plugins.selectedId) || plugins.allPlugins[0]
  );

  async function handleLinkLocalPlugin() {
    linkError = null;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Local Plugin Directory (containing nicle-plugin.json)',
      });
      if (typeof selected === 'string') {
        await linkDevPlugin(selected);
      }
    } catch (err) {
      linkError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleQuickToggle(e: MouseEvent, appId: string) {
    e.stopPropagation();
    const st = plugins.getStatus(appId);
    if (st.transitionActive) return;
    if (st.phase === 'running') {
      await stopApp(appId);
    } else {
      await startApp(appId);
    }
  }

  async function handleQuickInstall(e: MouseEvent, appId: string) {
    e.stopPropagation();
    await installApp(appId);
  }
</script>

<div class="plugins-workbench-view">
  <!-- Top Plugins Navigation and Search Bar -->
  <header class="plugins-header-bar">
    <div class="header-nav-left">
      <div class="title-with-icon">
        <Package size={18} style="color: var(--accent);" />
        <h1 class="plugins-title">Plugins</h1>
      </div>

      <div class="plugins-tab-switcher" role="tablist">
        <button
          role="tab"
          aria-selected={plugins.activeTab === 'discover'}
          class="nav-tab-btn"
          class:active={plugins.activeTab === 'discover'}
          onclick={() => (plugins.activeTab = 'discover')}
        >
          Discover
        </button>
        <button
          role="tab"
          aria-selected={plugins.activeTab === 'installed'}
          class="nav-tab-btn"
          class:active={plugins.activeTab === 'installed'}
          onclick={() => (plugins.activeTab = 'installed')}
        >
          Installed
          {#if plugins.installed.length > 0}
            <span class="count-bubble">{plugins.installed.length}</span>
          {/if}
        </button>
      </div>
    </div>

    <!-- Search Input -->
    <div class="header-nav-right">
      <div class="search-input-wrapper">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          placeholder="Search apps…"
          aria-label="Search apps"
          bind:value={plugins.searchQuery}
          class="search-text-field"
        />
        {#if plugins.searchQuery}
          <button
            class="icon-btn clear-search-btn"
            aria-label="Clear search"
            onclick={() => (plugins.searchQuery = '')}
          >
            <X size={13} />
          </button>
        {/if}
      </div>

      <button
        class="btn-secondary link-plugin-btn"
        onclick={handleLinkLocalPlugin}
        title="Link local directory containing nicle-plugin.json"
        aria-label="Link local plugin"
      >
        <FolderPlus size={14} />
        <span>Link Local</span>
      </button>

      <button
        class="btn-primary create-plugin-btn"
        onclick={() => (plugins.showCreateModal = true)}
        title="Create and scaffold a new plugin from template"
        aria-label="Create new plugin"
      >
        <Plus size={14} />
        <span>New Plugin</span>
      </button>

      {#if onClose}
        <button
          class="icon-btn close-plugins-btn"
          onclick={onClose}
          title="Return to editor (Esc)"
          aria-label="Close plugins page"
        >
          <X size={18} />
        </button>
      {/if}
    </div>
  </header>

  <!-- Error alert if linking failed -->
  {#if linkError}
    <div class="runtime-alert-banner" role="alert">
      <AlertCircle size={16} style="color: var(--error);" />
      <div class="alert-content">
        <strong>Plugin linking error:</strong>
        <span>{linkError}</span>
      </div>
      <button class="icon-btn" onclick={() => (linkError = null)} aria-label="Dismiss error">
        <X size={14} />
      </button>
    </div>
  {/if}

  <!-- Runtime Notice Banner if Node/npm missing -->
  {#if plugins.runtimeStatus && !plugins.runtimeStatus.available}
    <div class="runtime-alert-banner" role="alert">
      <AlertCircle size={16} style="color: var(--warning); flex-shrink: 0;" />
      <div class="runtime-alert-text">
        <strong>Runtime requirement:</strong> {plugins.runtimeStatus.error}
      </div>
      <button class="btn-secondary" onclick={() => void initPlugins()}>
        <RotateCw size={13} />
        <span>Check again</span>
      </button>
    </div>
  {/if}

  <!-- Main Split Layout -->
  <div class="plugins-body-layout" class:single-column={isSmallScreen}>
    <!-- Column 1: App List -->
    {#if !isSmallScreen || !plugins.selectedId}
      <aside class="plugins-list-column" aria-label="Available companion apps">
        {#if plugins.loading}
          <div class="loading-state">
            <RotateCw size={20} class="spin" />
            <span>Loading companion apps…</span>
          </div>
        {:else if plugins.errorMessage}
          <div class="empty-state">
            <AlertCircle size={24} style="color: var(--error);" />
            <p>Could not load apps: {plugins.errorMessage}</p>
            <button class="btn-primary" onclick={() => void initPlugins()}>
              Retry
            </button>
          </div>
        {:else if displayList.length === 0}
          <div class="empty-state">
            {#if plugins.searchQuery}
              <p>No apps match "{plugins.searchQuery}".</p>
              <button class="btn-secondary" onclick={() => (plugins.searchQuery = '')}>
                Clear search
              </button>
            {:else if plugins.activeTab === 'installed'}
              <p>No apps installed yet.</p>
              <button class="btn-primary" onclick={() => (plugins.activeTab = 'discover')}>
                Browse apps
              </button>
            {:else}
              <p>No companion apps available.</p>
            {/if}
          </div>
        {:else}
          <ul class="apps-card-list" role="listbox">
            {#each displayList as appItem (appItem.id)}
              {@const isInst = plugins.installedMap.has(appItem.id)}
              {@const st = plugins.getStatus(appItem.id)}
              {@const isSelected = plugins.selectedId === appItem.id}
              <li
                class="app-card-item"
                class:selected={isSelected && !isSmallScreen}
                role="option"
                aria-selected={isSelected}
                tabindex="0"
                onclick={() => (plugins.selectedId = appItem.id)}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    plugins.selectedId = appItem.id;
                  }
                }}
              >
                <div class="app-card-main">
                  <div class="app-card-header">
                    <div class="app-name-wrap">
                      <strong class="app-card-title">{appItem.name}</strong>
                      <span class="publisher-tag">{appItem.publisher}</span>
                    </div>
                    {#if isInst}
                      <span
                        class="status-dot-pill"
                        class:running={st.phase === 'running'}
                        class:busy={st.phase === 'starting' || st.phase === 'stopping'}
                      >
                        {st.phase === 'running' ? 'Running' : st.phase === 'starting' ? 'Starting' : 'Off'}
                      </span>
                    {/if}
                  </div>

                  <p class="app-card-tagline">{appItem.tagline}</p>

                  <div class="app-card-footer">
                    <div class="tech-tags">
                      {#if appItem.isDev}
                        <span class="tech-badge dev-badge">DEV</span>
                      {/if}
                      {#if appItem.pluginType === 'cli'}
                        <span class="tech-badge">CLI Tool</span>
                      {/if}
                      {#if appItem.runtimeRequirement.toLowerCase().includes('bun')}
                        <span class="tech-badge">Bun</span>
                      {:else}
                        <span class="tech-badge">Node.js</span>
                      {/if}
                    </div>

                    <div class="card-action-slot">
                      {#if appItem.pluginType === 'cli'}
                        <span class="cli-action-pill">CLI / IDE</span>
                      {:else if !isInst}
                        <button
                          class="btn-primary mini-btn"
                          disabled={st.phase === 'installing' || !plugins.runtimeStatus?.available}
                          onclick={(e) => handleQuickInstall(e, appItem.id)}
                          aria-label={`Install ${appItem.name}`}
                        >
                          <Download size={12} />
                          <span>{st.phase === 'installing' ? '…' : 'Install'}</span>
                        </button>
                      {:else}
                        <!-- Compact On/Off Switch -->
                        <button
                          type="button"
                          role="switch"
                          aria-checked={st.phase === 'running'}
                          aria-label={`Toggle ${appItem.name}`}
                          class="mini-switch"
                          class:checked={st.phase === 'running'}
                          disabled={st.transitionActive}
                          onclick={(e) => handleQuickToggle(e, appItem.id)}
                        >
                          <span class="mini-switch-thumb"></span>
                        </button>
                      {/if}
                    </div>
                  </div>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </aside>
    {/if}

    <!-- Column 2: Selected App Details -->
    {#if !isSmallScreen || plugins.selectedId}
      <section class="plugins-details-column">
        {#if selectedApp}
          <PluginDetails
            appId={selectedApp.id}
            onBack={isSmallScreen ? () => (plugins.selectedId = null) : undefined}
          />
        {:else}
          <div class="empty-state">
            <p>Select a companion app to manage.</p>
          </div>
        {/if}
      </section>
    {/if}
  </div>

  {#if plugins.showCreateModal}
    <CreatePluginModal onclose={() => (plugins.showCreateModal = false)} />
  {/if}
</div>

<style>
  .plugins-workbench-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background-color: var(--canvas);
    color: var(--text);
    overflow: hidden;
  }

  .plugins-header-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
    padding: 0 var(--space-4);
    background-color: var(--sidebar);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: var(--space-4);
  }

  .header-nav-left {
    display: flex;
    align-items: center;
    gap: var(--space-6);
  }

  .title-with-icon {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .plugins-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .plugins-tab-switcher {
    display: flex;
    gap: 4px;
    background-color: var(--canvas);
    padding: 2px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }

  .nav-tab-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    font-size: 12px;
    color: var(--muted);
    background: transparent;
    border: none;
    border-radius: 4px;
    transition: color 0.15s, background-color 0.15s;
  }

  .nav-tab-btn:hover {
    color: var(--text);
  }

  .nav-tab-btn.active {
    color: var(--text);
    background-color: var(--raised);
    font-weight: 500;
  }

  .count-bubble {
    font-size: 10px;
    background-color: var(--selection);
    color: var(--accent);
    padding: 1px 5px;
    border-radius: 999px;
    font-weight: 600;
  }

  .header-nav-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-text-field {
    width: 220px;
    height: 28px;
    padding: 4px 26px 4px 28px;
    background-color: var(--raised);
    border: 1px solid var(--control-border);
    border-radius: 4px;
    color: var(--text);
    font-size: 12px;
    transition: width 0.2s, border-color 0.2s;
  }

  .search-text-field:focus {
    width: 260px;
    border-color: var(--accent);
  }

  :global(.search-icon) {
    position: absolute;
    left: 8px;
    color: var(--muted);
    pointer-events: none;
  }

  .clear-search-btn {
    position: absolute;
    right: 4px;
    padding: 2px;
    color: var(--muted);
  }

  .runtime-alert-banner {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    background-color: rgba(232, 189, 117, 0.15);
    border-bottom: 1px solid var(--warning);
    padding: 8px 16px;
    font-size: 12px;
    color: var(--text);
  }

  .runtime-alert-text {
    flex: 1;
  }

  .plugins-body-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .plugins-list-column {
    width: 320px;
    min-width: 280px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    background-color: var(--sidebar);
  }

  .plugins-details-column {
    flex: 1;
    overflow: hidden;
    background-color: var(--canvas);
  }

  .single-column .plugins-list-column {
    width: 100%;
    border-right: none;
  }

  .apps-card-list {
    list-style: none;
    margin: 0;
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .app-card-item {
    background-color: var(--raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: var(--space-3);
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s;
    user-select: none;
  }

  .app-card-item:hover {
    background-color: var(--hover);
  }

  .app-card-item.selected {
    background-color: var(--selection);
    border-color: var(--accent);
  }

  .app-card-main {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .app-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .app-name-wrap {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .app-card-title {
    font-size: 13.5px;
    font-weight: 600;
  }

  .publisher-tag {
    font-size: 11px;
    color: var(--muted);
  }

  .status-dot-pill {
    font-size: 10.5px;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: var(--radius-control);
    background-color: var(--raised);
    color: var(--muted);
    border: 1px solid var(--border);
  }

  .status-dot-pill.running {
    background-color: var(--raised);
    color: var(--success);
    border-color: var(--success);
  }

  .status-dot-pill.busy {
    background-color: var(--selection);
    color: var(--accent);
    border-color: var(--control-border);
  }

  .app-card-tagline {
    margin: 0;
    font-size: 12px;
    color: var(--text-subtle);
    line-height: 1.4;
  }

  .app-card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 4px;
  }

  .tech-tags {
    display: flex;
    gap: 4px;
  }

  .tech-badge {
    font-size: 10px;
    background-color: var(--canvas);
    border: 1px solid var(--border);
    padding: 1px 5px;
    border-radius: var(--radius-control);
    color: var(--muted);
  }

  .tech-badge.dev-badge {
    color: var(--text);
    background-color: var(--selection);
    border-color: var(--control-border);
    font-weight: 600;
  }

  .cli-action-pill {
    font-size: 10.5px;
    font-weight: 600;
    color: var(--accent);
    background-color: var(--raised);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  .link-plugin-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .mini-btn {
    padding: 2px 8px;
    font-size: 11px;
  }

  /* Compact switch for card */
  .mini-switch {
    position: relative;
    width: 32px;
    height: 18px;
    background-color: var(--border);
    border-radius: 999px;
    border: 1px solid var(--control-border);
    cursor: pointer;
    padding: 0;
    transition: background-color 0.2s, border-color 0.2s;
  }

  .mini-switch.checked {
    background-color: var(--accent);
    border-color: var(--accent);
  }

  .mini-switch-thumb {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background-color: var(--text);
    transition: transform 0.2s;
  }

  .mini-switch.checked .mini-switch-thumb {
    transform: translateX(14px);
    background-color: var(--on-accent);
  }

  .mini-switch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .empty-state, .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--muted);
    gap: var(--space-3);
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  @media (max-width: 500px) {
    .search-text-field {
      width: 140px;
    }
    .search-text-field:focus {
      width: 170px;
    }
    .plugins-header-bar {
      padding: 0 var(--space-2);
    }
  }
</style>
