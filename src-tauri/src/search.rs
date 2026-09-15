use crate::{
    error::{message, Result},
    workspace::Workspace,
};
use parking_lot::Mutex;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tauri::State;

const IGNORED: &[&str] = &[".git", "node_modules", "target", "dist", "build", ".cache"];

#[derive(Default)]
pub struct SearchState {
    workspace_root: Mutex<Option<PathBuf>>,
    active_cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl SearchState {
    pub fn cancel(&self) {
        let flag = self.active_cancel.lock().take();
        if let Some(flag) = flag {
            flag.store(true, Ordering::Release);
        }
    }
}

#[tauri::command]
pub async fn search_files(
    query: String,
    workspace: State<'_, Workspace>,
    state: State<'_, SearchState>,
) -> Result<Vec<String>> {
    let root = workspace.root()?;

    // Cancel any previous search
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut active = state.active_cancel.lock();
        if let Some(old) = active.take() {
            old.store(true, Ordering::Release);
        }
        *active = Some(Arc::clone(&cancel_flag));
        *state.workspace_root.lock() = Some(root.clone());
    }

    let cancel = Arc::clone(&cancel_flag);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let start = Instant::now();
        let query = query.to_lowercase();
        let mut found = Vec::new();
        let walker = walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !IGNORED.contains(&e.file_name().to_str().unwrap_or("")));

        for entry in walker.take(100_000) {
            if cancel.load(Ordering::Relaxed) {
                return Ok(Vec::new());
            }
            if start.elapsed().as_millis() > 1500 || found.len() >= 100 {
                break;
            }
            let entry = entry.map_err(message)?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry
                .path()
                .strip_prefix(&root)
                .map_err(message)?
                .to_string_lossy()
                .replace('\\', "/");
            if path.to_lowercase().contains(&query) {
                found.push(path);
            }
        }
        Ok(found)
    })
    .await
    .map_err(message)?;

    {
        let mut active = state.active_cancel.lock();
        if let Some(current) = active.as_ref() {
            if Arc::ptr_eq(current, &cancel_flag) {
                active.take();
            }
        }
    }

    result
}

#[tauri::command]
pub fn cancel_search(state: State<'_, SearchState>) {
    state.cancel();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_stops_flag() {
        let state = SearchState::default();
        let flag = Arc::new(AtomicBool::new(false));
        *state.active_cancel.lock() = Some(Arc::clone(&flag));
        assert!(!flag.load(Ordering::Acquire));
        state.cancel();
        assert!(flag.load(Ordering::Acquire));
        assert!(state.active_cancel.lock().is_none());
    }
}
