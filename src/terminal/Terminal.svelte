<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { report, attempt } from '../lib/session.svelte';
  import { closeTerminal } from '../lib/tools.svelte';
  import '@xterm/xterm/css/xterm.css';

  let { theme, visible }: { theme: 'dark' | 'light'; visible: boolean } = $props();
  let host: HTMLDivElement;
  let terminal = $state.raw<import('@xterm/xterm').Terminal>();
  let fit: import('@xterm/addon-fit').FitAddon | undefined;
  let ready = false;
  let ended = $state(false);

  // Full ANSI 16-color palette from design.md Section 2 (no purple anywhere; magenta remapped to amber)
  const darkTerminalTheme = {
    background: '#121212',
    foreground: '#EEEEEE',
    cursor: '#D0D0D0',
    cursorAccent: '#121212',
    selectionBackground: '#3A3A3A',
    selectionForeground: '#EEEEEE',
    black: '#1A1A1A',
    red: '#F18D86',
    green: '#86D4A0',
    yellow: '#E8BD75',
    blue: '#79BCEE',
    magenta: '#D7A565', // Remapped to amber per design.md Section 2
    cyan: '#55C7E8',
    white: '#D0D0D0',
    brightBlack: '#808080',
    brightRed: '#FFB0A8',
    brightGreen: '#AFE8BE',
    brightYellow: '#F5D99E',
    brightBlue: '#ACD8F7',
    brightMagenta: '#EBC88F',
    brightCyan: '#85DCF2',
    brightWhite: '#EEEEEE',
  };

  const lightTerminalTheme = {
    background: '#FFFFFF',
    foreground: '#121212',
    cursor: '#333333',
    cursorAccent: '#FFFFFF',
    selectionBackground: '#DCDCDC',
    selectionForeground: '#121212',
    black: '#1A1A1A',
    red: '#D32F2F',
    green: '#1E7E34',
    yellow: '#B36B00',
    blue: '#1976D2',
    magenta: '#B36B00', // Remapped to amber
    cyan: '#0284A6',
    white: '#D0D0D0',
    brightBlack: '#7A7A7A',
    brightRed: '#E53935',
    brightGreen: '#2E7D32',
    brightYellow: '#F57F17',
    brightBlue: '#1E88E5',
    brightMagenta: '#D97706',
    brightCyan: '#00ACC1',
    brightWhite: '#121212',
  };

  let currentSessionId = 0;
  let nextSessionId = 1;

  async function resize() {
    if (!ready || !terminal || !fit || !host || host.clientHeight === 0 || host.clientWidth === 0) return;
    try {
      fit.fit();
      await invoke('terminal_resize', { cols: terminal.cols, rows: terminal.rows });
    } catch (e) {
      report(e);
    }
  }

  async function start() {
    const reqSessionId = nextSessionId++;
    currentSessionId = reqSessionId;
    const returnedSessionId = await invoke<number>('terminal_start', { sessionId: reqSessionId });
    currentSessionId = returnedSessionId;
    ended = false;
    ready = true;
    await resize();
    terminal?.focus();
  }

  onMount(() => {
    let disposed = false;
    const cleanup: (() => void)[] = [];

    void (async () => {
      const [{ Terminal }, { FitAddon }] = await Promise.all([
        import('@xterm/xterm'),
        import('@xterm/addon-fit'),
      ]);

      if (disposed) return;

      terminal = new Terminal({
        fontFamily: "'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        fontSize: 14,
        lineHeight: 1.4,
        cursorBlink: true,
        scrollback: 1000,
        allowProposedApi: false,
        theme: theme === 'dark' ? darkTerminalTheme : lightTerminalTheme,
      });

      fit = new FitAddon();
      terminal.loadAddon(fit);
      terminal.open(host);

      cleanup.push(
        await listen<{ sessionId: number; data: number[] }>('terminal-data', event => {
          if (event.payload.sessionId !== currentSessionId) return;
          const len = event.payload.data.length;
          if (!terminal) {
            void invoke('terminal_ack', { sessionId: event.payload.sessionId, bytes: len }).catch(() => {});
            return;
          }
          const bytes = new Uint8Array(event.payload.data);
          terminal.write(bytes, () => {
            void invoke('terminal_ack', { sessionId: event.payload.sessionId, bytes: len }).catch(() => {});
          });
        })
      );

      cleanup.push(
        await listen<{ sessionId: number }>('terminal-exit', event => {
          if (event.payload.sessionId !== currentSessionId) return;
          ready = false;
          ended = true;
          terminal?.write('\r\n[Shell exited]\r\n');
        })
      );

      const input = terminal.onData(data => {
        void invoke('terminal_write', { data }).catch(report);
      });
      cleanup.push(() => input.dispose());

      const observer = new ResizeObserver(() => {
        void resize().catch(report);
      });
      observer.observe(host);
      cleanup.push(() => observer.disconnect());

      terminal.attachCustomKeyEventHandler(event => {
        if (event.type !== 'keydown' || !(event.ctrlKey || event.metaKey)) return true;
        if (event.shiftKey && event.key.toLowerCase() === 'c') {
          void navigator.clipboard.writeText(terminal?.getSelection() ?? '').catch(report);
          return false;
        }
        if (event.shiftKey && event.key.toLowerCase() === 'v') {
          void navigator.clipboard.readText().then(text => terminal?.paste(text)).catch(report);
          return false;
        }
        return true;
      });

      if (!disposed) {
        await start();
      } else {
        for (const stop of cleanup) stop();
      }
    })().catch(report);

    return () => {
      disposed = true;
      ready = false;
      for (const stop of cleanup) stop();
      terminal?.dispose();
    };
  });

  $effect(() => {
    if (terminal) {
      terminal.options.theme = theme === 'dark' ? darkTerminalTheme : lightTerminalTheme;
    }
  });

  $effect(() => {
    if (visible) {
      requestAnimationFrame(() => {
        void resize().catch(report);
      });
    }
  });
</script>

<div class="terminal-viewport" bind:this={host}></div>
{#if ended}
  <div class="terminal-restart-float" style="display: flex; gap: 8px;">
    <button class="btn-secondary" onclick={() => start().catch(report)}>
      Restart shell
    </button>
    <button class="btn-secondary" onclick={() => attempt(closeTerminal)}>
      End session
    </button>
  </div>
{/if}
