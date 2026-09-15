<script lang="ts">
  import {
    GitBranch,
    RefreshCw,
    Plus,
    Minus,
    Undo2,
    Trash2,
    CheckCircle2,
    ChevronDown,
    ChevronRight,
    ArrowUp,
    ArrowDown,
    FileText,
    FolderGit2,
  } from 'lucide-svelte';
  import { gitStore, type GitFileChange } from '../../lib/git.svelte';
  import { session, report } from '../../lib/session.svelte';
  import { basename, parent } from '../../lib/api';

  let commitMessage = $state('');
  let isSubmitting = $state(false);
  let stagedExpanded = $state(true);
  let changesExpanded = $state(true);

  async function handleCommit() {
    const msg = commitMessage.trim();
    if (!msg || isSubmitting) return;

    isSubmitting = true;
    try {
      if (gitStore.staged.length === 0) {
        // Automatically stage all changes if nothing staged
        await gitStore.stage([]);
      }
      await gitStore.commit(msg);
      commitMessage = '';
    } catch (e) {
      report(e);
    } finally {
      isSubmitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      void handleCommit();
    }
  }

  async function handleStage(item: GitFileChange) {
    try {
      await gitStore.stage([item.path]);
    } catch (e) {
      report(e);
    }
  }

  async function handleUnstage(item: GitFileChange) {
    try {
      await gitStore.unstage([item.path]);
    } catch (e) {
      report(e);
    }
  }

  async function handleDiscard(item: GitFileChange) {
    if (!window.confirm(`Are you sure you want to discard changes in '${item.path}'? This cannot be undone.`)) {
      return;
    }
    try {
      await gitStore.discard([item.path]);
    } catch (e) {
      report(e);
    }
  }

  async function handleStageAll() {
    try {
      await gitStore.stage([]);
    } catch (e) {
      report(e);
    }
  }

  async function handleUnstageAll() {
    try {
      await gitStore.unstage([]);
    } catch (e) {
      report(e);
    }
  }

  async function handleDiscardAll() {
    if (!window.confirm('Are you sure you want to discard ALL uncommitted changes? This cannot be undone.')) {
      return;
    }
    try {
      await gitStore.discard([]);
    } catch (e) {
      report(e);
    }
  }

  async function handleSync() {
    try {
      if (gitStore.behind > 0) {
        await gitStore.pull();
      }
      if (gitStore.ahead > 0) {
        await gitStore.push();
      }
      await gitStore.refresh();
    } catch (e) {
      report(e);
    }
  }

  function getStatusLabel(status: string): string {
    switch (status) {
      case 'added':
        return 'A';
      case 'deleted':
        return 'D';
      case 'modified':
        return 'M';
      case 'renamed':
        return 'R';
      case 'untracked':
        return 'U';
      default:
        return 'M';
    }
  }

  function getStatusClass(status: string): string {
    switch (status) {
      case 'added':
      case 'untracked':
        return 'status-added';
      case 'deleted':
        return 'status-deleted';
      case 'modified':
        return 'status-modified';
      case 'renamed':
        return 'status-renamed';
      default:
        return 'status-modified';
    }
  }

  const allChanges = $derived([...gitStore.unstaged, ...gitStore.untracked]);
</script>

