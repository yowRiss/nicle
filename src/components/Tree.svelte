<script lang="ts">
  import { ChevronRight, ChevronDown, File, Folder, FolderOpen, MoreHorizontal, RotateCw } from 'lucide-svelte';
  import Tree from './Tree.svelte';
  import { api, type Entry } from '../lib/api';
  import { gitStore } from '../lib/git.svelte';

  let {
    path = '',
    depth = 0,
    revision,
    selected,
    onopen,
    onselect,
    onerror,
    onmenu,
    oncreatefile,
    oncreatefolder,
  }: {
    path?: string;
    depth?: number;
    revision: number;
    selected: string;
    onopen: (path: string) => void;
    onselect: (entry: Entry) => void;
    onerror: (e: unknown) => void;
    onmenu: (entry: Entry) => void;
    oncreatefile?: (folderPath: string) => void;
    oncreatefolder?: (folderPath: string) => void;
  } = $props();

  let entries = $state<Entry[]>([]);
  let expanded = $state<string[]>([]);
  let loading = $state(false);
  let errorMsg = $state('');
  let more = $state(false);
  let generation = 0;

  async function load(reset: boolean) {
    const id = ++generation;
    loading = true;
    errorMsg = '';
    try {
      const result = await api.list(path, reset ? 0 : entries.length);
      if (id !== generation) return;
      entries = reset ? result.entries : [...entries, ...result.entries];
      more = result.more;
    } catch (e: unknown) {
      if (id === generation) {
        errorMsg = e instanceof Error ? e.message : String(e);
      }
      onerror(e);
    } finally {
      if (id === generation) loading = false;
    }
  }

  $effect(() => {
    revision;
    void load(true);
  });

  function choose(entry: Entry) {
    onselect(entry);
    if (entry.directory) {
      expanded = expanded.includes(entry.path)
        ? expanded.filter(p => p !== entry.path)
        : [...expanded, entry.path];
    } else {
      onopen(entry.path);
    }
  }
</script>

{#each entries as entry (entry.path)}
  <div class="tree-row" class:selected={selected === entry.path} style:padding-left={`${12 + depth * 16}px`}>
    <button
      class="tree-entry"
      onclick={() => choose(entry)}
      title={entry.path}
      aria-expanded={entry.directory ? expanded.includes(entry.path) : undefined}
    >
      {#if entry.directory}
        {#if expanded.includes(entry.path)}
          <ChevronDown size={14} strokeWidth={2} />
          <FolderOpen size={16} strokeWidth={1.8} />
        {:else}
          <ChevronRight size={14} strokeWidth={2} />
          <Folder size={16} strokeWidth={1.8} />
        {/if}
      {:else}
        <span class="tree-indent-spacer"></span>
        <File size={16} strokeWidth={1.8} />
      {/if}
      <span class="entry-name">{entry.name}</span>
      {#if !entry.directory && gitStore.statusMap.has(entry.path)}
        {@const st = gitStore.statusMap.get(entry.path)}
        <span class={`tree-git-badge git-status-${st?.toLowerCase()}`}>{st}</span>
      {/if}
    </button>
    <button
      class="icon-btn tree-menu-btn"
      aria-label={`Actions for ${entry.name}`}
      title={`Actions for ${entry.name}`}
      onclick={(e) => {
        e.stopPropagation();
        onmenu(entry);
      }}
    >
      <MoreHorizontal size={14} strokeWidth={2} />
    </button>
  </div>
  {#if entry.directory && expanded.includes(entry.path)}
    <Tree
      path={entry.path}
      depth={depth + 1}
      {revision}
      {selected}
      {onopen}
      {onselect}
      {onerror}
      {onmenu}
      {oncreatefile}
      {oncreatefolder}
    />
  {/if}
{/each}

{#if loading}
  <div class="tree-status-msg" style:padding-left={`${16 + depth * 16}px`}>
    <span>Loading…</span>
  </div>
{:else if errorMsg}
  <div class="tree-status-msg" style:padding-left={`${16 + depth * 16}px`}>
    <span style="color: var(--error)">{errorMsg}</span>
    <button class="btn-secondary" style="font-size: 11px; padding: 2px 8px; min-height: 22px;" onclick={() => load(true)}>
      <RotateCw size={12} /> Retry
    </button>
  </div>
{:else if more}
  <div style:padding-left={`${16 + depth * 16}px`}>
    <button class="btn-secondary" style="font-size: 11px; margin: 4px 0;" onclick={() => load(false)}>
      Load more…
    </button>
  </div>
{:else if entries.length === 0}
  <div class="tree-status-msg" style:padding-left={`${16 + depth * 16}px`}>
    <span>Empty folder</span>
    {#if oncreatefile && oncreatefolder}
      <div class="tree-empty-actions">
        <button onclick={() => oncreatefile?.(path)}>+ File</button>
        <button onclick={() => oncreatefolder?.(path)}>+ Folder</button>
      </div>
    {/if}
  </div>
{/if}
