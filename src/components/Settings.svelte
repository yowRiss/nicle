<script lang="ts">
  import Modal from './Modal.svelte';
  import type { EditorSettings } from '../editor/documents';

  let {
    settings,
    onchange,
    onclose,
  }: {
    settings: EditorSettings;
    onchange: (s: EditorSettings) => void;
    onclose: () => void;
  } = $props();
</script>

<Modal title="Settings" {onclose}>
  <div style="display: flex; flex-direction: column; gap: var(--space-6);">
    <!-- Appearance Section -->
    <div class="settings-section">
      <h3 class="settings-section-title">Appearance</h3>
      <div class="form-field">
        <label for="setting-theme">Color theme</label>
        <select
          id="setting-theme"
          class="form-control"
          value={settings.theme}
          onchange={e =>
            onchange({
              ...settings,
              theme: e.currentTarget.value === 'light' ? 'light' : 'dark',
            })}
        >
          <option value="dark">Dark (Nicle default)</option>
          <option value="light">Light</option>
        </select>
      </div>
    </div>

    <!-- Editor Section -->
    <div class="settings-section" style="border-bottom: none; padding-bottom: 0;">
      <h3 class="settings-section-title">Editor</h3>

      <div class="form-field">
        <label for="setting-font-size">Font size</label>
        <select
          id="setting-font-size"
          class="form-control"
          value={settings.fontSize}
          onchange={e =>
            onchange({
              ...settings,
              fontSize: Number(e.currentTarget.value),
            })}
        >
          {#each [11, 12, 13, 14, 15, 16, 18, 20, 24] as size}
            <option value={size}>{size} px</option>
          {/each}
        </select>
      </div>

      <div class="form-field">
        <label for="setting-tab-size">Indentation size</label>
        <select
          id="setting-tab-size"
          class="form-control"
          value={settings.tabSize}
          onchange={e =>
            onchange({
              ...settings,
              tabSize: Number(e.currentTarget.value),
            })}
        >
          <option value={2}>2 spaces</option>
          <option value={4}>4 spaces</option>
          <option value={8}>8 spaces</option>
        </select>
      </div>

      <label class="checkbox-row" for="setting-word-wrap">
        <input
          id="setting-word-wrap"
          type="checkbox"
          checked={settings.wordWrap}
          onchange={e =>
            onchange({
              ...settings,
              wordWrap: e.currentTarget.checked,
            })}
        />
        <span>Wrap long lines</span>
      </label>

      <label class="checkbox-row" for="setting-auto-save">
        <input
          id="setting-auto-save"
          type="checkbox"
          checked={settings.autoSave}
          onchange={e =>
            onchange({
              ...settings,
              autoSave: e.currentTarget.checked,
            })}
        />
        <span>Auto save changes (1s delay & tab switch)</span>
      </label>
    </div>

    <div style="font-size: 12px; color: var(--muted); border-top: 1px solid var(--border); padding-top: var(--space-3);">
      Preferences are saved on this device.
    </div>
  </div>
</Modal>
