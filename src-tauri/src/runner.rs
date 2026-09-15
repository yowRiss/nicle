use crate::{error::{message, Result}, processes::{configure_group, kill_tree}, workspace::Workspace};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
use tauri::{AppHandle, Emitter, State};

static NEXT_RUN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerOutputPayload {
    pub run_id: u64,
    pub chunk: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerFinishedPayload {
    pub run_id: u64,
    pub code: Option<i32>,
}

#[derive(Default)]
struct Control {
    cancelled: bool,
    pid: Option<u32>,
}

struct Run {
    control: Arc<Mutex<Control>>,
    worker: JoinHandle<()>,
}

impl Run {
    fn stop(self) {
        {
            let mut control = self.control.lock();
            control.cancelled = true;
            if let Some(pid) = control.pid {
                kill_tree(pid);
            }
        }
        if self.worker.join().is_err() {
            eprintln!("Nicle: runner worker panicked during cleanup");
        }
    }
}

#[derive(Default)]
pub struct RunnerState(Arc<Mutex<Option<Run>>>);

impl RunnerState {
    pub fn stop(&self) {
        let run = self.0.lock().take();
        if let Some(run) = run {
            run.stop();
        }
    }
}

impl Drop for RunnerState {
    fn drop(&mut self) {
        self.stop();
    }
}

fn command(program: impl AsRef<std::ffi::OsStr>, file: &Path, cwd: &Path) -> Command {
    let mut command = Command::new(program);
    command.arg(file).current_dir(cwd);
    command
}
fn commands(file: &Path, root: &Path) -> Result<(Vec<Command>, Option<tempfile::TempDir>)> {
    let extension = file.extension().and_then(|value| value.to_str()).unwrap_or_default();
    let mut temporary = None;
    let commands = match extension {
        "js" | "mjs" | "cjs" => vec![command("node", file, root)],
        "ts" | "tsx" | "mts" | "cts" => {
            let installed = std::env::var_os("PATH").and_then(|path| std::env::split_paths(&path).map(|directory| directory.join(if cfg!(windows) { "tsx.cmd" } else { "tsx" })).find(|path| path.is_file()));
            let mut runner = match installed {
                Some(path) => Command::new(path),
                None => { let mut command = Command::new(if cfg!(windows) { "npx.cmd" } else { "npx" }); command.args(["--no-install", "tsx"]); command }
            };
            runner.arg(file).current_dir(root);
            vec![runner]
        }
        "py" => { let mut runner = Command::new("python3"); runner.arg("-u").arg(file).current_dir(root); vec![runner] },
        "go" => { let mut runner = Command::new("go"); runner.arg("run").arg(file).current_dir(root); vec![runner] }
        "rs" => {
            let cargo_root = file.ancestors().skip(1).take_while(|path| path.starts_with(root))
                .find(|path| path.join("Cargo.toml").is_file()).ok_or_else(|| message("Rust runner requires a Cargo.toml inside the workspace"))?;
            let mut runner = Command::new("cargo"); runner.arg("run").current_dir(cargo_root); vec![runner]
        }
        "c" | "cpp" | "cc" | "cxx" => {
            let directory = tempfile::tempdir()?;
            let executable = directory.path().join(if cfg!(windows) { "nicle-run.exe" } else { "nicle-run" });
            let mut compiler = command(if extension == "c" { "gcc" } else { "g++" }, file, root);
            compiler.arg("-o").arg(&executable);
            let mut runner = Command::new(&executable); runner.current_dir(root);
            temporary = Some(directory);
            vec![compiler, runner]
        }
        _ => return Err(message("No runner for this file type")),
    };
    Ok((commands, temporary))
}
fn stream(mut reader: impl Read, run_id: u64, app: AppHandle) {
    let mut bytes = [0_u8; 8192];
    let mut pending = Vec::with_capacity(8196);
    loop {
        let count = match reader.read(&mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        pending.extend_from_slice(&bytes[..count]);
        let valid = match std::str::from_utf8(&pending) {
            Ok(_) => pending.len(),
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => pending.len(),
        };
        if valid > 0 {
            let chunk = String::from_utf8_lossy(&pending[..valid]).into_owned();
            let _ = app.emit("runner-output", RunnerOutputPayload { run_id, chunk });
            pending.drain(..valid);
        }
    }
    if !pending.is_empty() {
        let chunk = String::from_utf8_lossy(&pending).into_owned();
        let _ = app.emit("runner-output", RunnerOutputPayload { run_id, chunk });
    }
}

fn execute(
    mut command: Command,
    run_id: u64,
    control: &Mutex<Control>,
    app: &AppHandle,
) -> Result<Option<i32>> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    configure_group(&mut command);
    let mut child = {
        let mut control = control.lock();
        if control.cancelled {
            return Ok(None);
        }
        let child = command.spawn().map_err(|error| {
            message(format!(
                "Cannot start {}: {error}. Install the tool on your system first.",
                command.get_program().to_string_lossy()
            ))
        })?;
        control.pid = Some(child.id());
        child
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_app = app.clone();
    let err_app = app.clone();
    let output = std::thread::spawn(move || {
        if let Some(stdout) = stdout {
            stream(stdout, run_id, out_app);
        }
    });
    let errors = std::thread::spawn(move || {
        if let Some(stderr) = stderr {
            stream(stderr, run_id, err_app);
        }
    });
    let status = child.wait();
    {
        let mut control = control.lock();
        kill_tree(child.id());
        control.pid = None;
    }
    if output.join().is_err() {
        eprintln!("Nicle: stdout worker panicked");
    }
    if errors.join().is_err() {
        eprintln!("Nicle: stderr worker panicked");
    }
    Ok(status?.code())
}

#[tauri::command]
pub async fn run_file(
    path: String,
    run_id: Option<u64>,
    app: AppHandle,
    workspace: State<'_, Workspace>,
    state: State<'_, RunnerState>,
) -> Result<u64> {
    let file: PathBuf = workspace.resolve(&path)?;
    let root = workspace.root()?;
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut slot = state.lock();
        if slot.as_ref().is_some_and(|run| !run.worker.is_finished()) {
            return Err(message("A runner is active. Stop it before starting another."));
        }
        if let Some(previous) = slot.take() {
            let _ = previous.worker.join();
        }
        let run_id = run_id.unwrap_or_else(|| NEXT_RUN_ID.fetch_add(1, Ordering::Relaxed));
        let (commands, temporary) = commands(&file, &root)?;
        let control = Arc::new(Mutex::new(Control::default()));
        let worker_control = Arc::clone(&control);
        let worker_app = app.clone();
        let worker = std::thread::spawn(move || {
            let _temporary = temporary;
            let mut code = None;
            for command in commands {
                match execute(command, run_id, &worker_control, &worker_app) {
                    Ok(result) => {
                        code = result;
                        if result != Some(0) {
                            break;
                        }
                    }
                    Err(error) => {
                        code = None;
                        let _ = worker_app.emit(
                            "runner-output",
                            RunnerOutputPayload {
                                run_id,
                                chunk: format!("{error}\n"),
                            },
                        );
                        break;
                    }
                }
            }
            let _ = worker_app.emit("runner-finished", RunnerFinishedPayload { run_id, code });
        });
        *slot = Some(Run { control, worker });
        Ok(run_id)
    })
    .await
    .map_err(message)?
}
#[tauri::command]
pub async fn stop_runner(state: State<'_, RunnerState>) -> Result<()> {
    let state = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let run = state.lock().take();
        if let Some(run) = run { run.stop(); }
    }).await.map_err(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn python_runner_preserves_space_in_filename() -> Result<()> {
        let root = tempfile::tempdir()?;
        let file = root.path().join("with spaces.py");
        std::fs::write(&file, "print('NICLE_RUN_OK')")?;
        let (commands, _temporary) = commands(&file, root.path())?;
        for mut command in commands {
            let output = command.output()?;
            assert!(output.status.success());
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "NICLE_RUN_OK");
        }
        Ok(())
    }
    #[test]
    fn c_runner_compiles_then_executes() -> Result<()> {
        let root = tempfile::tempdir()?;
        let file = root.path().join("hello.c");
        std::fs::write(&file, "#include <stdio.h>\nint main(void){puts(\"NICLE_C_OK\");return 0;}\n")?;
        let (commands, _temporary) = commands(&file, root.path())?;
        let mut last_output = String::new();
        for mut command in commands {
            let output = command.output()?;
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            last_output = String::from_utf8_lossy(&output.stdout).into_owned();
        }
        assert_eq!(last_output.trim(), "NICLE_C_OK");
        Ok(())
    }
}
