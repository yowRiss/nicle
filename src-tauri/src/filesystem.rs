use crate::{error::{message, Result}, workspace::Workspace};
use serde::Serialize;
use std::io::{Read, Write};
#[derive(Serialize)]
pub struct Entry { name: String, path: String, directory: bool }
#[derive(Serialize)]
pub struct Page { entries: Vec<Entry>, more: bool }
#[tauri::command]
pub async fn list_directory(path: String, offset: usize, workspace: tauri::State<'_, Workspace>) -> Result<Page> {
    let directory = workspace.resolve(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut entries = Vec::with_capacity(201);
        for entry in std::fs::read_dir(directory)?.skip(offset).take(201) {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let directory = entry.file_type()?.is_dir();
            entries.push(Entry { path: if path.is_empty() { name.clone() } else { format!("{path}/{name}") }, name, directory });
        }
        let more = entries.len() > 200;
        entries.truncate(200);
        entries.sort_by(|a,b| b.directory.cmp(&a.directory).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ok(Page { entries, more })
    }).await.map_err(message)?
}
#[tauri::command]
pub async fn read_file(path: String, workspace: tauri::State<'_, Workspace>) -> Result<String> {
    let path = workspace.resolve(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let file = std::fs::File::open(path)?;
        let mut content = String::new();
        file.take(8 * 1024 * 1024 + 1).read_to_string(&mut content)?;
        if content.len() > 8 * 1024 * 1024 { return Err(message("File exceeds the 8 MB editor limit")); }
        if content.contains('\0') { return Err(message("Binary files cannot be edited")); }
        Ok(content)
    }).await.map_err(message)?
}
#[tauri::command]
pub async fn save_file(path: String, content: String, workspace: tauri::State<'_, Workspace>) -> Result<()> {
    let path = workspace.resolve(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let destination = path.canonicalize()?;
        let parent = destination.parent().ok_or_else(|| message("Invalid file"))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.as_file().set_permissions(std::fs::metadata(&destination)?.permissions())?;
        temp.write_all(content.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist(destination).map_err(message)?;
        Ok(())
    }).await.map_err(message)?
}
#[tauri::command]
pub async fn create_entry(path: String, directory: bool, workspace: tauri::State<'_, Workspace>) -> Result<()> {
    let path = workspace.resolve(&path)?;
    if directory { tokio::fs::create_dir(path).await?; }
    else { tokio::fs::OpenOptions::new().write(true).create_new(true).open(path).await?; }
    Ok(())
}
#[tauri::command]
pub async fn rename_entry(path: String, destination: String, workspace: tauri::State<'_, Workspace>) -> Result<()> {
    if path.is_empty() { return Err(message("Cannot rename the workspace root")); }
    let path = workspace.resolve(&path)?;
    let destination = workspace.resolve(&destination)?;
    if destination.exists() { return Err(message("A file or folder already has that name")); }
    tokio::fs::rename(path, destination).await?;
    Ok(())
}
#[tauri::command]
pub async fn delete_entry(path: String, workspace: tauri::State<'_, Workspace>) -> Result<()> {
    if path.is_empty() { return Err(message("Cannot delete the workspace root")); }
    let path = workspace.resolve(&path)?;
    let metadata = tokio::fs::symlink_metadata(&path).await?;
    if metadata.is_dir() { tokio::fs::remove_dir_all(path).await?; } else { tokio::fs::remove_file(path).await?; }
    Ok(())
}
