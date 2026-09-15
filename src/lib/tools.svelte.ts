import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { saveDocument } from '../editor/documents';
import { session, report } from './session.svelte';
export const tools = $state({ panel: 'terminal', panelVisible: false, terminalMounted: false, running: false, output: '', previewUrl: '', previewPath: '', previewVersion: 0 });

let currentRunId = 0;
let nextRunId = 1;
let internalOutput = '';
let pendingRunnerChunks: string[] = [];
let pendingRunnerChars = 0;
const MAX_PENDING_RUNNER_CHARS = 100_000;
let runnerFlushTimer: ReturnType<typeof setTimeout> | null = null;

function flushRunnerOutput() {
  if (runnerFlushTimer) {
    clearTimeout(runnerFlushTimer);
    runnerFlushTimer = null;
  }
  if (pendingRunnerChunks.length === 0) return;
  const batch = pendingRunnerChunks.join('');
  pendingRunnerChunks = [];
  pendingRunnerChars = 0;
  internalOutput = (internalOutput + batch).slice(-200_000);
  tools.output = internalOutput;
}

function queueRunnerChunk(chunk: string) {
  if (pendingRunnerChars + chunk.length > MAX_PENDING_RUNNER_CHARS) {
    flushRunnerOutput();
  }
  pendingRunnerChunks.push(chunk);
  pendingRunnerChars += chunk.length;
  if (!runnerFlushTimer) {
    runnerFlushTimer = setTimeout(flushRunnerOutput, 80);
  }
}

export function toggleTerminal() { tools.panelVisible = !tools.panelVisible || tools.panel !== 'terminal'; tools.panel = 'terminal'; if (session.root && tools.panelVisible) tools.terminalMounted = true; }
export async function sendTerminalCommand(command: string) {
  tools.panel = 'terminal';
  tools.panelVisible = true;
  if (session.root) tools.terminalMounted = true;
  await invoke('terminal_write', { data: `${command}\r` });
}
export async function closeTerminal() { await invoke('terminal_stop'); tools.terminalMounted = false; }
export async function stopRunner() {
  flushRunnerOutput();
  await invoke('stop_runner');
  tools.running = false;
}
export async function runFile() {
  if (!session.active || tools.running) return;
  if (runnerFlushTimer) {
    clearTimeout(runnerFlushTimer);
    runnerFlushTimer = null;
  }
  pendingRunnerChunks = [];
  pendingRunnerChars = 0;
  internalOutput = '';
  tools.panel = 'output';
  tools.panelVisible = true;
  tools.output = '';
  tools.running = true;
  const runId = nextRunId++;
  currentRunId = runId;
  try {
    await saveDocument(session.active);
    await invoke('run_file', { path: session.active, runId });
  } catch (e) {
    tools.running = false;
    report(e);
  }
}
export async function restartRunner() { await stopRunner(); await runFile(); }
export async function previewFile() { await saveDocument(session.active); tools.previewUrl = await invoke<string>('start_preview', {path: session.active}); tools.previewPath = session.active; tools.previewVersion++; }
export async function closePreview() { await invoke('stop_preview'); tools.previewUrl = ''; tools.previewPath = ''; }
export async function cleanupTools() {
  if (runnerFlushTimer) {
    clearTimeout(runnerFlushTimer);
    runnerFlushTimer = null;
  }
  pendingRunnerChunks = [];
  pendingRunnerChars = 0;
  await closeTerminal();
  await stopRunner();
  await closePreview();
  tools.panelVisible = false;
}
export async function listenTools() {
  const removers = await Promise.all([
    listen<{ runId: number; chunk: string }>('runner-output', e => {
      if (e.payload.runId === currentRunId) {
        queueRunnerChunk(e.payload.chunk);
      }
    }),
    listen<{ runId: number; code: number | null }>('runner-finished', e => {
      if (e.payload.runId !== currentRunId) return;
      flushRunnerOutput();
      tools.running = false;
      const exitMsg = `\n[Process exited${e.payload.code === null ? '' : ` with code ${e.payload.code}`}]\n`;
      internalOutput = (internalOutput + exitMsg).slice(-200_000);
      tools.output = internalOutput;
    }),
    listen('preview-reload', () => tools.previewVersion++),
  ]);
  return () => {
    if (runnerFlushTimer) {
      clearTimeout(runnerFlushTimer);
      runnerFlushTimer = null;
    }
    for (const remove of removers) remove();
  };
}
