use crate::{
    error::{message, Result},
    workspace::Workspace,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};

const DEBOUNCE_WAIT: Duration = Duration::from_millis(150);
const MAX_DEBOUNCE_WAIT: Duration = Duration::from_millis(1000);
const MAX_WATCHED_FILES: usize = 2048;

enum WatcherMsg {
    Register(PathBuf),
    Fs(notify::Result<Event>),
    Stop,
}

struct ServerHandle {
    stop: Arc<AtomicBool>,
    http_thread: Option<JoinHandle<()>>,
    watcher_thread: Option<JoinHandle<()>>,
    tx: Option<Sender<WatcherMsg>>,
}

impl ServerHandle {
    fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(WatcherMsg::Stop);
        }
        if let Some(thread) = self.http_thread.take() {
            if thread.join().is_err() {
                eprintln!("Nicle: preview http worker failed during shutdown");
            }
        }
        if let Some(thread) = self.watcher_thread.take() {
            if thread.join().is_err() {
                eprintln!("Nicle: preview watcher worker failed during shutdown");
            }
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Default)]
pub struct PreviewState(Mutex<Option<ServerHandle>>);

impl PreviewState {
    pub fn stop(&self) {
        let handle = self.0.lock().take();
        if let Some(mut handle) = handle {
            handle.stop();
        }
    }
}

fn serve(request: tiny_http::Request, root: &Path, tx: &Sender<WatcherMsg>) {
    let url = request.url().split('?').next().unwrap_or("/");
    let decoded = percent_encoding::percent_decode_str(url).decode_utf8();
    let path = decoded.ok().and_then(|s| {
        let path = root.join(s.trim_start_matches('/'));
        let path = if path.is_dir() {
            path.join("index.html")
        } else {
            path
        };
        path.canonicalize().ok()
    });
    let Some(path) = path.filter(|p| p.starts_with(root) && p.is_file()) else {
        if let Err(e) = request.respond(tiny_http::Response::from_string("Not found").with_status_code(404)) {
            eprintln!("Preview response: {e}");
        }
        return;
    };

    if matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("html" | "htm" | "css" | "js" | "mjs")
    ) {
        let _ = tx.send(WatcherMsg::Register(path.clone()));
    }

    let result = std::fs::File::open(&path);
    let response = match result {
        Ok(file) => {
            let mut response = tiny_http::Response::from_file(file);
            if let Ok(header) = tiny_http::Header::from_bytes(
                "Content-Type",
                mime_guess::from_path(&path).first_or_octet_stream().as_ref(),
            ) {
                response.add_header(header);
            }
            if let Ok(header) = tiny_http::Header::from_bytes("Cache-Control", "no-store") {
                response.add_header(header);
            }
            request.respond(response)
        }
        Err(_) => request.respond(tiny_http::Response::from_string("Cannot read file").with_status_code(403)),
    };
    if let Err(e) = response {
        eprintln!("Preview response: {e}");
    }
}

