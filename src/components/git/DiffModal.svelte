<script lang="ts">
  import { X, Plus, Minus, ExternalLink } from 'lucide-svelte';
  import { gitStore } from '../../lib/git.svelte';
  import { openFile } from '../../lib/session.svelte';
  import { basename } from '../../lib/api';

  interface Props {
    onopeninforeditor?: (path: string) => void;
  }

  let { onopeninforeditor }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      gitStore.closeDiff();
    }
  }

  async function handleToggleStage() {
    const path = gitStore.diffModal.path;
    const isStaged = gitStore.diffModal.staged;
    if (isStaged) {
      await gitStore.unstage([path]);
      await gitStore.openDiff(path, false);
    } else {
      await gitStore.stage([path]);
      await gitStore.openDiff(path, true);
    }
  }

  function handleOpenInEditor() {
    const path = gitStore.diffModal.path;
    gitStore.closeDiff();
    if (onopeninforeditor) {
      onopeninforeditor(path);
    } else {
      void openFile(path);
    }
  }

  const MAX_DIFF_LINES = 2000;

  function parseDiff(diff: string) {
    const raw = diff.split('\n');
    const isTruncated = raw.length > MAX_DIFF_LINES;
    const sliced = raw.slice(0, MAX_DIFF_LINES);
    const parsed = sliced.map((line, idx) => {
      let type: 'header' | 'chunk' | 'added' | 'deleted' | 'normal' = 'normal';
      if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('diff ') || line.startsWith('index ')) {
        type = 'header';
      } else if (line.startsWith('@@')) {
        type = 'chunk';
      } else if (line.startsWith('+')) {
        type = 'added';
      } else if (line.startsWith('-')) {
        type = 'deleted';
      }
      return { id: idx, line, type };
    });
    return { lines: parsed, isTruncated, totalLines: raw.length };
  }

  const diffData = $derived(parseDiff(gitStore.diffModal.diff));
  const lines = $derived(diffData.lines);
  const isTruncated = $derived(diffData.isTruncated);
</script>

<svelte:window onkeydown={handleKeydown} />

