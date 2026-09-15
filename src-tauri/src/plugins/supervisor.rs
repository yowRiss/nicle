use crate::error::{message, Result};
use crate::plugins::adapters::get_adapter;
use crate::plugins::storage::{get_app_install_dir, get_installed_record};
use crate::processes::{configure_group, kill_tree};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    NotInstalled,
    Installing,
    InstalledOff,
    Starting,
    Running,
    Stopping,
    Failed,
    Updating,
    Uninstalling,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::NotInstalled => "not-installed",
            Phase::Installing => "installing",
            Phase::InstalledOff => "installed-off",
            Phase::Starting => "starting",
            Phase::Running => "running",
            Phase::Stopping => "stopping",
            Phase::Failed => "failed",
            Phase::Updating => "updating",
            Phase::Uninstalling => "uninstalling",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeStatus {
    pub id: String,
    pub phase: Phase,
    pub pid: Option<u32>,
    pub port: u16,
    pub dashboard_url: Option<String>,
    pub last_error: Option<String>,
    pub transition_active: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogChunkPayload {
    pub id: String,
    pub chunk: String,
}

struct ManagedChild {
    pid: u32,
    child: Arc<Mutex<Option<Child>>>,
}

struct AppInternalState {
    phase: Phase,
    pid: Option<u32>,
    port: u16,
    dashboard_url: Option<String>,
    last_error: Option<String>,
    transition_active: bool,
}

pub const MAX_LOG_BYTES: usize = 1024 * 1024; // 1 MiB
pub const MAX_TOTAL_LOG_BYTES: usize = 4 * 1024 * 1024; // 4 MiB

#[derive(Default)]
pub struct SecretRedactor {
    carryover: String,
    redact_next_value: bool,
}

impl SecretRedactor {
    pub fn process(&mut self, input: &str) -> String {
        let mut combined = std::mem::take(&mut self.carryover);
        combined.push_str(input);
        if combined.is_empty() {
            return String::new();
        }

        // Find the last delimiter (whitespace, =, :, ", ') to know what is a completed token
        let last_delim_idx = combined.rfind(|c: char| c.is_whitespace() || c == '=' || c == ':' || c == '"' || c == '\'');

        let (to_process, pending) = match last_delim_idx {
            Some(idx) => {
                let delim_char_len = combined[idx..].chars().next().map_or(1, |c| c.len_utf8());
                let split_at = idx + delim_char_len;
                let (ready, rem) = combined.split_at(split_at);
                (ready, rem)
            }
            None => {
                if combined.len() > 256 {
                    (combined.as_str(), "")
                } else {
                    self.carryover = combined;
                    return String::new();
                }
            }
        };

        let mut out = String::with_capacity(to_process.len());
        for word in to_process.split_inclusive(|c: char| c.is_whitespace() || c == '=' || c == ':' || c == '"' || c == '\'') {
            let trimmed = word.trim_matches(|c: char| c.is_whitespace() || c == '=' || c == ':' || c == '"' || c == '\'');

            let should_redact = (self.redact_next_value && !trimmed.is_empty())
                || (trimmed.starts_with("sk-") && trimmed.len() > 8)
                || (trimmed.starts_with("key-") && trimmed.len() > 8)
                || (trimmed.starts_with("ghp_") && trimmed.len() > 8);

            if !trimmed.is_empty() {
                self.redact_next_value = trimmed.eq_ignore_ascii_case("bearer");
            }

            if should_redact && !trimmed.is_empty() {
                if let Some(start) = word.find(trimmed) {
                    let end = start + trimmed.len();
                    out.push_str(&word[..start]);
                    out.push_str("[REDACTED]");
                    out.push_str(&word[end..]);
                } else {
                    out.push_str("[REDACTED]");
                }
            } else {
                out.push_str(word);
            }
        }

        self.carryover = pending.to_string();
        out
    }

    pub fn flush(&mut self) -> String {
        if self.carryover.is_empty() {
            return String::new();
        }
        let pending = std::mem::take(&mut self.carryover);
        let trimmed = pending.trim_matches(|c: char| c.is_whitespace() || c == '=' || c == ':' || c == '"' || c == '\'');
        let should_redact = (self.redact_next_value && !trimmed.is_empty())
            || (trimmed.starts_with("sk-") && trimmed.len() > 8)
            || (trimmed.starts_with("key-") && trimmed.len() > 8)
            || (trimmed.starts_with("ghp_") && trimmed.len() > 8);
        self.redact_next_value = false;

        if should_redact && !trimmed.is_empty() {
            if let Some(start) = pending.find(trimmed) {
                let end = start + trimmed.len();
                let mut out = String::with_capacity(pending.len());
                out.push_str(&pending[..start]);
                out.push_str("[REDACTED]");
                out.push_str(&pending[end..]);
                out
            } else {
                "[REDACTED]".to_string()
            }
        } else {
            pending
        }
    }
}

pub fn redact_secrets(input: &str) -> String {
    let mut redactor = SecretRedactor::default();
    let mut out = redactor.process(input);
    out.push_str(&redactor.flush());
    out
}

#[derive(Default)]
pub struct LogStore {
    chunks: std::collections::VecDeque<String>,
    byte_len: usize,
    truncated: bool,
    max_bytes: usize,
}

impl LogStore {
    #[cfg(test)]
    pub fn with_capacity(max_bytes: usize) -> Self {
        Self {
            chunks: std::collections::VecDeque::new(),
            byte_len: 0,
            truncated: false,
            max_bytes,
        }
    }

    pub fn append(&mut self, text: &str) {
        let max = if self.max_bytes == 0 {
            MAX_LOG_BYTES
        } else {
            self.max_bytes
        };
        if text.is_empty() {
            return;
        }

        let slice = if text.len() > max {
            self.truncated = true;
            let overflow = text.len() - max;
            let cut = text
                .char_indices()
                .map(|(i, _)| i)
                .find(|&i| i >= overflow)
                .unwrap_or(overflow);
            &text[cut..]
        } else {
            text
        };

        self.byte_len += slice.len();
        self.chunks.push_back(slice.to_string());

        while self.byte_len > max {
            if let Some(front) = self.chunks.pop_front() {
                self.byte_len = self.byte_len.saturating_sub(front.len());
                self.truncated = true;
            } else {
                break;
            }
        }
    }

    pub fn get(&self) -> String {
        let notice = "[NOTICE: Prior log output truncated to retain 1 MiB buffer]\n";
        let extra_cap = if self.truncated { notice.len() } else { 0 };
        let mut out = String::with_capacity(self.byte_len + extra_cap);
        if self.truncated {
            out.push_str(notice);
        }
        for chunk in &self.chunks {
            out.push_str(chunk);
        }
        out
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.byte_len = 0;
        self.truncated = false;
    }
}

#[derive(Clone, Default)]
pub struct PluginSupervisor {
    children: Arc<Mutex<HashMap<String, ManagedChild>>>,
    states: Arc<Mutex<HashMap<String, AppInternalState>>>,
    logs: Arc<Mutex<HashMap<String, LogStore>>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl PluginSupervisor {
    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.lock() = Some(handle);
    }

    fn emit_state_change(&self, app_id: &str) {
        let status = self.get_runtime_status(app_id);
        if let Some(app) = self.app_handle.lock().as_ref() {
            let _ = app.emit("plugin-state-changed", status);
        }
    }

    pub fn append_redacted_log(&self, app_id: &str, redacted_text: &str) {
        if redacted_text.is_empty() {
            return;
        }
        {
            let mut logs = self.logs.lock();
            let store = logs.entry(app_id.to_string()).or_default();
            store.append(redacted_text);

            let total_bytes: usize = logs.values().map(|s| s.byte_len).sum();
            if total_bytes > MAX_TOTAL_LOG_BYTES {
                if let Some((_, largest)) = logs.iter_mut().max_by_key(|(_, s)| s.byte_len) {
                    while largest.byte_len > MAX_LOG_BYTES / 2 {
                        if largest.chunks.pop_front().is_none() {
                            break;
                        }
                    }
                }
            }
        }
        if let Some(app) = self.app_handle.lock().as_ref() {
            let _ = app.emit(
                "plugin-log-chunk",
                LogChunkPayload {
                    id: app_id.to_string(),
                    chunk: redacted_text.to_string(),
                },
            );
        }
    }

    pub fn append_log(&self, app_id: &str, text: &str) {
        let redacted = redact_secrets(text);
        self.append_redacted_log(app_id, &redacted);
    }

    pub fn get_logs(&self, app_id: &str) -> String {
        self.logs.lock().get(app_id).map(|s| s.get()).unwrap_or_default()
    }

    pub fn clear_logs(&self, app_id: &str) {
        if let Some(store) = self.logs.lock().get_mut(app_id) {
            store.clear();
        }
    }

    pub fn set_error(&self, app_id: &str, err: String) {
        let mut states = self.states.lock();
        let state = states.entry(app_id.to_string()).or_insert_with(|| AppInternalState {
            phase: Phase::NotInstalled,
            pid: None,
            port: 20128,
            dashboard_url: None,
            last_error: None,
            transition_active: false,
        });
        state.last_error = Some(err);
        state.phase = Phase::Failed;
        state.transition_active = false;
        drop(states);
        self.emit_state_change(app_id);
    }

    pub fn start_transition(&self, app_id: &str, target_phase: Phase) -> Result<bool> {
        let mut states = self.states.lock();
        let state = states.entry(app_id.to_string()).or_insert_with(|| AppInternalState {
            phase: Phase::NotInstalled,
            pid: None,
            port: 20128,
            dashboard_url: None,
            last_error: None,
            transition_active: false,
        });

        if state.transition_active {
            return Ok(false);
        }

        state.transition_active = true;
        state.phase = target_phase;
        state.last_error = None;
        drop(states);
        self.emit_state_change(app_id);
        Ok(true)
    }

    pub fn finish_transition(&self, app_id: &str, final_phase: Phase) {
        let mut states = self.states.lock();
        if let Some(state) = states.get_mut(app_id) {
            state.transition_active = false;
            state.phase = final_phase;
            if final_phase != Phase::Running {
                state.pid = None;
                state.dashboard_url = None;
            }
        }
        drop(states);
        self.emit_state_change(app_id);
    }

    pub fn is_running(&self, app_id: &str) -> bool {
        let states = self.states.lock();
        matches!(states.get(app_id).map(|s| s.phase), Some(Phase::Running))
    }

    pub fn get_runtime_status(&self, app_id: &str) -> PluginRuntimeStatus {
        let states = self.states.lock();
        if let Some(st) = states.get(app_id) {
            PluginRuntimeStatus {
                id: app_id.to_string(),
                phase: st.phase,
                pid: st.pid,
                port: st.port,
                dashboard_url: st.dashboard_url.clone(),
                last_error: st.last_error.clone(),
                transition_active: st.transition_active,
            }
        } else {
            PluginRuntimeStatus {
                id: app_id.to_string(),
                phase: Phase::NotInstalled,
                pid: None,
                port: 20128,
                dashboard_url: None,
                last_error: None,
                transition_active: false,
            }
        }
    }

    pub fn sync_installed_state(&self, app_id: &str, installed: bool, port: u16) {
        let mut states = self.states.lock();
        let state = states.entry(app_id.to_string()).or_insert_with(|| AppInternalState {
            phase: Phase::NotInstalled,
            pid: None,
            port,
            dashboard_url: None,
            last_error: None,
            transition_active: false,
        });
        state.port = port;
        if !state.transition_active && state.phase != Phase::Running {
            state.phase = if installed { Phase::InstalledOff } else { Phase::NotInstalled };
        }
    }

    pub async fn start_app(&self, app: &tauri::AppHandle, app_id: &str) -> Result<()> {
        let record = get_installed_record(app, app_id)?
            .ok_or_else(|| message(format!("App '{app_id}' is not installed")))?;

        let install_dir = if record.is_dev || !record.install_path.is_empty() {
            let p = std::path::PathBuf::from(&record.install_path);
            if p.exists() {
                p
            } else {
                get_app_install_dir(app, app_id)?
            }
        } else {
            get_app_install_dir(app, app_id)?
        };
        if !install_dir.exists() {
            return Err(message("Installed app files not found on disk"));
        }

        let adapter = match crate::plugins::storage::get_installed_manifest(app, app_id)? {
            Some(m) => crate::plugins::adapters::get_adapter_for_manifest(m),
            None => get_adapter(app_id)?,
        };
        let port = record.port;

        if !self.start_transition(app_id, Phase::Starting)? {
            return Err(message("Operation already in progress for this app"));
        }

        self.append_log(app_id, &format!("[START] Validating port {port} availability...\n"));

        // Check if port is already occupied (NEVER kill unrelated process!)
        if TcpListener::bind(("127.0.0.1", port)).is_err() {
            let err_msg = format!("Port {port} is already in use. Choose another port in settings.");
            self.append_log(app_id, &format!("[START] Error: {err_msg}\n"));
            self.set_error(app_id, err_msg.clone());
            return Err(message(err_msg));
        }

        self.append_log(app_id, &format!("[START] Launching companion service on 127.0.0.1:{port}...\n"));

        let launch_config = match adapter.resolve_launch_config(&install_dir, port) {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to configure launch: {e}");
                self.append_log(app_id, &format!("[START] {err_msg}\n"));
                self.set_error(app_id, err_msg.clone());
                return Err(message(err_msg));
            }
        };

        let mut cmd = Command::new(&launch_config.program);
        cmd.args(&launch_config.args)
            .current_dir(&launch_config.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (k, v) in &launch_config.env {
            cmd.env(k, v);
        }

        configure_group(&mut cmd);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to start companion process: {e}");
                self.append_log(app_id, &format!("[START] {err_msg}\n"));
                self.set_error(app_id, err_msg.clone());
                return Err(message(err_msg));
            }
        };

        let pid = child.id();
        self.append_log(app_id, &format!("[START] Process spawned with PID {pid}.\n"));

        // Capture stdout and stderr in fixed chunks with UTF-8 safety and cross-chunk redaction
        if let Some(stdout) = child.stdout.take() {
            spawn_stream_reader(stdout, app_id.to_string(), "[RUNTIME] ", self.clone());
        }

        if let Some(stderr) = child.stderr.take() {
            spawn_stream_reader(stderr, app_id.to_string(), "[RUNTIME] ", self.clone());
        }

        let child_arc = Arc::new(Mutex::new(Some(child)));
        self.children.lock().insert(
            app_id.to_string(),
            ManagedChild {
                pid,
                child: Arc::clone(&child_arc),
            },
        );

        // Update state with PID
        {
            let mut states = self.states.lock();
            if let Some(state) = states.get_mut(app_id) {
                state.pid = Some(pid);
                state.port = port;
            }
        }

        // Probe readiness with bounded timeout (15s)
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut ready = false;
        while Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(250)).await;

            // Check if child exited prematurely
            {
                let mut guard = child_arc.lock();
                if let Some(ref mut c) = *guard {
                    if let Ok(Some(status)) = c.try_wait() {
                        let err_msg = format!("Process exited prematurely with code {:?}", status.code());
                        self.append_log(app_id, &format!("[START] Error: {err_msg}\n"));
                        drop(guard);
                        self.children.lock().remove(app_id);
                        self.set_error(app_id, err_msg.clone());
                        return Err(message(err_msg));
                    }
                }
            }

            if adapter.check_readiness(port) {
                ready = true;
                break;
            }
        }

        if !ready {
            let err_msg = format!("Service did not become ready on port {port} within 15 seconds");
            self.append_log(app_id, &format!("[START] Timeout: {err_msg}\n"));
            self.stop_app(app_id).await?;
            self.set_error(app_id, err_msg.clone());
            return Err(message(err_msg));
        }

        let dashboard_url = adapter.get_dashboard_url(port);
        self.append_log(app_id, &format!("[START] Service is ready and running at {dashboard_url}\n"));

        {
            let mut states = self.states.lock();
            if let Some(state) = states.get_mut(app_id) {
                state.phase = Phase::Running;
                state.dashboard_url = Some(dashboard_url);
                state.transition_active = false;
                state.last_error = None;
            }
        }
        self.emit_state_change(app_id);

        // Background monitor thread for unexpected exit
        let sup_clone = self.clone();
        let app_id_owned = app_id.to_string();
        std::thread::spawn(move || {
            let mut guard = child_arc.lock();
            if let Some(ref mut c) = *guard {
                let exit_status = c.wait();
                drop(guard);

                let is_running = {
                    let states = sup_clone.states.lock();
                    matches!(states.get(&app_id_owned).map(|s| s.phase), Some(Phase::Running))
                };

                if is_running {
                    let err = match exit_status {
                        Ok(s) => format!("App stopped unexpectedly (exit code {:?})", s.code()),
                        Err(e) => format!("App stopped unexpectedly ({e})"),
                    };
                    sup_clone.append_log(&app_id_owned, &format!("[RUNTIME] {err}\n"));
                    sup_clone.children.lock().remove(&app_id_owned);
                    sup_clone.set_error(&app_id_owned, err);
                }
            }
        });

        Ok(())
    }

    pub async fn stop_app(&self, app_id: &str) -> Result<()> {
        let managed = self.children.lock().remove(app_id);
        let Some(managed) = managed else {
            // Already stopped or not tracked
            self.finish_transition(app_id, Phase::InstalledOff);
            return Ok(());
        };

        if !self.start_transition(app_id, Phase::Stopping)? {
            // Force transition anyway for stop
            let mut states = self.states.lock();
            if let Some(st) = states.get_mut(app_id) {
                st.phase = Phase::Stopping;
                st.transition_active = true;
            }
            drop(states);
            self.emit_state_change(app_id);
        }

        let pid = managed.pid;
        self.append_log(app_id, &format!("[STOP] Requesting graceful stop for PID {pid}...\n"));

        // Step 1: Request graceful SIGTERM
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            if let Ok(raw) = i32::try_from(pid) {
                let _ = kill(Pid::from_raw(raw), Signal::SIGTERM);
            }
        }
        #[cfg(windows)]
        {
            // Windows forcefully terminates child trees via taskkill
            kill_tree(pid);
        }

        // Step 2: Wait bounded period (up to 3 seconds) for graceful exit
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut exited = false;
        while Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let mut guard = managed.child.lock();
            if let Some(ref mut c) = *guard {
                if let Ok(Some(_)) = c.try_wait() {
                    exited = true;
                    break;
                }
            } else {
                exited = true;
                break;
            }
        }

        // Step 3: Force kill if still alive
        if !exited {
            self.append_log(app_id, &format!("[STOP] Process did not exit in 3s. Terminating process tree...\n"));
            kill_tree(pid);
        }

        self.append_log(app_id, "[STOP] Companion service stopped.\n");
        self.finish_transition(app_id, Phase::InstalledOff);
        Ok(())
    }

    pub async fn restart_app(&self, app: &tauri::AppHandle, app_id: &str) -> Result<()> {
        self.append_log(app_id, "[RESTART] Restarting companion app...\n");
        self.stop_app(app_id).await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        self.start_app(app, app_id).await
    }

    pub fn stop_all(&self) {
        let keys: Vec<String> = self.children.lock().keys().cloned().collect();
        for key in keys {
            if let Some(managed) = self.children.lock().remove(&key) {
                #[cfg(unix)]
                {
                    use nix::sys::signal::{kill, Signal};
                    use nix::unistd::Pid;
                    if let Ok(raw) = i32::try_from(managed.pid) {
                        let _ = kill(Pid::from_raw(raw), Signal::SIGTERM);
                    }
                }
                std::thread::sleep(Duration::from_millis(150));
                kill_tree(managed.pid);
            }
        }
    }
}

