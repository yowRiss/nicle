<script lang="ts">
  import { GitBranch, Plus, Check, X, Search, Globe } from 'lucide-svelte';
  import { gitStore, type GitBranchInfo } from '../../lib/git.svelte';
  import { report } from '../../lib/session.svelte';

  let filter = $state('');
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (gitStore.branchModal) {
      filter = '';
      selectedIndex = 0;
      setTimeout(() => inputEl?.focus(), 50);
    }
  });

  const filteredBranches = $derived(
    gitStore.branches.filter(b =>
      b.name.toLowerCase().includes(filter.trim().toLowerCase())
    )
  );

  const exactMatch = $derived(
    gitStore.branches.some(b => b.name.toLowerCase() === filter.trim().toLowerCase())
  );

  const canCreate = $derived(
    filter.trim().length > 0 && !exactMatch
  );

  const totalItems = $derived(
    filteredBranches.length + (canCreate ? 1 : 0)
  );

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      gitStore.closeBranchModal();
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (totalItems > 0) {
        selectedIndex = (selectedIndex + 1) % totalItems;
      }
      return;
    }

    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (totalItems > 0) {
        selectedIndex = (selectedIndex - 1 + totalItems) % totalItems;
      }
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      if (canCreate && selectedIndex === 0) {
        void handleCreateBranch();
      } else {
        const branchIndex = canCreate ? selectedIndex - 1 : selectedIndex;
        if (filteredBranches[branchIndex]) {
          void handleSelectBranch(filteredBranches[branchIndex]);
        }
      }
    }
  }

  async function handleSelectBranch(branch: GitBranchInfo) {
    if (branch.current) {
      gitStore.closeBranchModal();
      return;
    }
    try {
      await gitStore.checkoutBranch(branch.name);
    } catch (e) {
      report(e);
    }
  }

  async function handleCreateBranch() {
    const name = filter.trim();
    if (!name) return;
    try {
      await gitStore.createBranch(name);
    } catch (e) {
      report(e);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if gitStore.branchModal}
  <div
    class="branch-modal-backdrop"
    onclick={() => gitStore.closeBranchModal()}
    aria-hidden="true"
  ></div>

  <div class="branch-modal-dialog" role="dialog" aria-modal="true" aria-label="Switch or Create Branch">
    <div class="branch-modal-header">
      <span class="search-icon"><Search size={15} /></span>
      <input
        bind:this={inputEl}
        bind:value={filter}
        type="text"
        class="branch-input"
        placeholder="Select a branch to checkout or type to create…"
      />
      <button
        class="icon-btn"
        aria-label="Close branch modal"
        onclick={() => gitStore.closeBranchModal()}
      >
        <X size={15} />
      </button>
    </div>

    <div class="branch-modal-list">
      {#if canCreate}
        <button
          class="branch-item-row create-row"
          class:selected={selectedIndex === 0}
          onclick={handleCreateBranch}
          onmouseenter={() => (selectedIndex = 0)}
        >
          <span class="branch-icon"><Plus size={14} /></span>
          <span class="branch-label">Create new branch <strong>"{filter.trim()}"</strong></span>
        </button>
      {/if}

      {#each filteredBranches as branch, i (branch.name)}
        {@const itemIndex = canCreate ? i + 1 : i}
        <button
          class="branch-item-row"
          class:selected={selectedIndex === itemIndex}
          class:current={branch.current}
          onclick={() => void handleSelectBranch(branch)}
          onmouseenter={() => (selectedIndex = itemIndex)}
        >
          {#if branch.isRemote}
            <span class="branch-icon"><Globe size={14} /></span>
          {:else}
            <span class="branch-icon"><GitBranch size={14} /></span>
          {/if}

          <span class="branch-label">{branch.name}</span>

          {#if branch.current}
            <span class="current-indicator">
              <Check size={14} /> Current
            </span>
          {:else if branch.isRemote}
            <span class="remote-tag">remote</span>
          {/if}
        </button>
      {/each}

      {#if filteredBranches.length === 0 && !canCreate}
        <div class="no-branches">No matching branches found</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .branch-modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 160;
  }

  .branch-modal-dialog {
    position: fixed;
    top: 15%;
    left: 50%;
    transform: translateX(-50%);
    width: min(90vw, 540px);
    max-height: 60vh;
    background: var(--raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-dialog);
    box-shadow: 0 16px 40px var(--shadow);
    display: flex;
    flex-direction: column;
    z-index: 161;
    overflow: hidden;
  }

  .branch-modal-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--raised);
    border-bottom: 1px solid var(--border);
  }

  .search-icon {
    color: var(--muted);
    flex-shrink: 0;
  }

  .branch-input {
    flex: 1;
    background: none;
    border: none;
    color: var(--text);
    font-size: 13px;
    outline: none;
    font-family: var(--font-sans);
  }

  .branch-input::placeholder {
    color: var(--text-subtle);
  }

  .branch-modal-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
    display: flex;
    flex-direction: column;
  }

  .branch-item-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    background: none;
    border: none;
    color: var(--text);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.1s;
  }

  .branch-item-row.selected {
    background: var(--hover);
  }

  .branch-item-row.create-row {
    color: var(--syntax-keyword, #569CD6);
    border-bottom: 1px solid var(--border);
    margin-bottom: 2px;
  }

  .branch-icon {
    color: var(--muted);
    flex-shrink: 0;
  }

  .create-row .branch-icon {
    color: var(--syntax-keyword, #569CD6);
  }

  .branch-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 400;
  }

  .current-indicator {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--success);
    font-size: 11px;
    font-weight: 500;
  }

  .remote-tag {
    color: var(--text-subtle);
    font-size: 11px;
    background: var(--canvas);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .no-branches {
    padding: 20px;
    text-align: center;
    color: var(--muted);
    font-size: 12px;
  }
</style>
