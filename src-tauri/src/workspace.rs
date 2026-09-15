use crate::error::{message, Result};
use parking_lot::RwLock;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
#[serde(tag = "type", content = "path")]
pub enum FolderDialogResult {
    Selected(String),
    Cancelled,
    NotSupported,
}

fn has_executable(name: &str) -> bool {
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

#[derive(Default)]
pub struct Workspace(pub RwLock<Option<PathBuf>>);

impl Workspace {
    pub fn root(&self) -> Result<PathBuf> {
        self.0.read().clone().ok_or_else(|| message("Open a folder first"))
    }

    pub fn resolve(&self, relative: &str) -> Result<PathBuf> {
        let root = self.root()?;
        let relative = Path::new(relative);
        if relative.components().any(|c| !matches!(c, std::path::Component::Normal(_)))
            && !relative.as_os_str().is_empty()
        {
            return Err(message("Path must stay inside the workspace"));
        }
        let path = root.join(relative);
        let checked = if path.exists() {
            path.canonicalize()?
        } else {
            let parent = match path.parent() {
                Some(p) => p,
                None => return Err(message("Invalid path")),
            };
            parent.canonicalize()?
        };
        if !checked.starts_with(&root) {
            return Err(message("Path is outside the workspace"));
        }
        Ok(path)
    }
}

#[tauri::command]
pub async fn open_workspace(
    path: String,
    workspace: tauri::State<'_, Workspace>,
    search_state: tauri::State<'_, crate::search::SearchState>,
) -> Result<String> {
    search_state.cancel();
    let root = tokio::fs::canonicalize(path).await?;
    if !tokio::fs::metadata(&root).await?.is_dir() {
        return Err(message("Choose a folder"));
    }
    *workspace.0.write() = Some(root.clone());
    Ok(root.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn choose_folder(default_path: Option<String>) -> Result<FolderDialogResult> {
    #[cfg(target_os = "linux")]
    {
        let is_kde = std::env::var("XDG_CURRENT_DESKTOP")
            .map(|d| d.to_uppercase().contains("KDE"))
            .unwrap_or(false)
            || std::env::var("DESKTOP_SESSION")
                .map(|s| s.to_uppercase().contains("PLASMA") || s.to_uppercase().contains("KDE"))
                .unwrap_or(false);

        // Native KDE Dolphin folder chooser via kdialog
        if is_kde && has_executable("kdialog") {
            let mut cmd = tokio::process::Command::new("kdialog");
            cmd.arg("--title").arg("Open project folder");
            cmd.arg("--getexistingdirectory");
            if let Some(ref p) = default_path {
                cmd.arg(p);
            } else if let Ok(home) = std::env::var("HOME") {
                cmd.arg(home);
            }
            if let Ok(output) = cmd.output().await {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Ok(FolderDialogResult::Selected(path));
                    }
                }
                return Ok(FolderDialogResult::Cancelled);
            }
        }

        // Native GNOME folder chooser via zenity
        if has_executable("zenity") && !is_kde {
            let mut cmd = tokio::process::Command::new("zenity");
            cmd.arg("--file-selection").arg("--directory").arg("--title=Open project folder");
            if let Some(ref p) = default_path {
                cmd.arg(format!("--filename={p}"));
            }
            if let Ok(output) = cmd.output().await {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Ok(FolderDialogResult::Selected(path));
                    }
                }
                return Ok(FolderDialogResult::Cancelled);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Native Windows File Explorer folder chooser via PowerShell FolderBrowserDialog
        let mut cmd = tokio::process::Command::new("powershell");
        let script = r#"
        Add-Type -AssemblyName System.Windows.Forms
        $f = New-Object System.Windows.Forms.FolderBrowserDialog
        $f.Description = 'Open project folder'
        if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
            Write-Output $f.SelectedPath
        }
        "#;
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(FolderDialogResult::Selected(path));
                }
                return Ok(FolderDialogResult::Cancelled);
            }
        }
    }

    Ok(FolderDialogResult::NotSupported)
}

#[tauri::command]
pub async fn reveal_in_file_manager(path: String) -> Result<()> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(message("Path does not exist"));
    }

    #[cfg(target_os = "linux")]
    {
        let is_kde = std::env::var("XDG_CURRENT_DESKTOP")
            .map(|d| d.to_uppercase().contains("KDE"))
            .unwrap_or(false)
            || std::env::var("DESKTOP_SESSION")
                .map(|s| s.to_uppercase().contains("PLASMA") || s.to_uppercase().contains("KDE"))
                .unwrap_or(false);

        if is_kde && has_executable("dolphin") {
            let mut cmd = tokio::process::Command::new("dolphin");
            if p.is_file() {
                cmd.arg("--select").arg(&path);
            } else {
                cmd.arg(&path);
            }
            let _ = cmd.spawn();
            return Ok(());
        }

        let dir = if p.is_dir() {
            p
        } else {
            match p.parent() {
                Some(parent) => parent,
                None => p,
            }
        };
        let _ = tokio::process::Command::new("xdg-open").arg(dir).spawn();
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        if p.is_file() {
            let _ = tokio::process::Command::new("explorer.exe")
                .arg(format!("/select,\"{}\"", path))
                .spawn();
        } else {
            let _ = tokio::process::Command::new("explorer.exe")
                .arg(&path)
                .spawn();
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let _ = tokio::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn();
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_cannot_escape_workspace() -> Result<()> {
        let root = tempfile::tempdir()?;
        let workspace = Workspace(RwLock::new(Some(root.path().canonicalize()?)));
        assert!(workspace.resolve("../outside.txt").is_err());
        assert!(workspace.resolve("/etc/passwd").is_err());
        assert!(workspace.resolve("new.txt")?.starts_with(root.path()));
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn symlinks_cannot_escape_workspace() -> Result<()> {
        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        std::fs::write(outside.path().join("secret.txt"), "private")?;
        std::os::unix::fs::symlink(outside.path(), root.path().join("link"))?;
        let workspace = Workspace(RwLock::new(Some(root.path().canonicalize()?)));
        assert!(workspace.resolve("link/secret.txt").is_err());
        assert!(workspace.resolve("link/new.txt").is_err());
        Ok(())
    }
}
