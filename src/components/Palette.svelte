<script lang="ts">
  import { onDestroy } from 'svelte';
  import Modal from './Modal.svelte';
  import { api, basename, parent } from '../lib/api';
  import { report } from '../lib/session.svelte';
  import { Search, File, Command as CommandIcon, RotateCw } from 'lucide-svelte';

  export type Command = { readonly label: string; readonly shortcut?: string; readonly action: () => void };

  let {
    mode,
    commands,
    onopen,
    onclose,
  }: {
    mode: 'files' | 'commands';
    commands: Command[];
    onopen: (path: string) => void;
    onclose: () => void;
  } = $props();

  let query = $state('');
  let files = $state<string[]>([]);
  let loading = $state(false);
  let searchError = $state('');
  let index = $state(0);
  let generation = 0;

  const filteredCommands = $derived(
    commands.filter(c => c.label.toLowerCase().includes(query.toLowerCase()))
  );
  const count = $derived(mode === 'files' ? files.length : filteredCommands.length);

  function executeSearch(q: string) {
    if (mode !== 'files') return;
    const id = ++generation;
    loading = true;
    searchError = '';
    api
      .search(q)
      .then(result => {
        if (id === generation) files = result;
      })
      .catch(e => {
        if (id === generation) {
          searchError = e instanceof Error ? e.message : String(e);
        }
        report(e);
      })
      .finally(() => {
        if (id === generation) loading = false;
      });
  }

  $effect(() => {
    const q = query;
    index = 0;
    if (mode !== 'files') return;
    const timer = setTimeout(() => executeSearch(q), 150);
    return () => {
      clearTimeout(timer);
      if (loading) {
        void api.cancelSearch().catch(() => {});
      }
    };
  });

  onDestroy(() => {
    void api.cancelSearch().catch(() => {});
  });

  function handleClose() {
    void api.cancelSearch().catch(() => {});
    onclose();
  }

  function choose(i: number) {
    void api.cancelSearch().catch(() => {});
    if (mode === 'files') {
      const path = files[i];
      if (path) {
        onclose();
        onopen(path);
      }
    } else {
      const cmd = filteredCommands[i];
      if (cmd) {
        onclose();
        cmd.action();
      }
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      index = (index + (e.key === 'ArrowDown' ? 1 : -1) + Math.max(count, 1)) % Math.max(count, 1);
      const targetEl = document.getElementById(`palette-result-${index}`);
      targetEl?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (count > 0) choose(index);
    }
  }
</script>

<Modal
  title={mode === 'files' ? 'Quick Open' : 'Command Palette'}
  variant="palette"
  onclose={handleClose}
>
  <div style="position: relative; display: flex; align-items: center;">
    <input
      class="palette-search-input"
      aria-label={mode === 'files' ? 'Search files by name' : 'Search commands'}
      placeholder={mode === 'files' ? 'Search files by name…' : 'Type a command…'}
      bind:value={query}
      onkeydown={handleKeydown}
      autocomplete="off"
      spellcheck="false"
    />
  </div>

  <div class="palette-results-list" role="listbox">
    {#if loading}
      <div style="padding: 16px; text-align: center; color: var(--muted); font-size: 13px;">
        Searching…
      </div>
    {:else if searchError}
      <div style="padding: 16px; text-align: center; color: var(--error); font-size: 13px; display: flex; align-items: center; justify-content: center; gap: 8px;">
        <span>Search failed: {searchError}</span>
        <button class="btn-secondary" style="font-size: 11px; padding: 2px 8px;" onclick={() => executeSearch(query)}>
          <RotateCw size={12} /> Retry
        </button>
      </div>
    {:else if count === 0}
      <div style="padding: 24px 16px; text-align: center; color: var(--muted); font-size: 13px;">
        {mode === 'files' ? 'No files match your search' : 'No commands match your search'}
      </div>
    {:else if mode === 'files'}
      {#each files as file, i (file)}
        <button
          id={`palette-result-${i}`}
          class="palette-row"
          class:chosen={i === index}
          role="option"
          aria-selected={i === index}
          onclick={() => choose(i)}
        >
          <div class="palette-file-info">
            <span class="palette-file-base">{basename(file)}</span>
            <span class="palette-file-path">{parent(file) || './'}</span>
          </div>
          <File size={15} style="color: var(--muted); flex-shrink: 0;" />
        </button>
      {/each}
    {:else}
      {#each filteredCommands as command, i (command.label)}
        <button
          id={`palette-result-${i}`}
          class="palette-row"
          class:chosen={i === index}
          role="option"
          aria-selected={i === index}
          onclick={() => choose(i)}
        >
          <span class="palette-cmd-label">{command.label}</span>
          {#if command.shortcut}
            <kbd>{command.shortcut}</kbd>
          {/if}
        </button>
      {/each}
    {/if}
  </div>

  <div class="palette-footer-hints">
    <span>Use ↑ ↓ to navigate, Enter to select</span>
    <span>Esc to dismiss</span>
  </div>
</Modal>
