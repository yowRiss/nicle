import { open } from '@tauri-apps/plugin-dialog';
import { api, basename, errorText, type Entry } from './api';
import { openDocument, activateDocument, closeDocument, saveDocument, renameDocument, resetEditorView, type Tab } from '../editor/documents';
export const session = $state<{root: string; tabs: Tab[]; active: string; selected: Entry | undefined; revision: number; error: string; line: number; column: number; busy: boolean}>({ root: '', tabs: [], active: '', selected: undefined, revision: 0, error: '', line: 1, column: 1, busy: false });
const autoSaveTimers = new Map<string, ReturnType<typeof setTimeout>>();
let autoSaveEnabled = false;

export function setAutoSave(enabled: boolean) {
  autoSaveEnabled = enabled;
  if (enabled) {
    void saveAll();
  } else {
    for (const timer of autoSaveTimers.values()) clearTimeout(timer);
    autoSaveTimers.clear();
  }
}

export function isAutoSaveEnabled(): boolean {
  return autoSaveEnabled;
}

export function report(e: unknown) { session.error = errorText(e); }
export async function attempt(action: () => Promise<unknown>) { try { await action(); } catch (e) { report(e); } }
export function changed(path: string, dirty: boolean, line: number, column: number) {
  const tab = session.tabs.find(t => t.path === path);
  if (tab && tab.dirty !== dirty) tab.dirty = dirty;
  if (path === session.active && line) { session.line = line; session.column = column; }

  if (autoSaveEnabled) {
    if (dirty) {
      if (autoSaveTimers.has(path)) {
        clearTimeout(autoSaveTimers.get(path));
      }
      autoSaveTimers.set(
        path,
        setTimeout(() => {
          autoSaveTimers.delete(path);
          const currentTab = session.tabs.find(t => t.path === path);
          if (currentTab?.dirty) {
            void saveDocument(path);
          }
        }, 1000)
      );
    } else if (autoSaveTimers.has(path)) {
      clearTimeout(autoSaveTimers.get(path));
      autoSaveTimers.delete(path);
    }
  }
}
export async function openFile(path: string) {
  if (session.busy) return;
  session.busy = true;
  try {
    await openDocument(path);
    if (!session.tabs.some(t => t.path === path)) session.tabs.push({path, dirty: false});
    session.active = path;
  }
  catch (e) { report(e); } finally { session.busy = false; }
}
export function activate(path: string) {
  if (autoSaveEnabled && session.active && session.active !== path) {
    const prevTab = session.tabs.find(t => t.path === session.active);
    if (prevTab?.dirty) {
      if (autoSaveTimers.has(session.active)) {
        clearTimeout(autoSaveTimers.get(session.active));
        autoSaveTimers.delete(session.active);
      }
      void saveDocument(session.active);
    }
  }
  session.active = path;
  activateDocument(path);
}
export async function saveAll() { for (const tab of session.tabs) if (tab.dirty) await saveDocument(tab.path); }
export function removeTab(path: string) {
  if (autoSaveTimers.has(path)) {
    clearTimeout(autoSaveTimers.get(path));
    autoSaveTimers.delete(path);
  }
  const index = session.tabs.findIndex(t => t.path === path);
  session.tabs = session.tabs.filter(t => t.path !== path);
  closeDocument(path);
  if (session.active === path) {
    const next = session.tabs[Math.min(index, session.tabs.length - 1)];
    session.active = next?.path ?? '';
    if (next) {
      activate(next.path);
    } else {
      resetEditorView();
    }
  }
}
export function remapTabs(path: string, destination: string) {
  for (const tab of session.tabs) if (tab.path === path || tab.path.startsWith(`${path}/`)) {
    const next = destination + tab.path.slice(path.length);
    renameDocument(tab.path, next);
    if (session.active === tab.path) session.active = next;
    tab.path = next;
  }
}
export async function chooseWorkspace(before: () => Promise<boolean>, cleanup: () => Promise<void>) {
  try {
    const res = await api.chooseFolder(session.root || undefined);
    if (res.type === 'Selected') {
      await openWorkspacePath(res.path, before, cleanup);
      return;
    }
    if (res.type === 'Cancelled') {
      return;
    }
  } catch {
    // Fallback to tauri dialog if native helper encounters an unexpected error
  }
  const selected = await open({ directory: true, multiple: false, title: 'Open project folder' });
  if (typeof selected !== 'string') return;
  await openWorkspacePath(selected, before, cleanup);
}
export async function openWorkspacePath(selected: string, before: () => Promise<boolean>, cleanup: () => Promise<void>) {
  if (!await before()) return;
  await cleanup();
  const root = await api.open(selected);
  for (const tab of [...session.tabs]) removeTab(tab.path);
  session.root = root;
  session.selected = undefined;
  session.revision++;
  document.title = `${basename(root)} — Nicle`;
}