fn format_runtime_lines(text: &str, prefix: &str, line_has_prefix: &mut bool) -> String {
    let mut out = String::with_capacity(text.len() + prefix.len() * 4);
    for line in text.split_inclusive('\n') {
        if !*line_has_prefix {
            out.push_str(prefix);
            *line_has_prefix = true;
        }
        out.push_str(line);
        if line.ends_with('\n') {
            *line_has_prefix = false;
        }
    }
    out
}

fn spawn_stream_reader<R: std::io::Read + Send + 'static>(
    mut reader: R,
    app_id: String,
    prefix: &'static str,
    supervisor: PluginSupervisor,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0_u8; 4096];
        let mut leftover_bytes: Vec<u8> = Vec::new();
        let mut redactor = SecretRedactor::default();
        let mut line_has_prefix = false;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };

            let mut all_bytes = std::mem::take(&mut leftover_bytes);
            all_bytes.extend_from_slice(&buf[..n]);

            // Validate UTF-8 boundary
            let valid_len = match std::str::from_utf8(&all_bytes) {
                Ok(s) => s.len(),
                Err(e) => e.valid_up_to(),
            };

            if valid_len < all_bytes.len() {
                leftover_bytes.extend_from_slice(&all_bytes[valid_len..]);
                if leftover_bytes.len() > 4 {
                    leftover_bytes.clear();
                }
            }

            let valid_str = match std::str::from_utf8(&all_bytes[..valid_len]) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let redacted = redactor.process(valid_str);
            if !redacted.is_empty() {
                let formatted = format_runtime_lines(&redacted, prefix, &mut line_has_prefix);
                supervisor.append_redacted_log(&app_id, &formatted);
            }
        }

        let mut rem = redactor.flush();
        if !leftover_bytes.is_empty() {
            rem.push_str(&String::from_utf8_lossy(&leftover_bytes));
        }
        if !rem.is_empty() {
            let formatted = format_runtime_lines(&rem, prefix, &mut line_has_prefix);
            supervisor.append_redacted_log(&app_id, &formatted);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_secrets_filters_tokens_and_preserves_formatting() {
        let raw = "Config loaded. OPENAI_API_KEY=\"sk-proj1234567890abcdef\"\nAuthorization: Bearer my_secret_token_12345\nGITHUB_TOKEN=ghp_abcdefghijklmnop\nNormal line 20128 ok\n";
        let redacted = redact_secrets(raw);
        assert!(!redacted.contains("sk-proj1234567890abcdef"));
        assert!(!redacted.contains("my_secret_token_12345"));
        assert!(!redacted.contains("ghp_abcdefghijklmnop"));
        assert!(redacted.contains("OPENAI_API_KEY=\"[REDACTED]\"\n"));
        assert!(redacted.contains("Authorization: Bearer [REDACTED]\n"));
        assert!(redacted.contains("GITHUB_TOKEN=[REDACTED]\n"));
        assert!(redacted.contains("Normal line 20128 ok\n"));
    }

    #[test]
    fn test_log_store_bounded_truncation_and_clear() {
        let mut store = LogStore::with_capacity(50);
        store.append("Line 1: 1234567890\n");
        assert_eq!(store.truncated, false);
        assert_eq!(store.get(), "Line 1: 1234567890\n");

        // Exceed capacity
        store.append("Line 2: 1234567890\nLine 3: 1234567890\nLine 4: 1234567890\n");
        assert_eq!(store.truncated, true);
        let logs = store.get();
        assert!(logs.contains("[NOTICE: Prior log output truncated to retain 1 MiB buffer]"));

        // Clear resets
        store.clear();
        assert_eq!(store.truncated, false);
        assert_eq!(store.get(), "");
    }

    #[test]
    fn test_port_occupancy_check() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("Failed to bind ephemeral port");
        let port = listener.local_addr().expect("Failed to get local addr").port();

        // While listener is held, port is in use
        let second_bind = TcpListener::bind(("127.0.0.1", port));
        assert!(second_bind.is_err(), "Occupied port must fail to bind");

        // Release port
        drop(listener);
        std::thread::sleep(Duration::from_millis(50));

        // Port is now available
        let free_bind = TcpListener::bind(("127.0.0.1", port));
        assert!(free_bind.is_ok(), "Released port must succeed to bind");
    }

    #[test]
    fn test_phase_string_representations() {
        assert_eq!(Phase::NotInstalled.as_str(), "not-installed");
        assert_eq!(Phase::Installing.as_str(), "installing");
        assert_eq!(Phase::InstalledOff.as_str(), "installed-off");
        assert_eq!(Phase::Starting.as_str(), "starting");
        assert_eq!(Phase::Running.as_str(), "running");
        assert_eq!(Phase::Stopping.as_str(), "stopping");
        assert_eq!(Phase::Failed.as_str(), "failed");
        assert_eq!(Phase::Updating.as_str(), "updating");
        assert_eq!(Phase::Uninstalling.as_str(), "uninstalling");
    }

    #[test]
    fn test_secret_redactor_across_chunks() {
        let mut redactor = SecretRedactor::default();
        // Chunk 1 ends in the middle of a token: "Authorization: Bear"
        let out1 = redactor.process("Authorization: Bear");
        // Chunk 2 completes the Bearer keyword: "er secret_api_token_123456\n"
        let out2 = redactor.process("er secret_api_token_123456\n");
        let mut full = out1;
        full.push_str(&out2);
        full.push_str(&redactor.flush());
        assert!(!full.contains("secret_api_token_123456"));
        assert!(full.contains("Authorization: Bearer [REDACTED]"));

        // Another case: sk- split across chunk boundary
        let mut redactor2 = SecretRedactor::default();
        let p1 = redactor2.process("key=\"sk-");
        let p2 = redactor2.process("proj999988887777\"\n");
        let mut full2 = p1;
        full2.push_str(&p2);
        full2.push_str(&redactor2.flush());
        assert!(!full2.contains("sk-proj999988887777"));
        assert!(full2.contains("key=\"[REDACTED]\""));
    }

    #[test]
    fn test_stream_without_newlines_bounded_buffer() {
        let mut redactor = SecretRedactor::default();
        // Send a 1000-character payload with no whitespace/newlines
        let repeated = "a".repeat(1000);
        let out = redactor.process(&repeated);
        // It shouldn't buffer indefinitely; it should emit once threshold is passed
        assert!(!out.is_empty());
        let flushed = redactor.flush();
        assert_eq!(out.len() + flushed.len(), 1000);
    }
}
