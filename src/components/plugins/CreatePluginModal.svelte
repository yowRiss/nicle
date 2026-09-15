<script lang="ts">
  import Modal from '../Modal.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    plugins,
    scaffoldPlugin,
    linkDevPlugin,
  } from '../../lib/plugins.svelte';
  import { session } from '../../lib/session.svelte';
  import { Terminal, Server, Folder, Check, AlertCircle, Sparkles } from 'lucide-svelte';

  interface Props {
    onclose: () => void;
  }

  const { onclose }: Props = $props();

  let name = $state('My Plugin');
  let slug = $state('my-plugin');
  let userEditedSlug = $state(false);
  let template = $state<'claude-code-bun' | 'dev-companion-bun' | 'ai-harness-agent'>('claude-code-bun');
  let targetDir = $state('');
  let autoLink = $state(true);
  let isSubmitting = $state(false);
  let formError = $state<string | null>(null);

  function toSlug(str: string): string {
    return str
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, '-')
      .replace(/-+/g, '-')
      .replace(/^-|-$/g, '');
  }

  // Update slug and default targetDir reactively when name changes (unless user manually touched slug)
  $effect(() => {
    if (!userEditedSlug) {
      slug = toSlug(name) || 'my-plugin';
    }
  });

  $effect(() => {
    const currentSlug = slug || 'my-plugin';
    if (!targetDir || targetDir.includes('/plugins/')) {
      const base = session.root || '';
      targetDir = base ? `${base}/plugins/${currentSlug}` : `./${currentSlug}`;
    }
  });

  async function handleBrowse() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Destination Folder for New Plugin',
      });
      if (typeof selected === 'string') {
        const s = slug || 'my-plugin';
        targetDir = `${selected}/${s}`;
      }
    } catch (err) {
      formError = err instanceof Error ? err.message : String(err);
    }
  }

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) {
      formError = 'Plugin name cannot be empty.';
      return;
    }
    if (!slug.trim()) {
      formError = 'Plugin ID slug cannot be empty.';
      return;
    }
    if (!targetDir.trim()) {
      formError = 'Target directory cannot be empty.';
      return;
    }

    isSubmitting = true;
    formError = null;

    try {
      await scaffoldPlugin(targetDir.trim(), template);
      if (autoLink) {
        const record = await linkDevPlugin(targetDir.trim());
        plugins.selectedId = record.id;
        plugins.activeTab = 'installed';
      }
      onclose();
    } catch (err) {
      formError = err instanceof Error ? err.message : String(err);
    } finally {
      isSubmitting = false;
    }
  }
</script>