<div class="git-panel" role="region" aria-label="Source Control Panel">
  {#if !gitStore.gitAvailable}
    <div class="git-empty-state">
      <FolderGit2 size={36} style="color: var(--danger, #f87171); opacity: 0.8; margin-bottom: 8px;" />
      <h3 class="git-empty-title">Git Not Found</h3>
      <p class="git-empty-desc">
        Git is not installed or not found in system PATH. Install Git to enable source control features.
      </p>
      <button class="btn-primary" onclick={() => void gitStore.refresh()}>
        <RefreshCw size={14} class={gitStore.loading ? 'spin' : ''} />
        Check Again
      </button>
    </div>
  {:else if !gitStore.isRepo}
    <div class="git-empty-state">
      <FolderGit2 size={36} style="color: var(--muted); opacity: 0.7; margin-bottom: 8px;" />
      <h3 class="git-empty-title">No Git Repository</h3>
      <p class="git-empty-desc">
        The current workspace is not a Git repository. Initialize Git to begin tracking version changes.
      </p>
      <button class="btn-primary" onclick={() => void gitStore.initRepo()}>
        <GitBranch size={14} />
        Initialize Repository
      </button>
    </div>
  {:else}
    <!-- Header: Branch info & Sync actions -->
    <div class="git-header">
      <button
        class="branch-pill-btn"
        title="Switch or create branch"
        onclick={() => void gitStore.openBranchModal()}
      >
        <GitBranch size={13} style="flex-shrink: 0;" />
        <span class="branch-name">{gitStore.branch || 'HEAD'}</span>
        {#if gitStore.ahead > 0}
          <span class="badge badge-ahead" title={`${gitStore.ahead} commit(s) ahead`}>
            <ArrowUp size={10} />{gitStore.ahead}
          </span>
        {/if}
        {#if gitStore.behind > 0}
          <span class="badge badge-behind" title={`${gitStore.behind} commit(s) behind`}>
            <ArrowDown size={10} />{gitStore.behind}
          </span>
        {/if}
      </button>

      <div class="git-header-actions">
        {#if gitStore.ahead > 0 || gitStore.behind > 0}
          <button
            class="icon-btn"
            title={`Sync changes (${gitStore.behind} pull, ${gitStore.ahead} push)`}
            onclick={() => void handleSync()}
          >
            <RefreshCw size={13} class={gitStore.loading ? 'spin' : ''} />
          </button>
        {/if}
        <button
          class="icon-btn"
          title="Refresh Git Status"
          onclick={() => void gitStore.refresh()}
        >
          <RefreshCw size={13} class={gitStore.loading ? 'spin' : ''} />
        </button>
      </div>
    </div>

    <!-- Commit Input Section -->
    <div class="git-commit-box">
      <textarea
        class="commit-textarea"
        placeholder="Message (Ctrl+Enter to commit)"
        rows={3}
        bind:value={commitMessage}
        onkeydown={handleKeydown}
      ></textarea>
      <button
        class="btn-primary commit-btn"
        disabled={isSubmitting || !commitMessage.trim() || gitStore.totalChanges === 0}
        onclick={() => void handleCommit()}
      >
        Commit
        {#if gitStore.staged.length > 0}
          ({gitStore.staged.length})
        {:else if allChanges.length > 0}
          (All {allChanges.length})
        {/if}
      </button>
    </div>

    <!-- Changes Sections -->
    <div class="git-lists-container">
      {#if gitStore.totalChanges === 0}
        <div class="git-clean-state">
          <CheckCircle2 size={24} style="color: var(--success); margin-bottom: 6px;" />
          <span>Working tree clean</span>
        </div>
      {/if}

      <!-- Staged Changes Section -->
      {#if gitStore.staged.length > 0}
        <div class="git-section">
          <div class="git-section-header">
            <button
              class="git-section-toggle"
              onclick={() => (stagedExpanded = !stagedExpanded)}
            >
              {#if stagedExpanded}
                <ChevronDown size={13} />
              {:else}
                <ChevronRight size={13} />
              {/if}
              <span class="git-section-title">STAGED CHANGES</span>
              <span class="git-count-badge">{gitStore.staged.length}</span>
            </button>
            <div class="git-section-actions">
              <button
                class="icon-btn"
                title="Unstage All Changes"
                onclick={() => void handleUnstageAll()}
              >
                <Minus size={13} />
              </button>
            </div>
          </div>

          {#if stagedExpanded}
            <div class="git-items-list">
              {#each gitStore.staged as item (item.path)}
                <div class="git-file-row">
                  <button
                    class="git-file-info"
                    onclick={() => void gitStore.openDiff(item.path, true)}
                    title={`View staged diff: ${item.path}`}
                  >
                    <FileText size={14} class="git-file-icon" />
                    <span class="git-file-name">{basename(item.path)}</span>
                    {#if parent(item.path)}
                      <span class="git-file-dir">{parent(item.path)}</span>
                    {/if}
                  </button>
                  <div class="git-row-actions">
                    <span class={`git-status-badge ${getStatusClass(item.status)}`}>
                      {getStatusLabel(item.status)}
                    </span>
                    <button
                      class="icon-btn row-btn"
                      title="Unstage change"
                      onclick={() => void handleUnstage(item)}
                    >
                      <Minus size={13} />
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Unstaged / Untracked Changes Section -->
      {#if allChanges.length > 0}
        <div class="git-section">
          <div class="git-section-header">
            <button
              class="git-section-toggle"
              onclick={() => (changesExpanded = !changesExpanded)}
            >
              {#if changesExpanded}
                <ChevronDown size={13} />
              {:else}
                <ChevronRight size={13} />
              {/if}
              <span class="git-section-title">CHANGES</span>
              <span class="git-count-badge">{allChanges.length}</span>
            </button>
            <div class="git-section-actions">
              <button
                class="icon-btn"
                title="Discard All Changes"
                onclick={() => void handleDiscardAll()}
              >
                <Undo2 size={13} />
              </button>
              <button
                class="icon-btn"
                title="Stage All Changes"
                onclick={() => void handleStageAll()}
              >
                <Plus size={13} />
              </button>
            </div>
          </div>

          {#if changesExpanded}
            <div class="git-items-list">
              {#each allChanges as item (item.path)}
                <div class="git-file-row">
                  <button
                    class="git-file-info"
                    onclick={() => void gitStore.openDiff(item.path, false)}
                    title={`View diff: ${item.path}`}
                  >
                    <FileText size={14} class="git-file-icon" />
                    <span class="git-file-name">{basename(item.path)}</span>
                    {#if parent(item.path)}
                      <span class="git-file-dir">{parent(item.path)}</span>
                    {/if}
                  </button>
                  <div class="git-row-actions">
                    <span class={`git-status-badge ${getStatusClass(item.status)}`}>
                      {getStatusLabel(item.status)}
                    </span>
                    <button
                      class="icon-btn row-btn"
                      title={item.status === 'untracked' ? 'Delete untracked file' : 'Discard changes'}
                      onclick={() => void handleDiscard(item)}
                    >
                      {#if item.status === 'untracked'}
                        <Trash2 size={13} />
                      {:else}
                        <Undo2 size={13} />
                      {/if}
                    </button>
                    <button
                      class="icon-btn row-btn"
                      title="Stage change"
                      onclick={() => void handleStage(item)}
                    >
                      <Plus size={13} />
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .git-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: var(--sidebar);
    color: var(--text);
    overflow: hidden;
  }

  .git-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
  }

  .branch-pill-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    padding: 3px 8px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    max-width: 180px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background 0.15s, border-color 0.15s;
  }

  .branch-pill-btn:hover {
    background: var(--hover);
    border-color: var(--control-border);
  }

  .branch-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 600;
  }

  .badge-ahead {
    background: rgba(134, 212, 160, 0.2);
    color: var(--success);
  }

  .badge-behind {
    background: rgba(232, 189, 117, 0.2);
    color: var(--warning);
  }

  .git-header-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .git-commit-box {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--border);
  }

  .commit-textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--canvas);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    padding: 6px 8px;
    font-family: var(--font-sans);
    font-size: 12px;
    line-height: 16px;
    resize: vertical;
    outline: none;
  }

  .commit-textarea:focus {
    border-color: var(--control-border);
    box-shadow: 0 0 0 1px var(--control-border);
  }

  .commit-btn {
    width: 100%;
    justify-content: center;
    font-size: 12px;
    height: 28px;
  }

  .git-lists-container {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .git-clean-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 16px;
    color: var(--muted);
    font-size: 12px;
    gap: 4px;
  }

  .git-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 40px 20px;
    color: var(--muted);
  }

  .git-empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    margin: 0 0 6px 0;
  }

  .git-empty-desc {
    font-size: 12px;
    line-height: 18px;
    margin: 0 0 16px 0;
    color: var(--muted);
  }

  .git-section {
    display: flex;
    flex-direction: column;
  }

  .git-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px 4px 6px;
    background: var(--sidebar);
    user-select: none;
  }

  .git-section-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    color: var(--text-subtle);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-control);
  }

  .git-section-toggle:hover {
    color: var(--text);
  }

  .git-section-title {
    letter-spacing: 0.5px;
  }

  .git-count-badge {
    display: inline-block;
    background: var(--raised);
    color: var(--muted);
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 8px;
    margin-left: 2px;
  }

  .git-section-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .git-items-list {
    display: flex;
    flex-direction: column;
  }

  .git-file-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 8px 2px 14px;
    height: 26px;
    cursor: pointer;
    border-radius: var(--radius-control);
  }

  .git-file-row:hover {
    background: var(--hover);
  }

  .git-file-info {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: none;
    color: var(--text);
    font-size: 12px;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
    flex: 1;
    padding: 0;
  }

  .git-file-icon {
    flex-shrink: 0;
    color: var(--muted);
  }

  .git-file-name {
    font-weight: 400;
    white-space: nowrap;
  }

  .git-file-dir {
    color: var(--text-subtle);
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .git-row-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .git-status-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    min-width: 14px;
    text-align: center;
  }

  .status-added {
    color: var(--success);
  }

  .status-deleted {
    color: var(--error);
  }

  .status-modified {
    color: var(--warning);
  }

  .status-renamed {
    color: var(--syntax-keyword, #569CD6);
  }

  .row-btn {
    opacity: 0;
    transition: opacity 0.15s;
    height: 20px;
    width: 20px;
    padding: 0;
  }

  .git-file-row:hover .row-btn,
  .row-btn:focus-visible {
    opacity: 1;
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
