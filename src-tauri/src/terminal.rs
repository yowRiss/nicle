use crate::{error::{message, Result}, processes::{enable_reaping, kill_tree}, workspace::Workspace};
use parking_lot::{Condvar, Mutex};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};
use tauri::{AppHandle, Emitter, State};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
const MAX_IN_FLIGHT_BYTES: usize = 64 * 1024; // 64 KiB backpressure limit

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalDataPayload {
    pub session_id: u64,
    pub data: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalExitPayload {
    pub session_id: u64,
}

struct Session {
    id: u64,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    pid: Option<u32>,
    done: Arc<AtomicBool>,
    flow_control: Arc<(Mutex<usize>, Condvar)>,
    waiter: JoinHandle<()>,
}

impl Session {
    fn stop(mut self) {
        self.done.store(true, Ordering::Release);
        self.flow_control.1.notify_all();
        if let Some(pid) = self.pid {
            kill_tree(pid);
        }
        let _ = self.killer.kill();
        drop(self.writer);
        drop(self.master);
        if self.waiter.join().is_err() {
            eprintln!("Nicle: terminal worker panicked during cleanup");
        }
    }
}

#[derive(Default)]
pub struct TerminalState(Arc<Mutex<Option<Session>>>);

impl TerminalState {
    pub fn stop(&self) {
        let session = self.0.lock().take();
        if let Some(session) = session {
            session.stop();
        }
    }

    pub fn ack(&self, session_id: u64, bytes: usize) {
        let slot = self.0.lock();
        if let Some(session) = slot.as_ref() {
            if session.id == session_id {
                let (lock, cvar) = &*session.flow_control;
                let mut guard = lock.lock();
                *guard = guard.saturating_sub(bytes);
                cvar.notify_all();
            }
        }
    }
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        self.stop();
    }
}

#[tauri::command]
pub async fn terminal_start(
    app: AppHandle,
    session_id: Option<u64>,
    workspace: State<'_, Workspace>,
    state: State<'_, TerminalState>,
) -> Result<u64> {
    let root = workspace.root()?;
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut slot = state.lock();
        if let Some(session) = slot.as_ref() {
            if !session.done.load(Ordering::Acquire) {
                return Ok(session.id);
            }
        }
        if let Some(old) = slot.take() {
            old.stop();
        }

        let session_id = session_id.unwrap_or_else(|| NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed));
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(message)?;
        let mut reader = pair.master.try_clone_reader().map_err(message)?;
        let writer = pair.master.take_writer().map_err(message)?;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "powershell.exe"
            } else {
                "/bin/sh"
            }
            .into()
        });
        let mut command = CommandBuilder::new(shell);
        command.cwd(root);
        command.env("TERM", "xterm-256color");
        enable_reaping();
        let mut child = pair.slave.spawn_command(command).map_err(message)?;
        drop(pair.slave);
        let killer = child.clone_killer();
        let pid = child.process_id();
        let done = Arc::new(AtomicBool::new(false));
        let flow_control = Arc::new((Mutex::new(0), Condvar::new()));

        let reader_flow = Arc::clone(&flow_control);
        let reader_done = Arc::clone(&done);
        let output_app = app.clone();

        let reader_thread = std::thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                let count = match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => count,
                };

                // Flow control / backpressure check: wait if in-flight bytes exceed threshold
                {
                    let (lock, cvar) = &*reader_flow;
                    let mut in_flight = lock.lock();
                    while *in_flight >= MAX_IN_FLIGHT_BYTES && !reader_done.load(Ordering::Acquire) {
                        cvar.wait_for(&mut in_flight, Duration::from_millis(50));
                    }
                    if reader_done.load(Ordering::Acquire) {
                        break;
                    }
                    *in_flight += count;
                }

                let payload = TerminalDataPayload {
                    session_id,
                    data: buffer[..count].to_vec(),
                };
                if output_app.emit("terminal-data", &payload).is_err() {
                    break;
                }
            }
        });

        let finished = Arc::clone(&done);
        let exit_app = app.clone();
        let waiter = std::thread::spawn(move || {
            if let Err(error) = child.wait() {
                eprintln!("Nicle: terminal wait failed: {error}");
            }
            if let Some(pid) = pid {
                kill_tree(pid);
            }
            finished.store(true, Ordering::Release);
            let _ = reader_thread.join();
            let _ = exit_app.emit("terminal-exit", TerminalExitPayload { session_id });
        });

        *slot = Some(Session {
            id: session_id,
            master: pair.master,
            writer,
            killer,
            pid,
            done,
            flow_control,
            waiter,
        });
        Ok(session_id)
    })
    .await
    .map_err(message)?
}

#[tauri::command]
pub fn terminal_ack(session_id: u64, bytes: usize, state: State<'_, TerminalState>) {
    state.ack(session_id, bytes);
}

#[tauri::command]
pub async fn terminal_write(data: String, state: State<'_, TerminalState>) -> Result<()> {
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut slot = state.lock();
        let session = slot.as_mut().ok_or_else(|| message("Terminal is closed"))?;
        session.writer.write_all(data.as_bytes())?;
        session.writer.flush()?;
        Ok(())
    }).await.map_err(message)?
}

#[tauri::command]
pub fn terminal_resize(cols: u16, rows: u16, state: State<'_, TerminalState>) -> Result<()> {
    let slot = state.0.lock();
    let session = slot.as_ref().ok_or_else(|| message("Terminal is closed"))?;
    session.master.resize(PtySize { rows: rows.max(1), cols: cols.max(1), pixel_width: 0, pixel_height: 0 }).map_err(message)
}

#[tauri::command]
pub async fn terminal_stop(state: State<'_, TerminalState>) -> Result<()> {
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let session = state.lock().take();
        if let Some(session) = session { session.stop(); }
    }).await.map_err(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(unix)]
    fn shell_accepts_input_and_resizes() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let pair = native_pty_system().openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })?;
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let mut child = pair.slave.spawn_command(CommandBuilder::new("/bin/sh"))?;
        drop(pair.slave);
        pair.master.resize(PtySize { rows: 31, cols: 101, pixel_width: 0, pixel_height: 0 })?;
        writer.write_all(b"printf 'NICLE_PTY_OK\\n'; stty size; exit\n")?;
        let mut output = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop { match reader.read(&mut buffer) { Ok(0) | Err(_) => break, Ok(count) => output.extend_from_slice(&buffer[..count]) } }
        assert!(child.wait()?.success());
        let output = String::from_utf8_lossy(&output);
        assert!(output.contains("NICLE_PTY_OK"));
        assert!(output.contains("31 101"));
        Ok(())
    }
}
