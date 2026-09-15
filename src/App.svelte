<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    FolderOpen,
    FilePlus,
    FolderPlus,
    RefreshCw,
    PanelLeft,
    Search,
    Settings,
    X,
    Save,
    Files,
    Terminal,
    Play,
    Square,
    Globe,
    RotateCw,
    TerminalSquare,
    AlertCircle,
    ChevronRight,
    MoreVertical,
    FileCode,
    Package,
    GitBranch,
  } from 'lucide-svelte';

  import Palette from './components/Palette.svelte';
  import Preferences from './components/Settings.svelte';
  import BottomPanel from './components/BottomPanel.svelte';
  import Tree from './components/Tree.svelte';
  import Modal from './components/Modal.svelte';
  import PluginsPage from './components/plugins/PluginsPage.svelte';
  import GitPanel from './components/git/GitPanel.svelte';
  import DiffModal from './components/git/DiffModal.svelte';
  import BranchModal from './components/git/BranchModal.svelte';
  import { plugins, startApp, listenPluginEvents, dispatchEditorActivity } from './lib/plugins.svelte';
  import { gitStore } from './lib/git.svelte';
  import { setupBenchmark } from './lib/benchmark';

  import {
    tools,
    toggleTerminal,
    sendTerminalCommand,
    runFile,
    stopRunner,
    restartRunner,
    previewFile,
    closePreview,
    closeTerminal,
    cleanupTools,
    listenTools,
  } from './lib/tools.svelte';
  import { api, basename, parent, type Entry } from './lib/api';
  import {
    session as s,
    report,
    attempt,
    changed,
    openFile,
    activate,
    saveAll,
    removeTab,
    remapTabs,
    chooseWorkspace,
    openWorkspacePath,
    setAutoSave,
  } from './lib/session.svelte';
  import {
    mountEditor,
    saveDocument,
    findInDocument,
    configureEditor,
    isLargeFileDocument,
    toggleLargeFileMode,
    type EditorSettings,
  } from './editor/documents';

  let editorHost = $state<HTMLDivElement>();
  let toggleButtonEl = $state<HTMLButtonElement>();

  // Responsive state (Section 4 design.md)
  let isSmallScreen = $state(false);
  let sidebar = $state(true);
  let sidebarTab = $state<'explorer' | 'git'>('explorer');
  let sidebarWidth = $state(256);
  let sidebarDrag = $state<{ x: number; width: number }>();
  let userAdjustedSidebar = false;

  $effect(() => {
    if (s.root) {
      const _rev = s.revision;
      gitStore.debouncedRefresh(150);
    }
  });

  // HTML Preview state & resizer (Section 4 & 5 design.md)
  let previewWidth = $state(420);
  let previewDrag = $state<{ x: number; width: number }>();
  let previewActiveTab = $state<'editor' | 'preview'>('editor');
  let userAdjustedPreview = false;

  // Shell Layout and Navigation State
  let palette = $state<'none' | 'files' | 'commands'>('none');
  let preferences = $state(false);
  let overflowMenu = $state(false);
  let activeView = $state<'editor' | 'plugins'>('editor');
  let settings = $state<EditorSettings>({ theme: 'dark', fontSize: 14, tabSize: 2, wordWrap: false, autoSave: false });

  function updateSettings(next: EditorSettings) {
    settings = next;
    setAutoSave(next.autoSave);
    configureEditor(next);
    document.documentElement.dataset.theme = next.theme;
    if (isTauri()) void invoke('save_settings', { settings: next }).catch(report);
  }

  let zenMode = $state(false);
  let preZenSidebar = true;
  let preZenPanel = false;

  function toggleWordWrap() {
    updateSettings({ ...settings, wordWrap: !settings.wordWrap });
  }

  function toggleAutoSave() {
    updateSettings({ ...settings, autoSave: !settings.autoSave });
  }

  function toggleZenMode() {
    zenMode = !zenMode;
    if (zenMode) {
      preZenSidebar = sidebar;
      preZenPanel = tools.panelVisible;
      sidebar = false;
      tools.panelVisible = false;
      if (!settings.wordWrap) {
        updateSettings({ ...settings, wordWrap: true });
      }
    } else {
      sidebar = preZenSidebar;
      tools.panelVisible = preZenPanel;
    }
  }

  const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform);
  const modKey = isMac ? '⌘' : 'Ctrl';

  const runnable = $derived(/\.(js|mjs|cjs|ts|tsx|mts|cts|py|go|rs|c|cpp|cc|cxx)$/i.test(s.active));
  const html = $derived(/\.html?$/i.test(s.active));

  const commands = $derived([
    { label: 'Open Folder…', shortcut: `${modKey} O`, action: () => void openFolder() },
    { label: 'Save File', shortcut: `${modKey} S`, action: () => void attempt(() => saveDocument(s.active)) },
    { label: 'Save All Files', shortcut: `${modKey} Shift S`, action: () => void attempt(saveAll) },
    { label: 'Quick Open (Search Files)', shortcut: `${modKey} P`, action: () => (palette = 'files') },
    { label: 'Command Palette', shortcut: `${modKey} Shift P`, action: () => (palette = 'commands') },
    { label: 'Open Plugins', shortcut: `${modKey} Shift X`, action: () => (activeView = 'plugins') },
    { label: 'Return to Editor', action: () => (activeView = 'editor') },
    { label: 'Toggle Terminal', shortcut: `${modKey} \``, action: toggleTerminal },
    { label: 'Find and Replace', shortcut: `${modKey} F`, action: () => findInDocument() },
    { label: 'Toggle Explorer Sidebar', action: () => (sidebar = !sidebar) },
    { label: 'Open Settings', action: () => (preferences = true) },
    {
      label: 'Reveal in File Manager',
      shortcut: `${modKey} Alt R`,
      action: () => {
        const targetPath = s.selected?.path || s.active || s.root;
        if (targetPath) void api.revealInFileManager(targetPath);
      },
    },
    {
      label: 'Toggle Dark / Light Theme',
      action: () => updateSettings({ ...settings, theme: settings.theme === 'dark' ? 'light' : 'dark' }),
    },
    {
      label: `Toggle Word Wrap (${settings.wordWrap ? 'On' : 'Off'})`,
      shortcut: 'Alt Z',
      action: toggleWordWrap,
    },
    {
      label: `Toggle Zen Mode (${zenMode ? 'Exit' : 'Enter'})`,
      action: toggleZenMode,
    },
    {
      label: `Toggle Auto Save (${settings.autoSave ? 'On' : 'Off'})`,
      action: toggleAutoSave,
    },
    {
      label: 'Git: View Source Control',
      shortcut: `${modKey} Shift G`,
      action: () => {
        sidebar = true;
        sidebarTab = 'git';
      },
    },
    {
      label: 'Git: Stage All Changes',
      action: () => void attempt(() => gitStore.stage([])),
    },
    {
      label: 'Git: Unstage All Changes',
      action: () => void attempt(() => gitStore.unstage([])),
    },
    {
      label: 'Git: Commit',
      action: () => {
        sidebar = true;
        sidebarTab = 'git';
      },
    },
    {
      label: 'Git: Push',
      action: () => void attempt(() => gitStore.push()),
    },
    {
      label: 'Git: Pull',
      action: () => void attempt(() => gitStore.pull()),
    },
    {
      label: 'Git: Switch Branch…',
      action: () => void gitStore.openBranchModal(),
    },
    {
      label: 'Git: Create Branch…',
      action: () => void gitStore.openBranchModal(),
    },
    {
      label: 'Git: Refresh Status',
      action: () => void gitStore.refresh(),
    },
    ...(runnable
      ? [
          { label: 'Run Active File', action: () => void attempt(runFile) },
          { label: 'Restart Active Runner', action: () => void attempt(restartRunner) },
        ]
      : []),
    ...(tools.running ? [{ label: 'Stop Active Runner', action: () => void attempt(stopRunner) }] : []),
    ...(tools.terminalMounted ? [{ label: 'End Terminal Session', action: () => void attempt(closeTerminal) }] : []),
    ...(html ? [{ label: 'Preview HTML Live', action: () => void attempt(previewFile) }] : []),
    ...(tools.previewUrl ? [{ label: 'Close HTML Preview', action: () => void attempt(closePreview) }] : []),
    ...(s.active
      ? [
          {
            label: isLargeFileDocument(s.active)
              ? 'Disable Large File Mode for Active File'
              : 'Enable Large File Mode for Active File',
            action: () => void attempt(() => toggleLargeFileMode(s.active)),
          },
        ]
      : []),
    {
      label: 'Plugins: Link Local Plugin Folder…',
      action: () => {
        activeView = 'plugins';
        plugins.activeTab = 'installed';
      },
    },
    {
      label: 'Plugins: Create New Plugin…',
      action: () => {
        activeView = 'plugins';
        plugins.showCreateModal = true;
      },
    },
    ...plugins.commands.map(cmd => ({
      label: cmd.title,
      action: () => {
        if (cmd.actionType === 'terminal') {
          void sendTerminalCommand(cmd.command);
          if (activeView !== 'editor') activeView = 'editor';
        } else if (cmd.actionType === 'service') {
          void startApp(cmd.pluginId);
        }
      },
    })),
  ]);

  // Dialog State
  let dialog = $state<'none' | 'create-file' | 'create-folder' | 'rename' | 'delete' | 'menu'>('none');
  let value = $state('');
  let target = $state<Entry>();
  let pending = $state(false);
  let confirmAction = $state<(choice: 'save' | 'discard' | 'cancel') => void>();
  let confirmLabel = $state('');

  async function confirmDirty(tabs = s.tabs): Promise<boolean> {
    const dirty = tabs.filter(t => t.dirty);
    if (!dirty.length) return true;
    confirmLabel = dirty.length === 1 ? basename(dirty[0]?.path ?? '') : `${dirty.length} files`;
    const choice = await new Promise<'save' | 'discard' | 'cancel'>(resolve => (confirmAction = resolve));
    confirmAction = undefined;
    if (choice === 'cancel') return false;
    if (choice === 'save') for (const tab of dirty) await saveDocument(tab.path);
    return true;
  }

  async function closeTab(path: string) {
    if (await confirmDirty(s.tabs.filter(t => t.path === path))) {
      removeTab(path);
      if (tools.previewPath === path || s.tabs.length === 0) {
        if (tools.previewUrl) void attempt(closePreview);
      }
    }
  }

  async function cleanup() {
    await cleanupTools();
  }

  const openFolder = () =>
    attempt(async () => {
      await chooseWorkspace(() => confirmDirty(), cleanup);
    });

  function startDialog(kind: typeof dialog, entry = s.selected) {
    target = entry;
    value = kind === 'rename' && entry ? basename(entry.path) : '';
    dialog = kind;
  }

  async function mutate() {
    pending = true;
    try {
      const base = target?.directory ? target.path : parent(target?.path ?? '');
      if (dialog !== 'delete' && (!value.trim() || /[/\\]/.test(value) || value === '.' || value === '..')) {
        report('Enter a name without slashes or dot navigation');
        return;
      }
      if (dialog === 'create-file' || dialog === 'create-folder') {
        const path = base ? `${base}/${value}` : value;
        await api.create(path, dialog === 'create-folder');
        if (dialog === 'create-file') await openFile(path);
      }
      if (dialog === 'rename' && target) {
        const dir = parent(target.path);
        const destination = dir ? `${dir}/${value}` : value;
        await api.rename(target.path, destination);
        remapTabs(target.path, destination);
      }
      if (dialog === 'delete' && target) {
        const affected = s.tabs.filter(t => t.path === target?.path || t.path.startsWith(`${target?.path}/`));
        if (!(await confirmDirty(affected))) return;
        await api.delete(target.path);
        for (const tab of affected) removeTab(tab.path);
      }
      s.revision++;
      s.selected = undefined;
      dialog = 'none';
    } catch (e) {
      report(e);
    } finally {
      pending = false;
    }
  }

  function keydown(e: KeyboardEvent) {
    // Escape closes overlay explorer or menus, or returns to editor from plugins
    if (e.key === 'Escape') {
      if (overflowMenu) {
        overflowMenu = false;
        return;
      }
      if (activeView === 'plugins') {
        activeView = 'editor';
        return;
      }
      if (isSmallScreen && sidebar) {
        sidebar = false;
        toggleButtonEl?.focus();
        return;
      }
    }

    // Alt + Z: Toggle Word Wrap (VS Code standard)
    if (e.altKey && (e.key === 'z' || e.key === 'Z')) {
      e.preventDefault();
      toggleWordWrap();
      return;
    }

    if (!(e.ctrlKey || e.metaKey) || confirmAction || dialog !== 'none' || preferences) return;
    const key = e.key.toLowerCase();
    if (key === 'g' && e.shiftKey) {
      e.preventDefault();
      if (!sidebar) {
        sidebar = true;
        sidebarTab = 'git';
      } else if (sidebarTab === 'git') {
        sidebar = false;
      } else {
        sidebarTab = 'git';
      }
      return;
    }
    if (key === 'x' && e.shiftKey) {
      e.preventDefault();
      activeView = activeView === 'plugins' ? 'editor' : 'plugins';
      return;
    }
    if (key === 'p') {
      e.preventDefault();
      palette = e.shiftKey ? 'commands' : 'files';
      return;
    }
    if (key === '`') {
      e.preventDefault();
      toggleTerminal();
      return;
    }
    if (key === 'o') {
      e.preventDefault();
      openFolder();
      return;
    }
    if (palette !== 'none') return;
    if (key === 's') {
      e.preventDefault();
      void attempt(() => (e.shiftKey ? saveAll() : saveDocument(s.active)));
    }
    if (key === 'w') {
      e.preventDefault();
      void attempt(() => closeTab(s.active));
    }
    if (key === 'f' || key === 'h') {
      e.preventDefault();
      findInDocument(key === 'h');
    }
    if (e.key === 'Tab' && s.tabs.length) {
      e.preventDefault();
      const index = s.tabs.findIndex(t => t.path === s.active);
      const next = s.tabs[(index + (e.shiftKey ? -1 : 1) + s.tabs.length) % s.tabs.length];
      if (next) activate(next.path);
    }
  }

  // Sidebar resizer (200-360px per Section 4 design.md)
  function onSidebarPointerMove(e: PointerEvent) {
    if (!sidebarDrag) return;
    userAdjustedSidebar = true;
    const newWidth = Math.max(200, Math.min(360, sidebarDrag.width + (e.clientX - sidebarDrag.x)));
    sidebarWidth = newWidth;
  }

  function onSidebarKeyResize(e: KeyboardEvent) {
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      userAdjustedSidebar = true;
      sidebarWidth = Math.min(360, sidebarWidth + 10);
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      userAdjustedSidebar = true;
      sidebarWidth = Math.max(200, sidebarWidth - 10);
    }
  }

  // Preview resizer (Section 4 & 5 design.md: resizable divider on wide windows)
  function onPreviewPointerMove(e: PointerEvent) {
    if (!previewDrag) return;
    const currentSidebar = (!isSmallScreen && sidebar) ? sidebarWidth : 0;
    const available = window.innerWidth - currentSidebar;
    const maxPreview = Math.max(280, available - 360); // Preserve at least 360px for editor
    const newWidth = Math.max(280, Math.min(maxPreview, previewDrag.width - (e.clientX - previewDrag.x)));
    previewWidth = newWidth;
  }

  function onPreviewKeyResize(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      const currentSidebar = (!isSmallScreen && sidebar) ? sidebarWidth : 0;
      const maxPreview = Math.max(280, window.innerWidth - currentSidebar - 360);
      previewWidth = Math.min(maxPreview, previewWidth + 15);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      previewWidth = Math.max(280, previewWidth - 15);
    }
  }

  function checkScreen() {
    if (typeof window !== 'undefined') {
      const width = window.innerWidth;
      isSmallScreen = width < 900;
      if (isSmallScreen) {
        sidebar = false;
      } else if (!userAdjustedSidebar) {
        // Section 4: At 900-1279px explorer defaults to 220px; at 1280px+ defaults to 256px
        if (width < 1280) {
          sidebarWidth = 220;
        } else {
          sidebarWidth = 256;
        }
      }
    }
  }

  function ensureActiveTabVisible() {
    requestAnimationFrame(() => {
      const activeTabEl = document.querySelector('.editor-tab.active');
      activeTabEl?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    });
  }

  $effect(() => {
    if (s.active) {
      ensureActiveTabVisible();
    }
  });

  // Debounced editor activity dispatch for companion plugins (e.g. Discord RPC)
  $effect(() => {
    const activePath = s.active;
    const rootPath = s.root;
    const line = s.line;
    const column = s.column;

    dispatchEditorActivity({
      workspace: rootPath
        ? {
            name: basename(rootPath) || rootPath,
            root: rootPath,
          }
        : undefined,
      file: activePath
        ? {
            name: basename(activePath),
            path: activePath,
            extension: activePath.includes('.') ? (activePath.split('.').pop()?.toLowerCase() ?? '') : '',
            line: line || 1,
            column: column || 1,
          }
        : undefined,
      timestamp: Date.now(),
    });
  });

  // Relative path derivation for breadcrumb
  const breadcrumbParts = $derived.by(() => {
    if (!s.active) return [];
    if (s.root && s.active.startsWith(s.root)) {
      const rel = s.active.slice(s.root.length).replace(/^[/\\]+/, '');
      return rel ? rel.split(/[/\\]+/) : [basename(s.active)];
    }
    return s.active.split(/[/\\]+/).filter(Boolean);
  });

  function getLanguageLabel(path: string): string {
    if (!path) return 'Plain Text';
    const ext = path.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts':
        return 'TypeScript';
      case 'tsx':
        return 'TypeScript React';
      case 'js':
      case 'mjs':
      case 'cjs':
        return 'JavaScript';
      case 'jsx':
        return 'JavaScript React';
      case 'rs':
        return 'Rust';
      case 'py':
        return 'Python';
      case 'html':
      case 'htm':
        return 'HTML';
      case 'css':
        return 'CSS';
      case 'json':
        return 'JSON';
      case 'go':
        return 'Go';
      case 'c':
      case 'h':
        return 'C';
      case 'cpp':
      case 'hpp':
      case 'cc':
      case 'cxx':
        return 'C++';
      case 'md':
        return 'Markdown';
      default:
        return ext ? ext.toUpperCase() : 'Plain Text';
    }
  }

  onMount(() => {
    checkScreen();
    window.addEventListener('resize', checkScreen);

    const destroy = editorHost ? mountEditor(editorHost, changed, toggleWordWrap) : () => {};
    const events = isTauri() ? listenTools() : Promise.resolve(() => {});
    const pluginEvents = isTauri() ? listenPluginEvents() : Promise.resolve(() => {});
    if (isTauri()) setupBenchmark();

    if (isTauri()) {
      void invoke<EditorSettings>('load_settings')
        .then(next => {
          settings = next;
          setAutoSave(next.autoSave ?? false);
          configureEditor(next);
          document.documentElement.dataset.theme = next.theme;
        })
        .catch(report);
    }

    const unlisten = isTauri()
      ? getCurrentWindow().onCloseRequested(async e => {
          e.preventDefault();
          await attempt(async () => {
            if (await confirmDirty()) {
              await cleanup();
              await getCurrentWindow().destroy();
            }
          });
        })
      : Promise.resolve(() => {});

    return () => {
      window.removeEventListener('resize', checkScreen);
      destroy();
      void unlisten.then(fn => fn());
      void events.then(fn => fn());
      void pluginEvents.then(fn => fn());
    };
  });
