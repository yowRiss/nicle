import { listen, emit } from '@tauri-apps/api/event';
import { openWorkspacePath, openFile, removeTab, session } from './session.svelte';
import { tools, previewFile, closePreview, closeTerminal, cleanupTools } from './tools.svelte';

export function setupBenchmark() {
  void listen<{ step: string; arg?: string }>('benchmark-run', async event => {
    const payload = event.payload;
    try {
      if (payload.step === 'open-workspace') {
        if (payload.arg) {
          await openWorkspacePath(payload.arg, () => Promise.resolve(true), cleanupTools);
        }
      } else if (payload.step === 'open-file') {
        if (payload.arg) {
          await openFile(payload.arg);
        }
      } else if (payload.step === 'close-file') {
        if (payload.arg) {
          removeTab(payload.arg);
        }
      } else if (payload.step === 'close-all-tabs') {
        for (const tab of [...session.tabs]) {
          removeTab(tab.path);
        }
      } else if (payload.step === 'start-terminal') {
        tools.panel = 'terminal';
        tools.panelVisible = true;
        tools.terminalMounted = true;
      } else if (payload.step === 'hide-terminal') {
        tools.panelVisible = false;
      } else if (payload.step === 'end-terminal') {
        await closeTerminal();
      } else if (payload.step === 'start-preview') {
        if (session.active) {
          await previewFile();
        }
      } else if (payload.step === 'close-preview') {
        await closePreview();
      }
      await emit('benchmark-step-done', { step: payload.step, ok: true });
    } catch (e) {
      await emit('benchmark-step-done', { step: payload.step, ok: false, error: String(e) });
    }
  });

  void emit('benchmark-frontend-ready', {});
}
