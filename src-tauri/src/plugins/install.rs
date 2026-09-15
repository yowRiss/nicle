use crate::error::{message, Result};
use crate::plugins::adapters::get_adapter;
use crate::plugins::catalog::find_catalog_plugin;
use crate::plugins::runtime::check_node_runtime;
use crate::plugins::storage::{
    get_app_install_dir, get_staging_dir, remove_installed_record, upsert_installed_record,
    InstalledRecord,
};
use crate::plugins::supervisor::{Phase, PluginSupervisor};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        if name_str == ".git" || name_str == "node_modules" {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&file_name);
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub fn find_bundled_plugin_dir(app: &tauri::AppHandle, app_id: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("plugins").join(app_id));
        candidates.push(cwd.join("examples").join(app_id));
        candidates.push(cwd.join("templates").join(format!("{app_id}-bun")));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("plugins").join(app_id));
        candidates.push(resource_dir.join("examples").join(app_id));
        candidates.push(resource_dir.join("templates").join(format!("{app_id}-bun")));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("plugins").join(app_id));
            candidates.push(parent.join("examples").join(app_id));
            candidates.push(parent.join("..").join("plugins").join(app_id));
            candidates.push(parent.join("..").join("examples").join(app_id));
        }
    }

    for candidate in candidates {
        if candidate.join("nicle-plugin.json").exists() {
            return Some(candidate);
        }
    }

    None
}

pub async fn install_plugin(
    app: &tauri::AppHandle,
    app_id: &str,
    supervisor: &PluginSupervisor,
) -> Result<()> {
    let catalog = find_catalog_plugin(app_id)
        .ok_or_else(|| message(format!("App '{app_id}' not found in catalog")))?;

    let runtime = check_node_runtime();
    if !runtime.available {
        let err_msg = runtime
            .error
            .unwrap_or_else(|| "Node.js and npm are required".to_string());
        supervisor.set_error(app_id, err_msg.clone());
        return Err(message(err_msg));
    }

    if !supervisor.start_transition(app_id, Phase::Installing)? {
        return Err(message("Another operation is already in progress for this app"));
    }

    supervisor.append_log(
        app_id,
        &format!(
            "[INSTALL] Starting installation of {} v{} into private managed storage...\n",
            catalog.name, catalog.pinned_version
        ),
    );

    let staging_base = get_staging_dir(app)?;
    let staging_dir = staging_base.join(format!("{}-stage-{}", app_id, current_timestamp()));
    if let Err(e) = std::fs::create_dir_all(&staging_dir) {
        supervisor.set_error(app_id, format!("Failed to create staging directory: {e}"));
        supervisor.finish_transition(app_id, Phase::NotInstalled);
        return Err(e.into());
    }

    let bundled_opt = find_bundled_plugin_dir(app, app_id);
    if let Some(bundled_dir) = bundled_opt {
        supervisor.append_log(
            app_id,
            &format!(
                "[INSTALL] Installing {} v{} from bundled package files at {}...\n",
                catalog.name,
                catalog.pinned_version,
                bundled_dir.display()
            ),
        );
        if let Err(e) = copy_dir_recursive(&bundled_dir, &staging_dir) {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let err_str = format!("Failed to copy bundled plugin files: {e}");
            supervisor.set_error(app_id, err_str.clone());
            supervisor.finish_transition(app_id, Phase::NotInstalled);
            return Err(message(err_str));
        }

        if !staging_dir.join("nicle-plugin.json").exists() {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let err_str = "Bundled package files missing manifest 'nicle-plugin.json'".to_string();
            supervisor.set_error(app_id, err_str.clone());
            supervisor.finish_transition(app_id, Phase::NotInstalled);
            return Err(message(err_str));
        }
    } else {
        // Write minimal package.json in staging folder
        let pkg_json = format!(
            r#"{{"name":"nicle-companion-{}","private":true,"version":"1.0.0"}}"#,
            app_id
        );
        if let Err(e) = std::fs::write(staging_dir.join("package.json"), pkg_json) {
            let _ = std::fs::remove_dir_all(&staging_dir);
            supervisor.set_error(app_id, format!("Failed to initialize staging: {e}"));
            supervisor.finish_transition(app_id, Phase::NotInstalled);
            return Err(e.into());
        }

        let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };
        let pkg_spec = format!("{}@{}", catalog.npm_package, catalog.pinned_version);

        supervisor.append_log(
            app_id,
            &format!("[INSTALL] Executing npm install --omit=dev --no-audit --no-fund {pkg_spec}\n"),
        );

        let mut cmd = Command::new(npm_cmd);
        cmd.args([
            "install",
            "--omit=dev",
            "--no-audit",
            "--no-fund",
            "--prefer-online",
            &pkg_spec,
        ])
        .current_dir(&staging_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                let err_str = format!("Failed to spawn npm: {e}");
                supervisor.append_log(app_id, &format!("[INSTALL] Error: {err_str}\n"));
                supervisor.set_error(app_id, err_str.clone());
                supervisor.finish_transition(app_id, Phase::NotInstalled);
                return Err(message(err_str));
            }
        };

        // Stream npm stdout & stderr into supervisor logs
        if let Some(stdout) = child.stdout.take() {
            let app_id_str = app_id.to_string();
            let sup = supervisor.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(std::result::Result::ok) {
                    sup.append_log(&app_id_str, &format!("[INSTALL] {line}\n"));
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let app_id_str = app_id.to_string();
            let sup = supervisor.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(std::result::Result::ok) {
                    sup.append_log(&app_id_str, &format!("[INSTALL] {line}\n"));
                }
            });
        }

        let status = match child.wait() {
            Ok(s) => s,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                let err_str = format!("npm process failed: {e}");
                supervisor.set_error(app_id, err_str.clone());
                supervisor.finish_transition(app_id, Phase::NotInstalled);
                return Err(message(err_str));
            }
        };

        if !status.success() {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let err_str = format!("npm install failed with exit code {:?}", status.code());
            supervisor.append_log(app_id, &format!("[INSTALL] {err_str}\n"));
            supervisor.set_error(app_id, err_str.clone());
            supervisor.finish_transition(app_id, Phase::NotInstalled);
            return Err(message(err_str));
        }

        // Verify package installation
        let installed_pkg_dir = staging_dir
            .join("node_modules")
            .join(&catalog.npm_package);
        if !installed_pkg_dir.exists() {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let err_str = "Package files missing after npm install".to_string();
            supervisor.set_error(app_id, err_str.clone());
            supervisor.finish_transition(app_id, Phase::NotInstalled);
            return Err(message(err_str));
        }
    }

    // Move staging to final app dir
    let target_dir = get_app_install_dir(app, app_id)?;
    if target_dir.exists() {
        let backup_dir = staging_base.join(format!("{}-old-{}", app_id, current_timestamp()));
        if let Err(e) = std::fs::rename(&target_dir, &backup_dir) {
            let _ = std::fs::remove_dir_all(&staging_dir);
            supervisor.set_error(app_id, format!("Failed to move old installation: {e}"));
            supervisor.finish_transition(app_id, Phase::InstalledOff);
            return Err(e.into());
        }
        let _ = std::fs::remove_dir_all(&backup_dir);
    }

    if let Err(e) = std::fs::rename(&staging_dir, &target_dir) {
        let _ = std::fs::remove_dir_all(&staging_dir);
        supervisor.set_error(app_id, format!("Failed to finalize installation: {e}"));
        supervisor.finish_transition(app_id, Phase::NotInstalled);
        return Err(e.into());
    }

    let manifest = crate::plugins::manifest::load_manifest_from_dir(&target_dir).ok();
    let port = manifest
        .as_ref()
        .and_then(|m| m.service.as_ref())
        .map(|s| s.default_port)
        .unwrap_or(catalog.default_port);

    let record = InstalledRecord {
        id: app_id.to_string(),
        version: catalog.pinned_version.clone(),
        install_path: target_dir.to_string_lossy().to_string(),
        installed_at: current_timestamp(),
        port,
        start_with_nicle: false,
        is_dev: false,
        manifest,
    };

    if let Err(e) = upsert_installed_record(app, record) {
        supervisor.set_error(app_id, format!("Failed to save install record: {e}"));
        supervisor.finish_transition(app_id, Phase::NotInstalled);
        return Err(e);
    }

    supervisor.append_log(
        app_id,
        &format!("[INSTALL] Successfully installed {} v{}. App is ready to turn On.\n", catalog.name, catalog.pinned_version),
    );
    supervisor.finish_transition(app_id, Phase::InstalledOff);
    Ok(())
}