</script>

<svelte:window
  onkeydown={keydown}
  onpointermove={e => {
    onSidebarPointerMove(e);
    onPreviewPointerMove(e);
  }}
  onpointerup={() => {
    sidebarDrag = undefined;
    previewDrag = undefined;
  }}
  onpointerdown={e => {
    if (
      overflowMenu &&
      !(e.target as HTMLElement)?.closest('.toolbar-overflow-menu') &&
      !(e.target as HTMLElement)?.closest('.overflow-trigger-btn')
    ) {
      overflowMenu = false;
    }
  }}
/>

<!-- Workspace Toolbar: 44px high -->
<header class="workspace-toolbar">
  <!-- Group 1: Identity & Explorer Toggle -->
  <div class="toolbar-group identity">
    <button
      bind:this={toggleButtonEl}
      class="icon-btn"
      class:active={sidebar}
      title="Toggle Explorer"
      aria-label="Toggle Explorer"
      onclick={() => (sidebar = !sidebar)}
    >
      <PanelLeft size={16} />
    </button>
    <strong class="brand">nicle<span class="brand-dot">.</span></strong>
    <span
      class="workspace-name"
      class:has-root={Boolean(s.root)}
      title={s.root ? s.root : 'No folder open'}
    >
      {s.root ? basename(s.root) : 'No folder open'}
    </span>
  </div>

  <div class="spacer"></div>

  <!-- Group 2: File Search -->
  <button
    class="search-trigger"
    aria-label="Search files by name"
    onclick={() => (palette = 'files')}
  >
    <div class="search-trigger-content">
      <Search size={14} />
      <span>Search files…</span>
    </div>
    <kbd>{modKey} P</kbd>
  </button>

  <div class="spacer"></div>

  <!-- Group 3: Contextual Actions (Section 4 & 5 design.md) -->
  <div class="toolbar-actions">
    <button class="icon-btn" onclick={openFolder} title="Open Folder… (Ctrl+O)" aria-label="Open folder">
      <FolderOpen size={16} />
    </button>

    <button
      class="icon-btn"
      disabled={!s.active}
      onclick={() => attempt(() => saveDocument(s.active))}
      title="Save active file (Ctrl+S)"
      aria-label="Save file"
    >
      <Save size={16} />
    </button>

    {#if runnable || tools.running}
      <div class="runner-group">
        {#if runnable}
          <button
            class="btn-run"
            disabled={tools.running}
            onclick={() => attempt(runFile)}
            title="Run current file"
            aria-label="Run current file"
          >
            <Play size={13} fill="currentColor" />
            <span>Run</span>
          </button>
        {/if}
        {#if tools.running}
          <button
            class="btn-stop"
            onclick={() => attempt(stopRunner)}
            title="Stop running process"
            aria-label="Stop running process"
          >
            <Square size={12} fill="currentColor" />
          </button>
        {/if}
      </div>
    {/if}

    {#if html}
      <button class="btn-secondary" onclick={() => attempt(previewFile)} title="Live HTML preview" aria-label="Preview HTML">
        <Globe size={14} />
        <span>Preview</span>
      </button>
    {/if}

    <button
      class="icon-btn"
      class:active={tools.panelVisible && tools.panel === 'terminal'}
      onclick={toggleTerminal}
      title="Toggle terminal (Ctrl+`)"
      aria-label="Toggle terminal"
    >
      <Terminal size={16} />
    </button>

    <button
      class="icon-btn"
      onclick={() => (palette = 'commands')}
      title="Command palette (Ctrl+Shift+P)"
      aria-label="Command palette"
    >
      <TerminalSquare size={16} />
    </button>

    <button
      class="icon-btn overflow-trigger-btn"
      class:active={overflowMenu}
      onclick={() => (overflowMenu = !overflowMenu)}
      title="More actions"
      aria-label="More actions"
    >
      <MoreVertical size={16} />
    </button>

    {#if overflowMenu}
      <div class="toolbar-overflow-menu" role="menu">
        <button
          class="toolbar-overflow-item"
          role="menuitem"
          onclick={() => {
            overflowMenu = false;
            palette = 'files';
          }}
        >
          <div class="toolbar-overflow-item-content">
            <Search size={14} />
            <span>Search files…</span>
          </div>
          <kbd>{modKey} P</kbd>
        </button>

        <button
          class="toolbar-overflow-item"
          role="menuitem"
          onclick={() => {
            overflowMenu = false;
            palette = 'commands';
          }}
        >
          <div class="toolbar-overflow-item-content">
            <TerminalSquare size={14} />
            <span>Command palette</span>
          </div>
          <kbd>{modKey} Shift P</kbd>
        </button>

        <button
          class="toolbar-overflow-item"
          role="menuitem"
          onclick={() => {
            overflowMenu = false;
            activeView = activeView === 'plugins' ? 'editor' : 'plugins';
          }}
        >
          <div class="toolbar-overflow-item-content">
            <Package size={14} />
            <span>Plugins</span>
          </div>
          <kbd>{modKey} Shift X</kbd>
        </button>

        <button
          class="toolbar-overflow-item"
          role="menuitem"
          onclick={() => {
            overflowMenu = false;
            updateSettings({ ...settings, theme: settings.theme === 'dark' ? 'light' : 'dark' });
          }}
        >
          <div class="toolbar-overflow-item-content">
            <RotateCw size={14} />
            <span>Toggle theme ({settings.theme === 'dark' ? 'Light' : 'Dark'})</span>
          </div>
        </button>

        <button
          class="toolbar-overflow-item"
          role="menuitem"
          onclick={() => {
            overflowMenu = false;
            toggleAutoSave();
          }}
        >
          <div class="toolbar-overflow-item-content">
            <Save size={14} />
            <span>Auto Save</span>
          </div>
          <span style="font-size: 11px; font-weight: 500; color: {settings.autoSave ? 'var(--success)' : 'var(--muted)'};">
            {settings.autoSave ? '✓ On' : 'Off'}
          </span>
        </button>

        <button
          class="toolbar-overflow-item"
          role="menuitem"
          onclick={() => {
            overflowMenu = false;
            toggleWordWrap();
          }}
        >
          <div class="toolbar-overflow-item-content">
            <Files size={14} />
            <span>Word Wrap</span>
          </div>
          <kbd>Alt Z</kbd>
        </button>

        {#if s.selected?.path || s.active || s.root}
          <button
            class="toolbar-overflow-item"
            role="menuitem"
            onclick={() => {
              overflowMenu = false;
              const p = s.selected?.path || s.active || s.root;
              if (p) void api.revealInFileManager(p);
            }}
          >
            <div class="toolbar-overflow-item-content">
              <FolderOpen size={14} />
              <span>Reveal in file manager</span>
            </div>
            <kbd>{modKey} Alt R</kbd>
          </button>
        {/if}
      </div>
    {/if}

    <button
      class="icon-btn"
      onclick={() => (preferences = true)}
      title="Preferences & Settings"
      aria-label="Settings"
    >
      <Settings size={16} />
    </button>
  </div>
</header>

<!-- Main Shell Area -->
<main class="workbench-body">
  <!-- Explorer overlay backdrop on small screens -->
  {#if isSmallScreen && sidebar}
    <div
      class="explorer-overlay-backdrop"
      onclick={() => (sidebar = false)}
      aria-hidden="true"
    ></div>
  {/if}

  <!-- Explorer Sidebar -->
  {#if sidebar}
    <aside
      class="explorer-panel"
      class:overlay-mode={isSmallScreen}
      style:--sidebar-width={`${sidebarWidth}px`}
      aria-label="Project Explorer"
    >
      <div class="explorer-header">
        <div class="sidebar-tab-switcher">
          <button
            class="sidebar-tab-btn"
            class:active={sidebarTab === 'explorer'}
            onclick={() => (sidebarTab = 'explorer')}
            title="Explorer"
          >
            <Files size={13} />
            <span>Explorer</span>
          </button>
          <button
            class="sidebar-tab-btn"
            class:active={sidebarTab === 'git'}
            onclick={() => (sidebarTab = 'git')}
            title="Source Control"
          >
            <GitBranch size={13} />
            <span>Git</span>
            {#if gitStore.isRepo && gitStore.totalChanges > 0}
              <span class="sidebar-tab-badge">{gitStore.totalChanges}</span>
            {/if}
          </button>
        </div>

        {#if sidebarTab === 'explorer'}
          <div class="explorer-actions">
            <button
              class="icon-btn"
              disabled={!s.root}
              aria-label="New file"
              title="New file"
              onclick={() => startDialog('create-file')}
            >
              <FilePlus size={15} />
            </button>
            <button
              class="icon-btn"
              disabled={!s.root}
              aria-label="New folder"
              title="New folder"
              onclick={() => startDialog('create-folder')}
            >
              <FolderPlus size={15} />
            </button>
            <button
              class="icon-btn"
              disabled={!s.root}
              aria-label="Refresh explorer"
              title="Refresh explorer"
              onclick={() => s.revision++}
            >
              <RefreshCw size={14} />
            </button>
            {#if isSmallScreen}
              <button
                class="icon-btn"
                aria-label="Close explorer"
                onclick={() => (sidebar = false)}
              >
                <X size={15} />
              </button>
            {/if}
          </div>
        {:else if isSmallScreen}
          <button
            class="icon-btn"
            aria-label="Close explorer"
            onclick={() => (sidebar = false)}
          >
            <X size={15} />
          </button>
        {/if}
      </div>

      {#if sidebarTab === 'explorer'}
        <div class="explorer-content">
          {#if s.root}
            <button class="explorer-root-header" onclick={() => (s.selected = undefined)}>
              <FolderOpen size={15} style="flex-shrink: 0;" />
              <span>{basename(s.root)}</span>
            </button>

            {#key s.root}
              <Tree
                revision={s.revision}
                selected={s.selected?.path ?? s.active}
                onopen={path => {
                  activeView = 'editor';
                  void openFile(path);
                }}
                onselect={entry => (s.selected = entry)}
                onerror={report}
                onmenu={entry => startDialog('menu', entry)}
                oncreatefile={folderPath => {
                  target = { name: '', path: folderPath, directory: true };
                  value = '';
                  dialog = 'create-file';
                }}
                oncreatefolder={folderPath => {
                  target = { name: '', path: folderPath, directory: true };
                  value = '';
                  dialog = 'create-folder';
                }}
              />
            {/key}
          {:else}
            <div class="explorer-empty">
              <Files size={32} style="color: var(--muted); opacity: 0.6;" />
              <h3>No folder open</h3>
              <p>Open a project folder to view files, make edits, and run commands.</p>
              <button class="btn-primary" onclick={openFolder}>
                <FolderOpen size={15} />
                Open folder
              </button>
            </div>
          {/if}
        </div>
      {:else}
        <GitPanel />
      {/if}
    </aside>

    <!-- Resizable Vertical Splitter -->
    {#if !isSmallScreen}
      <button
        class="splitter-v"
        class:dragging={Boolean(sidebarDrag)}
        aria-label="Resize explorer sidebar"
        onpointerdown={e => {
          e.preventDefault();
          sidebarDrag = { x: e.clientX, width: sidebarWidth };
        }}
        onkeydown={onSidebarKeyResize}
      ></button>
    {/if}
  {/if}

  <!-- Main Workbench Column -->
  <section class="workbench-main">
    {#if activeView === 'plugins'}
      <PluginsPage onClose={() => (activeView = 'editor')} />
    {:else}
      <!-- Tabs Strip: 36px high -->
      <nav class="tabs-strip" aria-label="Editor tabs">
        {#each s.tabs as tab (tab.path)}
          <div class="editor-tab" class:active={s.active === tab.path}>
            <button
              class="tab-title-btn"
              title={tab.path}
              onclick={() => {
                activeView = 'editor';
                activate(tab.path);
              }}
            >
              <span class="tab-ext-badge">{tab.path.split('.').pop()?.slice(0, 3) || 'txt'}</span>
              <span class="tab-file-name">{basename(tab.path)}</span>
              {#if tab.dirty}
                <span class="dirty-indicator" aria-label="Unsaved changes" title="Unsaved changes">●</span>
              {/if}
            </button>
          <button
            class="icon-btn tab-close-btn"
            aria-label={`Close ${basename(tab.path)}`}
            title="Close tab"
            onclick={e => {
              e.stopPropagation();
              void attempt(() => closeTab(tab.path));
            }}
          >
            <X size={13} />
          </button>
        </div>
      {/each}
    </nav>

    <!-- Middle: Breadcrumb + Editor + Optional HTML Preview -->
    <div class="editor-and-preview-row">
      <div class="editing-viewport">
        <!-- Breadcrumb: 28px high -->
        {#if s.active}
          <div class="breadcrumb-bar" title={s.active}>
            <div class="breadcrumb-path">
              {#each breadcrumbParts as segment, idx}
                <span
                  class="breadcrumb-segment"
                  class:active={idx === breadcrumbParts.length - 1}
                >
                  {segment}
                </span>
                {#if idx < breadcrumbParts.length - 1}
                  <span class="breadcrumb-sep">/</span>
                {/if}
              {/each}
            </div>
          </div>
        {/if}

        <!-- CodeMirror Editor Mount Point -->
        <div class="editor-mount" class:hidden={!s.active} bind:this={editorHost}></div>

        <!-- Redesigned Welcome Surface (dominant when no active file) -->
        {#if !s.active}
          <div class="welcome-surface">
            <div class="welcome-container">
              {#if !s.root}
                <!-- State A: No Workspace Open -->
                <div class="welcome-header">
                  <h1>Start a workspace</h1>
                  <p>Open a folder to browse files, edit code, and run commands.</p>
                </div>

                <div class="welcome-columns">
                  <div class="welcome-action-list">
                    <h2 class="welcome-section-title">Actions</h2>
                    <button class="welcome-action-row primary-action" onclick={openFolder}>
                      <div class="welcome-action-label">
                        <FolderOpen size={18} />
                        <div>
                          <div>Open folder</div>
                          <div class="welcome-action-desc">Select an existing project folder from disk</div>
                        </div>
                      </div>
                      <kbd>{modKey} O</kbd>
                    </button>

                    <button class="welcome-action-row" onclick={() => (palette = 'commands')}>
                      <div class="welcome-action-label">
                        <TerminalSquare size={18} style="color: var(--muted);" />
                        <div>
                          <div>Command palette</div>
                          <div class="welcome-action-desc">Explore and run any IDE command</div>
                        </div>
                      </div>
                      <kbd>{modKey} Shift P</kbd>
                    </button>
                  </div>

                  <div class="welcome-shortcut-list">
                    <h2 class="welcome-section-title">Keyboard shortcuts</h2>
                    <div class="welcome-shortcut-item unavailable">
                      <span>Open file <em class="shortcut-reason">(Requires an open workspace)</em></span>
                      <kbd>{modKey} P</kbd>
                    </div>
                    <div class="welcome-shortcut-item">
                      <span>Command palette</span>
                      <kbd>{modKey} Shift P</kbd>
                    </div>
                    <div class="welcome-shortcut-item unavailable">
                      <span>Save file <em class="shortcut-reason">(Requires an open file)</em></span>
                      <kbd>{modKey} S</kbd>
                    </div>
                    <div class="welcome-shortcut-item">
                      <span>Toggle terminal</span>
                      <kbd>{modKey} `</kbd>
                    </div>
                  </div>
                </div>
              {:else}
                <!-- State B: Workspace open, no file selected -->
                <div class="welcome-header">
                  <h1>{basename(s.root)}</h1>
                  <div class="welcome-path-tag">{s.root}</div>
                </div>

                <div class="welcome-columns">
                  <div class="welcome-action-list">
                    <h2 class="welcome-section-title">Project actions</h2>
                    <button class="welcome-action-row primary-action" onclick={() => (palette = 'files')}>
                      <div class="welcome-action-label">
                        <Search size={18} />
                        <div>
                          <div>Open file</div>
                          <div class="welcome-action-desc">Quickly find and open any file in this workspace</div>
                        </div>
                      </div>
                      <kbd>{modKey} P</kbd>
                    </button>

                    <button class="welcome-action-row" onclick={() => startDialog('create-file')}>
                      <div class="welcome-action-label">
                        <FilePlus size={18} style="color: var(--muted);" />
                        <div>
                          <div>New file</div>
                          <div class="welcome-action-desc">Create a new source file in the project</div>
                        </div>
                      </div>
                    </button>

                    <button class="welcome-action-row" onclick={toggleTerminal}>
                      <div class="welcome-action-label">
                        <Terminal size={18} style="color: var(--muted);" />
                        <div>
                          <div>Open terminal</div>
                          <div class="welcome-action-desc">Launch a shell session in this project root</div>
                        </div>
                      </div>
                      <kbd>{modKey} `</kbd>
                    </button>

                    <div class="welcome-context-note">
                      <div style="font-weight: 500; margin-bottom: 4px; color: var(--text);">Run a supported file & Preview an HTML file</div>
                      <div>Open a supported code file (.ts, .rs, .py, .go, .c) to run it directly from the toolbar or editor, or open an HTML file to start live preview.</div>
                    </div>
                  </div>

                  <div class="welcome-shortcut-list">
                    <h2 class="welcome-section-title">Keyboard shortcuts</h2>
                    <div class="welcome-shortcut-item">
                      <span>Quick Open files</span>
                      <kbd>{modKey} P</kbd>
                    </div>
                    <div class="welcome-shortcut-item">
                      <span>Command palette</span>
                      <kbd>{modKey} Shift P</kbd>
                    </div>
                    <div class="welcome-shortcut-item">
                      <span>Save file</span>
                      <kbd>{modKey} S</kbd>
                    </div>
                    <div class="welcome-shortcut-item">
                      <span>Find and replace</span>
                      <kbd>{modKey} F</kbd>
                    </div>
                    <div class="welcome-shortcut-item">
                      <span>Toggle terminal</span>
                      <kbd>{modKey} `</kbd>
                    </div>
                  </div>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <!-- Live HTML Preview Panel (Section 4 & 5 design.md: Resizable divider on wide windows, adaptive on narrow) -->
      {#if tools.previewUrl}
        {#if !isSmallScreen}
          <button
            class="splitter-v"
            class:dragging={Boolean(previewDrag)}
            aria-label="Resize HTML preview panel"
            onpointerdown={e => {
              e.preventDefault();
              previewDrag = { x: e.clientX, width: previewWidth };
            }}
            onkeydown={onPreviewKeyResize}
          ></button>
        {/if}
        <div
          class="preview-frame"
          style:width={isSmallScreen ? '100%' : `${previewWidth}px`}
          role="region"
          aria-label="HTML live preview"
        >
          <div class="preview-toolbar">
            <div class="preview-title">
              <Globe size={14} />
              <span>{basename(tools.previewPath)}</span>
            </div>
            <div style="display: flex; align-items: center; gap: 2px;">
              <button
                class="icon-btn"
                aria-label="Refresh preview"
                title="Refresh preview"
                onclick={() => tools.previewVersion++}
              >
                <RotateCw size={13} />
              </button>
              <button
                class="icon-btn"
                aria-label="Close preview"
                title="Close preview"
                onclick={() => attempt(closePreview)}
              >
                <X size={14} />
              </button>
            </div>
          </div>
          <div class="preview-content">
            {#key tools.previewVersion}
              <iframe
                title="HTML live preview"
                src={tools.previewUrl}
                sandbox="allow-scripts allow-forms allow-same-origin"
              ></iframe>
            {/key}
          </div>
        </div>
      {/if}
    </div>

    <!-- Bottom Panel (Terminal & Output) -->
    <BottomPanel theme={settings.theme} />
    {/if}
  </section>
</main>

<!-- Status Bar: 24px high -->
<footer class="workspace-status-bar">
  <div class="status-left-group">
    {#if gitStore.isRepo}
      <button
        class="status-item-btn status-branch-pill"
        onclick={() => {
          sidebar = true;
          sidebarTab = 'git';
        }}
        title={`Git: ${gitStore.branch || 'HEAD'} (Click to view Source Control)`}
      >
        <GitBranch size={13} />
        <span>{gitStore.branch || 'HEAD'}</span>
        {#if gitStore.ahead > 0}
          <span class="status-git-ahead" title={`${gitStore.ahead} commit(s) ahead`}>↑{gitStore.ahead}</span>
        {/if}
        {#if gitStore.behind > 0}
          <span class="status-git-behind" title={`${gitStore.behind} commit(s) behind`}>↓{gitStore.behind}</span>
        {/if}
      </button>
    {/if}
    {#if s.busy}
      <span>Opening…</span>
    {/if}
    {#if s.tabs.some(t => t.dirty)}
      <span class="status-dirty-pill">• Unsaved changes</span>
    {/if}
  </div>
  <div class="status-right-group">
    {#if s.active}
      <span>Ln {s.line}, Col {s.column}</span>
      <button
        class="status-item-btn"
        class:active={settings.wordWrap}
        onclick={toggleWordWrap}
        title="Toggle Word Wrap (Alt+Z)"
      >
        Wrap: {settings.wordWrap ? 'On' : 'Off'}
      </button>
      <button
        class="status-item-btn"
        class:active={settings.autoSave}
        onclick={toggleAutoSave}
        title="Toggle Auto Save"
      >
        Auto Save: {settings.autoSave ? 'On' : 'Off'}
      </button>
      {#if zenMode}
        <button
          class="status-item-btn active"
          style="color: var(--warning);"
          onclick={toggleZenMode}
          title="Click to exit Zen Mode"
        >
          Zen Mode
        </button>
      {/if}
      <span>UTF-8</span>
      {#if isLargeFileDocument(s.active)}
        <span
          class="status-large-file-badge"
          style="color: var(--warning); font-weight: 500;"
          title="Large-file mode: syntax parsing disabled, undo history limited to 10 entries"
        >
          Large File
        </span>
      {:else}
        <span>{getLanguageLabel(s.active)}</span>
      {/if}
    {:else}
      <button
        class="status-item-btn"
        class:active={settings.wordWrap}
        onclick={toggleWordWrap}
        title="Toggle Word Wrap (Alt+Z)"
      >
        Wrap: {settings.wordWrap ? 'On' : 'Off'}
      </button>
      <button
        class="status-item-btn"
        class:active={settings.autoSave}
        onclick={toggleAutoSave}
        title="Toggle Auto Save"
      >
        Auto Save: {settings.autoSave ? 'On' : 'Off'}
      </button>
      {#if zenMode}
        <button
          class="status-item-btn active"
          style="color: var(--warning);"
          onclick={toggleZenMode}
          title="Click to exit Zen Mode"
        >
          Zen Mode
        </button>
      {/if}
      <span>UTF-8</span>
      <span>Nicle Workbench</span>
    {/if}
  </div>
</footer>

<!-- Modals & Overlays -->
{#if palette !== 'none'}
  <Palette
    mode={palette}
    {commands}
    onopen={path => {
      activeView = 'editor';
      void openFile(path);
    }}
    onclose={() => (palette = 'none')}
  />
{/if}

{#if preferences}
  <Preferences
    {settings}
    onchange={updateSettings}
    onclose={() => (preferences = false)}
  />
{/if}

{#if dialog !== 'none'}
  <Modal
    title={dialog === 'menu'
      ? basename(target?.path ?? '')
      : dialog === 'delete'
      ? 'Delete permanently?'
      : dialog === 'rename'
      ? 'Rename'
      : dialog === 'create-file'
      ? 'New file'
      : 'New folder'}
    onclose={() => {
      if (!pending) dialog = 'none';
    }}
  >
    {#if dialog === 'menu'}
      <div style="display: flex; flex-direction: column; gap: var(--space-2);">
        {#if target?.directory}
          <button
            class="btn-secondary"
            style="justify-content: flex-start;"
            onclick={() => startDialog('create-file', target)}
          >
            <FilePlus size={15} /> New file inside folder
          </button>
          <button
            class="btn-secondary"
            style="justify-content: flex-start;"
            onclick={() => startDialog('create-folder', target)}
          >
            <FolderPlus size={15} /> New folder inside folder
          </button>
        {/if}
        {#if target}
          <button
            class="btn-secondary"
            style="justify-content: flex-start;"
            onclick={() => {
              if (target) void api.revealInFileManager(target.path);
              dialog = 'none';
            }}
          >
            <FolderOpen size={15} /> Reveal in File Manager
          </button>
        {/if}
        <button
          class="btn-secondary"
          style="justify-content: flex-start;"
          onclick={() => startDialog('rename', target)}
        >
          Rename…
        </button>
        <button
          class="btn-danger"
          style="justify-content: flex-start;"
          onclick={() => startDialog('delete', target)}
        >
          Delete permanently…
        </button>
      </div>
    {:else}
      <form
        onsubmit={e => {
          e.preventDefault();
          void mutate();
        }}
      >
        {#if dialog === 'delete'}
          <p style="margin: 0 0 var(--space-4) 0; line-height: 1.5; color: var(--text);">
            Delete <strong>{target?.name}</strong>{target?.directory ? ' and all its contents' : ''}?
            This action cannot be undone.
          </p>
        {:else}
          <div class="form-field">
            <label for="modal-input-field">
              Name
            </label>
            <input
              id="modal-input-field"
              class="form-control"
              bind:value
              autocomplete="off"
              spellcheck="false"
              placeholder="e.g. index.ts"
            />
          </div>
        {/if}

        <div class="dialog-footer">
          <button
            type="button"
            class="btn-secondary"
            disabled={pending}
            onclick={() => (dialog = 'none')}
          >
            Cancel
          </button>
          <button
            type="submit"
            class={dialog === 'delete' ? 'btn-danger' : 'btn-primary'}
            disabled={pending}
          >
            {#if pending}
              Working…
            {:else if dialog === 'delete'}
              Delete permanently
            {:else if dialog === 'rename'}
              Rename
            {:else if dialog === 'create-file'}
              Create file
            {:else}
              Create folder
            {/if}
          </button>
        </div>
      </form>
    {/if}
  </Modal>
{/if}

<!-- Dirty File Confirmation Modal -->
{#if confirmAction}
  <Modal title="Unsaved changes" onclose={() => confirmAction?.('cancel')}>
    <p style="margin: 0 0 var(--space-4) 0; line-height: 1.5; color: var(--text);">
      Save changes to <strong>{confirmLabel}</strong> before continuing?
    </p>
    <div class="dialog-footer">
      <button class="btn-secondary" onclick={() => confirmAction?.('cancel')}>
        Cancel
      </button>
      <button class="btn-secondary" onclick={() => confirmAction?.('discard')}>
        Discard
      </button>
      <button class="btn-primary" onclick={() => confirmAction?.('save')}>
        Save
      </button>
    </div>
  </Modal>
{/if}

<!-- Severity Toast Notifications -->
{#if s.error}
  <div class="notification-toast" role="alert" aria-live="assertive">
    <AlertCircle size={18} class="toast-icon" />
    <span class="toast-msg">{s.error}</span>
    <button
      class="icon-btn"
      aria-label="Dismiss notification"
      title="Dismiss notification"
      onclick={() => (s.error = '')}
    >
      <X size={15} />
    </button>
  </div>
{/if}

<!-- Git Diff & Branch Modals -->
<DiffModal onopeninforeditor={(p) => { activeView = 'editor'; void openFile(p); }} />
<BranchModal />
