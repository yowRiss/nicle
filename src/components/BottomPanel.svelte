<script lang="ts">
  import { X, Plus, Square, RotateCw, Trash2, Terminal as TerminalIcon, FileText } from 'lucide-svelte';
  import Terminal from '../terminal/Terminal.svelte';
  import { tools, closeTerminal, stopRunner, restartRunner } from '../lib/tools.svelte';
  import { session, attempt } from '../lib/session.svelte';

  let { theme }: { theme: 'dark' | 'light' } = $props();
  let height = $state(240); // 240px initial height per design.md
  let drag = $state<{ y: number; height: number }>();

  function move(e: PointerEvent) {
    if (!drag) return;
    const minHeight = 100;
    const maxHeight = Math.max(minHeight, window.innerHeight - 240); // Preserve at least 240px of editor height
    height = Math.max(minHeight, Math.min(maxHeight, drag.height + drag.y - e.clientY));
  }

  function keyResize(e: KeyboardEvent) {
    const minHeight = 100;
    const maxHeight = Math.max(minHeight, window.innerHeight - 240);
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      height = Math.min(maxHeight, height + 20);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      height = Math.max(minHeight, height - 20);
    }
  }
</script>

<svelte:window onpointermove={move} onpointerup={() => (drag = undefined)} />

<div class="bottom-tool-panel" class:hidden={!tools.panelVisible} style:height={`${height}px`}>
  <!-- Accessible keyboard-enabled resize splitter -->
  <button
    class="splitter-h"
    class:dragging={Boolean(drag)}
    aria-label="Resize bottom tool panel"
    onpointerdown={e => {
      e.preventDefault();
      drag = { y: e.clientY, height };
    }}
    onkeydown={keyResize}
  ></button>

  <!-- Shared panel heading: 36px high -->
  <div class="bottom-panel-heading">
    <button
      class="panel-tab-btn"
      class:active={tools.panel === 'terminal'}
      onclick={() => (tools.panel = 'terminal')}
      aria-label="Terminal panel"
    >
      <TerminalIcon size={14} />
      <span>Terminal</span>
    </button>

    <button
      class="panel-tab-btn"
      class:active={tools.panel === 'output'}
      onclick={() => (tools.panel = 'output')}
      aria-label="Output panel"
    >
      <FileText size={14} />
      <span>Output</span>
      {#if tools.running}
        <span class="status-badge running" title="Process is running">
          <span class="status-badge-dot"></span>
          Running
        </span>
      {/if}
    </button>

    <div class="spacer"></div>

    <div class="panel-controls">
      {#if tools.panel === 'terminal'}
        {#if tools.terminalMounted}
          <button
            class="icon-btn"
            aria-label="End terminal session"
            title="End terminal session (closes shell and buffers)"
            onclick={() => attempt(closeTerminal)}
          >
            <Trash2 size={14} />
          </button>
        {:else}
          <button
            class="btn-secondary"
            style="font-size: 11px; padding: 2px 8px; min-height: 24px;"
            disabled={!session.root}
            aria-label="Start terminal session"
            onclick={() => (tools.terminalMounted = true)}
          >
            <Plus size={13} />
            Start terminal
          </button>
        {/if}
      {:else}
        <button
          class="icon-btn"
          aria-label="Restart runner"
          title="Restart runner"
          disabled={!session.active}
          onclick={() => attempt(restartRunner)}
        >
          <RotateCw size={14} />
        </button>
        <button
          class="icon-btn"
          aria-label="Stop runner"
          title="Stop runner"
          disabled={!tools.running}
          onclick={() => attempt(stopRunner)}
        >
          <Square size={13} />
        </button>
        <button
          class="icon-btn"
          aria-label="Clear output"
          title="Clear output"
          disabled={!tools.output}
          onclick={() => (tools.output = '')}
        >
          <Trash2 size={14} />
        </button>
      {/if}

      <button
        class="icon-btn"
        aria-label="Hide panel"
        title="Hide panel (session preserved)"
        onclick={() => (tools.panelVisible = false)}
      >
        <X size={15} />
      </button>
    </div>
  </div>

  <div class="bottom-panel-body">
    <!-- Terminal Tab Content -->
    <div class="terminal-container" class:hidden={tools.panel !== 'terminal'} style="height: 100%; position: relative;">
      {#if tools.terminalMounted}
        <Terminal {theme} visible={tools.panelVisible && tools.panel === 'terminal'} />
      {:else}
        <div class="panel-empty-state">
          <span>{session.root ? 'No active terminal session' : 'Open a folder to start a terminal'}</span>
          {#if session.root}
            <button class="btn-primary" onclick={() => (tools.terminalMounted = true)}>
              <Plus size={14} />
              Start terminal
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Output Tab Content -->
    <div class="output-container" class:hidden={tools.panel !== 'output'}>
      {#if tools.output}
        <pre>{tools.output}</pre>
      {:else}
        <div class="panel-empty-state">
          <span>Run a file to see its output here.</span>
        </div>
      {/if}
    </div>
  </div>
</div>