pub async fn uninstall_plugin(
    app: &tauri::AppHandle,
    app_id: &str,
    delete_data: bool,
    supervisor: &PluginSupervisor,
) -> Result<()> {
    if supervisor.is_running(app_id) {
        supervisor.stop_app(app_id).await?;
    }

    if !supervisor.start_transition(app_id, Phase::Uninstalling)? {
        return Err(message("Another operation is in progress"));
    }

    supervisor.append_log(app_id, "[UNINSTALL] Removing managed installation...\n");

    let target_dir = get_app_install_dir(app, app_id)?;
    if target_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&target_dir) {
            supervisor.append_log(app_id, &format!("[UNINSTALL] Warning: Could not remove directory {}: {e}\n", target_dir.display()));
        }
    }

    let _ = remove_installed_record(app, app_id);

    if delete_data {
        supervisor.append_log(app_id, "[UNINSTALL] Removing companion app data paths...\n");
        if let Ok(adapter) = get_adapter(app_id) {
            for data_path in adapter.get_app_data_paths() {
                if data_path.exists() {
                    let _ = std::fs::remove_dir_all(&data_path);
                    supervisor.append_log(app_id, &format!("[UNINSTALL] Deleted data at {}\n", data_path.display()));
                }
            }
        }
    }

    supervisor.append_log(app_id, "[UNINSTALL] App uninstalled successfully.\n");
    supervisor.finish_transition(app_id, Phase::NotInstalled);
    Ok(())
}

pub async fn update_plugin(
    app: &tauri::AppHandle,
    app_id: &str,
    supervisor: &PluginSupervisor,
) -> Result<()> {
    let was_running = supervisor.is_running(app_id);
    if was_running {
        supervisor.append_log(app_id, "[UPDATE] Stopping app before update...\n");
        supervisor.stop_app(app_id).await?;
    }

    if !supervisor.start_transition(app_id, Phase::Updating)? {
        return Err(message("Another operation is in progress"));
    }

    supervisor.finish_transition(app_id, Phase::InstalledOff);
    install_plugin(app, app_id, supervisor).await?;

    if was_running {
        supervisor.append_log(app_id, "[UPDATE] Restarting app following successful update...\n");
        supervisor.start_app(app, app_id).await?;
    }

    Ok(())
}