<Modal title="Create New Plugin" onclose={onclose}>
  <form class="create-plugin-form" onsubmit={handleSubmit}>
    {#if formError}
      <div class="form-error-banner" role="alert">
        <AlertCircle size={16} />
        <span>{formError}</span>
      </div>
    {/if}

    <!-- Template Selector Cards -->
    <div class="form-group">
      <label class="group-label" for="template-selector">Select Plugin Architecture:</label>
      <div class="template-cards-grid" id="template-selector">
        <button
          type="button"
          class="template-card"
          class:selected={template === 'claude-code-bun'}
          onclick={() => (template = 'claude-code-bun')}
        >
          <div class="card-icon cli">
            <Terminal size={20} />
          </div>
          <div class="card-content">
            <div class="card-header">
              <span class="card-title">CLI Assistant</span>
              {#if template === 'claude-code-bun'}
                <span class="check-icon"><Check size={16} /></span>
              {/if}
            </div>
            <p class="card-desc">
              Interactive terminal tool running via Bun in Nicle's built-in PTY shell with Command Palette integration.
            </p>
            <span class="card-runtime-pill">Runtime: Bun · CLI</span>
          </div>
        </button>

        <button
          type="button"
          class="template-card"
          class:selected={template === 'dev-companion-bun'}
          onclick={() => (template = 'dev-companion-bun')}
        >
          <div class="card-icon service">
            <Server size={20} />
          </div>
          <div class="card-content">
            <div class="card-header">
              <span class="card-title">Companion Service</span>
              {#if template === 'dev-companion-bun'}
                <span class="check-icon"><Check size={16} /></span>
              {/if}
            </div>
            <p class="card-desc">
              Supervised background daemon with loopback port binding, /health probe, logs viewer, and web dashboard.
            </p>
            <span class="card-runtime-pill">Runtime: Bun · Daemon</span>
          </div>
        </button>

        <button
          type="button"
          class="template-card"
          class:selected={template === 'ai-harness-agent'}
          onclick={() => {
            template = 'ai-harness-agent';
            if (name === 'My Plugin') {
              name = 'AI Harness Agent';
            }
          }}
        >
          <div class="card-icon harness">
            <Sparkles size={20} />
          </div>
          <div class="card-content">
            <div class="card-header">
              <span class="card-title">AI Harness Agent</span>
              {#if template === 'ai-harness-agent'}
                <span class="check-icon"><Check size={16} /></span>
              {/if}
            </div>
            <p class="card-desc">
              Multi-agent task harness coordinating Agent A (Coder) and Agent B (Reviewer) for any LLM.
            </p>
            <span class="card-runtime-pill">Runtime: Node · Multi-Agent</span>
          </div>
        </button>
      </div>
    </div>

    <!-- Plugin Display Name -->
    <div class="form-field">
      <label for="plugin-name">Plugin Display Name</label>
      <input
        id="plugin-name"
        type="text"
        class="form-control"
        placeholder="e.g. Claude Code Assistant"
        bind:value={name}
        required
      />
    </div>

    <!-- Plugin Slug -->
    <div class="form-field">
      <label for="plugin-slug">Plugin Identifier (ID)</label>
      <input
        id="plugin-slug"
        type="text"
        class="form-control"
        placeholder="e.g. claude-code"
        bind:value={slug}
        oninput={() => (userEditedSlug = true)}
        required
      />
      <span class="field-hint">Unique lowercase slug used for manifest and settings.</span>
    </div>

    <!-- Destination Directory -->
    <div class="form-field">
      <label for="target-dir">Destination Directory</label>
      <div class="dir-input-wrapper">
        <input
          id="target-dir"
          type="text"
          class="form-control"
          bind:value={targetDir}
          placeholder="/path/to/plugin"
          required
        />
        <button
          type="button"
          class="btn-secondary browse-btn"
          onclick={handleBrowse}
          title="Browse directory"
        >
          <Folder size={14} />
          <span>Browse</span>
        </button>
      </div>
      <span class="field-hint">Folder where nicle-plugin.json and source files will be created.</span>
    </div>

    <!-- Auto-link Switch -->
    <div class="checkbox-row">
      <label class="checkbox-label">
        <input type="checkbox" bind:checked={autoLink} />
        <span>Link immediately into Nicle IDE for live local development ([DEV] mode)</span>
      </label>
    </div>

    <!-- Dialog Actions -->
    <div class="dialog-footer">
      <button type="button" class="btn-secondary" onclick={onclose} disabled={isSubmitting}>
        Cancel
      </button>
      <button type="submit" class="btn-primary" disabled={isSubmitting}>
        {#if isSubmitting}
          <span>Scaffolding…</span>
        {:else}
          <Sparkles size={14} />
          <span>Create Plugin</span>
        {/if}
      </button>
    </div>
  </form>
</Modal>

<style>
  .create-plugin-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .form-error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-control);
    background-color: var(--raised);
    border: 1px solid var(--error);
    border-left: 4px solid var(--error);
    color: var(--error);
    font-size: 13px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  .template-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 10px;
  }

  .template-card {
    display: flex;
    gap: 12px;
    padding: 12px;
    background-color: var(--canvas);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    text-align: left;
    cursor: pointer;
    transition: all 120ms ease;
    align-items: flex-start;
  }

  .template-card:hover {
    background-color: var(--hover);
    border-color: var(--control-border);
  }

  .template-card.selected {
    border-color: var(--accent);
    background-color: var(--raised);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .card-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-control);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background-color: var(--raised);
    border: 1px solid var(--border);
  }

  .card-icon.cli {
    color: var(--text);
  }

  .card-icon.service {
    color: var(--text);
  }

  .card-icon.harness {
    color: var(--accent);
  }

  .card-content {
    flex: 1;
    min-width: 0;
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 4px;
  }

  .card-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .check-icon {
    color: var(--accent);
    flex-shrink: 0;
  }

  .card-desc {
    font-size: 11px;
    line-height: 1.4;
    color: var(--muted);
    margin: 0 0 8px 0;
  }

  .card-runtime-pill {
    display: inline-block;
    font-size: 10px;
    font-family: var(--font-mono, monospace);
    background-color: rgba(255, 255, 255, 0.05);
    color: var(--muted);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  .dir-input-wrapper {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .browse-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 36px;
  }

  .field-hint {
    font-size: 11px;
    color: var(--muted);
    margin-top: 2px;
  }

  .checkbox-row {
    margin-top: 4px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }

  .checkbox-label input {
    cursor: pointer;
    accent-color: var(--accent);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
</style>