fn run_watcher(
    rx: Receiver<WatcherMsg>,
    mut watcher: Option<RecommendedWatcher>,
    app: AppHandle,
    stop: Arc<AtomicBool>,
) {
    let mut watched_files = HashSet::new();
    let mut watched_parents = HashSet::new();
    let mut pending_reload = false;
    let mut first_event_at = Instant::now();
    let mut last_event_at = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        let timeout = if pending_reload {
            let elapsed = last_event_at.elapsed();
            let max_elapsed = first_event_at.elapsed();
            if elapsed >= DEBOUNCE_WAIT || max_elapsed >= MAX_DEBOUNCE_WAIT {
                pending_reload = false;
                let _ = app.emit("preview-reload", ());
                Duration::from_millis(500)
            } else {
                let remain_debounce = DEBOUNCE_WAIT.saturating_sub(elapsed);
                let remain_max = MAX_DEBOUNCE_WAIT.saturating_sub(max_elapsed);
                remain_debounce.min(remain_max).min(Duration::from_millis(50))
            }
        } else {
            Duration::from_millis(500)
        };

        match rx.recv_timeout(timeout) {
            Ok(WatcherMsg::Stop) => break,
            Ok(WatcherMsg::Register(path)) => {
                if watched_files.len() < MAX_WATCHED_FILES {
                    let canon_path = path.canonicalize().unwrap_or_else(|_| path.clone());
                    if let Some(parent) = canon_path.parent() {
                        if watched_parents.insert(parent.to_path_buf()) {
                            if let Some(w) = watcher.as_mut() {
                                if let Err(e) = w.watch(parent, RecursiveMode::NonRecursive) {
                                    eprintln!("Nicle: cannot watch preview directory {}: {e}", parent.display());
                                }
                            }
                        }
                    }
                    watched_files.insert(canon_path);
                }
            }
            Ok(WatcherMsg::Fs(Ok(event))) => {
                let matches_watched = event.paths.iter().any(|p| {
                    watched_files.contains(p)
                        || p.canonicalize().is_ok_and(|cp| watched_files.contains(&cp))
                });
                if matches_watched {
                    let now = Instant::now();
                    if !pending_reload {
                        pending_reload = true;
                        first_event_at = now;
                    }
                    last_event_at = now;
                }
            }
            Ok(WatcherMsg::Fs(Err(_))) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if pending_reload
                    && (last_event_at.elapsed() >= DEBOUNCE_WAIT
                        || first_event_at.elapsed() >= MAX_DEBOUNCE_WAIT)
                {
                    pending_reload = false;
                    let _ = app.emit("preview-reload", ());
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[tauri::command]
pub async fn start_preview(
    path: String,
    app: AppHandle,
    workspace: State<'_, Workspace>,
    state: State<'_, PreviewState>,
) -> Result<String> {
    let root = workspace.root()?;
    let file = workspace.resolve(&path)?;
    if !matches!(file.extension().and_then(|e| e.to_str()), Some("html" | "htm")) {
        return Err(message("Choose an HTML file"));
    }
    state.stop();

    let server = tiny_http::Server::http("127.0.0.1:0").map_err(message)?;
    let address = server.server_addr();
    let encoded = path
        .split('/')
        .map(|s| percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string())
        .collect::<Vec<_>>()
        .join("/");
    let url = format!("http://{address}/{encoded}");

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = channel::<WatcherMsg>();

    let watcher_tx = tx.clone();
    let watcher_res = RecommendedWatcher::new(
        move |res| {
            let _ = watcher_tx.send(WatcherMsg::Fs(res));
        },
        Config::default(),
    );

    let watcher = match watcher_res {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("Nicle: cannot initialize filesystem watcher: {e}");
            None
        }
    };

    let _ = tx.send(WatcherMsg::Register(file));

    let watcher_stop = Arc::clone(&stop);
    let watcher_app = app.clone();
    let watcher_thread = std::thread::spawn(move || {
        run_watcher(rx, watcher, watcher_app, watcher_stop);
    });

    let http_stop = Arc::clone(&stop);
    let http_tx = tx.clone();
    let http_thread = std::thread::spawn(move || {
        while !http_stop.load(Ordering::Relaxed) {
            match server.recv_timeout(Duration::from_millis(500)) {
                Ok(Some(request)) => serve(request, &root, &http_tx),
                Ok(None) => {}
                Err(e) => {
                    if !http_stop.load(Ordering::Relaxed) {
                        eprintln!("Preview server: {e}");
                    }
                    break;
                }
            }
        }
    });

    *state.0.lock() = Some(ServerHandle {
        stop,
        http_thread: Some(http_thread),
        watcher_thread: Some(watcher_thread),
        tx: Some(tx),
    });

    Ok(url)
}

#[tauri::command]
pub async fn stop_preview(state: State<'_, PreviewState>) -> Result<()> {
    state.stop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_state_stop_cleans_up_cleanly() {
        let state = PreviewState::default();
        state.stop();
        assert!(state.0.lock().is_none());
    }

    #[test]
    fn server_handle_stops_without_hanging() {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel::<WatcherMsg>();
        let watcher_stop = Arc::clone(&stop);
        let watcher_thread = std::thread::spawn(move || {
            while !watcher_stop.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(WatcherMsg::Stop) => break,
                    _ => {}
                }
            }
        });

        let mut handle = ServerHandle {
            stop,
            http_thread: None,
            watcher_thread: Some(watcher_thread),
            tx: Some(tx),
        };
        handle.stop();
        assert!(handle.watcher_thread.is_none());
    }
}