{#if gitStore.diffModal.open}
  <!-- Backdrop -->
  <div
    class="diff-modal-backdrop"
    onclick={() => gitStore.closeDiff()}
    aria-hidden="true"
  ></div>

  <div class="diff-modal-dialog" role="dialog" aria-modal="true" aria-label="Diff Viewer">
    <!-- Header -->
    <div class="diff-modal-header">
      <div class="diff-title-area">
        <span class="diff-file-name">{basename(gitStore.diffModal.path)}</span>
        <span class="diff-path">{gitStore.diffModal.path}</span>
        <span class={`diff-badge ${gitStore.diffModal.staged ? 'staged' : 'working'}`}>
          {gitStore.diffModal.staged ? 'Staged' : 'Working Tree'}
        </span>
      </div>
      <div class="diff-actions">
        <button
          class="btn-secondary diff-action-btn"
          onclick={handleToggleStage}
        >
          {#if gitStore.diffModal.staged}
            <Minus size={13} /> Unstage
          {:else}
            <Plus size={13} /> Stage
          {/if}
        </button>
        <button
          class="btn-secondary diff-action-btn"
          onclick={handleOpenInEditor}
        >
          <ExternalLink size={13} /> Open in Editor
        </button>
        <button
          class="icon-btn"
          aria-label="Close diff"
          onclick={() => gitStore.closeDiff()}
        >
          <X size={15} />
        </button>
      </div>
    </div>

    <!-- Body -->
    <div class="diff-modal-body">
      {#if isTruncated}
        <div class="diff-truncated-banner">
          Diff preview truncated to 2,000 lines ({diffData.totalLines} lines total). Open in editor to view complete file.
        </div>
      {/if}
      {#if lines.length === 0 || (lines.length === 1 && lines[0]?.line.trim() === '')}
        <div class="diff-empty">No differences to display.</div>
      {:else}
        <div class="diff-code-container">
          {#each lines as { id, line, type } (id)}
            <div class={`diff-line diff-line-${type}`}>
              <span class="diff-prefix">
                {#if type === 'added'}+{/if}
                {#if type === 'deleted'}-{/if}
                {#if type !== 'added' && type !== 'deleted'}&nbsp;{/if}
              </span>
              <pre class="diff-text">{line.startsWith('+') || line.startsWith('-') ? line.slice(1) : line}</pre>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="diff-modal-footer">
      <div class="diff-stats">
        <span>{lines.filter(l => l.type === 'added').length} added</span>
        <span>•</span>
        <span>{lines.filter(l => l.type === 'deleted').length} removed</span>
      </div>
      <button class="btn-secondary" onclick={() => gitStore.closeDiff()}>
        Close
      </button>
    </div>
  </div>
{/if}

<style>
  .diff-modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 150;
  }

  .diff-modal-dialog {
    position: fixed;
    top: 5%;
    left: 50%;
    transform: translateX(-50%);
    width: min(92vw, 1000px);
    height: 85vh;
    background: var(--canvas);
    border: 1px solid var(--border);
    border-radius: var(--radius-dialog);
    box-shadow: 0 16px 40px var(--shadow);
    display: flex;
    flex-direction: column;
    z-index: 151;
    overflow: hidden;
  }

  .diff-modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--raised);
    border-bottom: 1px solid var(--border);
    gap: 12px;
  }

  .diff-title-area {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
  }

  .diff-file-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .diff-path {
    font-size: 12px;
    color: var(--muted);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 4px;
  }

  .diff-badge.staged {
    background: rgba(134, 212, 160, 0.2);
    color: var(--success);
  }

  .diff-badge.working {
    background: rgba(232, 189, 117, 0.2);
    color: var(--warning);
  }

  .diff-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .diff-action-btn {
    font-size: 12px;
    padding: 3px 8px;
    min-height: 26px;
  }

  .diff-modal-body {
    flex: 1;
    overflow: auto;
    background: var(--canvas);
    padding: 8px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 19px;
  }

  .diff-empty {
    padding: 40px;
    text-align: center;
    color: var(--muted);
  }

  .diff-truncated-banner {
    padding: 8px 16px;
    background: var(--surface-subtle);
    border-bottom: 1px solid var(--border);
    color: var(--warning);
    font-size: var(--font-size-sm);
    font-weight: 500;
  }

  .diff-code-container {
    display: flex;
    flex-direction: column;
    min-width: 100%;
    width: max-content;
  }

  .diff-line {
    display: flex;
    align-items: baseline;
    padding: 0 12px;
    width: 100%;
    box-sizing: border-box;
  }

  .diff-prefix {
    width: 16px;
    flex-shrink: 0;
    user-select: none;
    font-weight: 600;
  }

  .diff-text {
    margin: 0;
    font-family: inherit;
    font-size: inherit;
    white-space: pre;
    color: var(--text);
  }

  .diff-line-header {
    color: var(--muted);
    background: rgba(255, 255, 255, 0.03);
  }

  .diff-line-header .diff-text {
    color: var(--muted);
  }

  .diff-line-chunk {
    color: var(--syntax-keyword, #569CD6);
    background: rgba(86, 156, 214, 0.08);
  }

  .diff-line-chunk .diff-text {
    color: var(--syntax-keyword, #569CD6);
  }

  .diff-line-added {
    background: rgba(134, 212, 160, 0.12);
  }

  .diff-line-added .diff-prefix,
  .diff-line-added .diff-text {
    color: var(--success);
  }

  .diff-line-deleted {
    background: rgba(241, 141, 134, 0.12);
  }

  .diff-line-deleted .diff-prefix,
  .diff-line-deleted .diff-text {
    color: var(--error);
  }

  .diff-modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background: var(--raised);
    border-top: 1px solid var(--border);
  }

  .diff-stats {
    font-size: 12px;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
