use crate::{
    error::{message, Result},
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_DIFF_BYTES: usize = 1024 * 1024; // 1 MiB

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    pub path: String,
    pub status: String, // "modified" | "added" | "deleted" | "renamed" | "untracked"
    pub old_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusResult {
    pub git_available: bool,
    pub is_repo: bool,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<GitFileChange>,
    pub unstaged: Vec<GitFileChange>,
    pub untracked: Vec<GitFileChange>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchInfo {
    pub name: String,
    pub current: bool,
    pub is_remote: bool,
}

pub fn is_git_available() -> bool {
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(if cfg!(windows) { "git.exe" } else { "git" });
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

pub fn validate_relative_path(path_str: &str) -> Result<&Path> {
    let p = Path::new(path_str);
    if p.as_os_str().is_empty() {
        return Err(message("Path cannot be empty"));
    }
    if p.is_absolute() {
        return Err(message("Absolute paths not allowed in Git operations"));
    }
    for component in p.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err(message("Path must stay inside the repository"));
            }
        }
    }
    Ok(p)
}

fn resolve_repo_dir(
    workspace_root: Option<String>,
    workspace: &tauri::State<'_, Workspace>,
) -> Result<PathBuf> {
    if let Some(dir) = workspace_root {
        if !dir.trim().is_empty() {
            let path = PathBuf::from(dir);
            if path.exists() {
                return Ok(path);
            }
        }
    }
    workspace.root()
}

async fn run_git_cmd<I, S>(dir: &Path, args: I) -> Result<std::process::Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut cmd = tokio::process::Command::new("git");
    cmd.current_dir(dir);
    cmd.args(args);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    cmd.output()
        .await
        .map_err(|e| message(format!("Failed to execute git: {e}")))
}

pub fn clean_git_path(raw: &str) -> String {
    let s = raw.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn parse_rename_path(raw: &str) -> (String, String) {
    if let Some((old_p, new_p)) = raw.split_once(" -> ") {
        (clean_git_path(old_p), clean_git_path(new_p))
    } else {
        (String::new(), clean_git_path(raw))
    }
}

pub fn parse_branch_header(header: &str) -> (String, usize, usize) {
    let header = header.trim();
    if let Some(rest) = header.strip_prefix("No commits yet on ") {
        return (rest.trim().to_string(), 0, 0);
    }
    if let Some(rest) = header.strip_prefix("Initial commit on ") {
        return (rest.trim().to_string(), 0, 0);
    }
    if header.starts_with("HEAD (no branch)") {
        return ("HEAD".to_string(), 0, 0);
    }

    let mut ahead = 0;
    let mut behind = 0;

    let (branch_part, meta_part) = if let Some(bracket_start) = header.find('[') {
        let branch_part = header[..bracket_start].trim();
        let meta_part = &header[bracket_start..];
        (branch_part, Some(meta_part))
    } else {
        (header, None)
    };

    if let Some(meta) = meta_part {
        if let Some(a_idx) = meta.find("ahead ") {
            let rest = &meta[a_idx + 6..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = num_str.parse::<usize>() {
                ahead = num;
            }
        }
        if let Some(b_idx) = meta.find("behind ") {
            let rest = &meta[b_idx + 7..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = num_str.parse::<usize>() {
                behind = num;
            }
        }
    }

    let branch_name = if let Some(dot_idx) = branch_part.find("...") {
        &branch_part[..dot_idx]
    } else {
        branch_part
    };

    (branch_name.trim().to_string(), ahead, behind)
}

pub fn parse_porcelain_v1(stdout: &str) -> GitStatusResult {
    let mut branch = String::new();
    let mut ahead = 0;
    let mut behind = 0;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    let mut lines = stdout.lines();
    if let Some(first_line) = lines.next() {
        if let Some(branch_line) = first_line.strip_prefix("## ") {
            let (b_name, b_ahead, b_behind) = parse_branch_header(branch_line);
            branch = b_name;
            ahead = b_ahead;
            behind = b_behind;
        }
    }

    for line in lines {
        if line.len() < 3 {
            continue;
        }
        let (status_chars, path_part) = line.split_at(2);
        let path_str = path_part.trim_start();
        let chars: Vec<char> = status_chars.chars().collect();
        if chars.len() < 2 {
            continue;
        }
        let x = chars[0];
        let y = chars[1];

        // Untracked
        if x == '?' && y == '?' {
            untracked.push(GitFileChange {
                path: clean_git_path(path_str),
                status: "untracked".to_string(),
                old_path: None,
            });
            continue;
        }

        // Staged (index 0)
        if x != ' ' && x != '?' {
            let (status, path, old_path) = match x {
                'M' => ("modified".to_string(), clean_git_path(path_str), None),
                'A' => ("added".to_string(), clean_git_path(path_str), None),
                'D' => ("deleted".to_string(), clean_git_path(path_str), None),
                'R' => {
                    let (old_p, new_p) = parse_rename_path(path_str);
                    ("renamed".to_string(), new_p, Some(old_p))
                }
                'C' => ("added".to_string(), clean_git_path(path_str), None),
                _ => ("modified".to_string(), clean_git_path(path_str), None),
            };
            staged.push(GitFileChange {
                path,
                status,
                old_path,
            });
        }

        // Unstaged (index 1)
        if y != ' ' && y != '?' {
            let (status, path, old_path) = match y {
                'M' => ("modified".to_string(), clean_git_path(path_str), None),
                'D' => ("deleted".to_string(), clean_git_path(path_str), None),
                'A' => ("added".to_string(), clean_git_path(path_str), None),
                'R' => {
                    let (old_p, new_p) = parse_rename_path(path_str);
                    ("renamed".to_string(), new_p, Some(old_p))
                }
                _ => ("modified".to_string(), clean_git_path(path_str), None),
            };
            unstaged.push(GitFileChange {
                path,
                status,
                old_path,
            });
        }
    }

    GitStatusResult {
        git_available: true,
        is_repo: true,
        branch,
        ahead,
        behind,
        staged,
        unstaged,
        untracked,
    }
}

pub fn parse_git_branches(stdout: &str) -> Vec<GitBranchInfo> {
    let mut branches = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains(" -> ") {
            continue;
        }
        let current = trimmed.starts_with('*');
        let raw_name = trimmed.trim_start_matches('*').trim();
        let is_remote = raw_name.starts_with("remotes/");
        let name = if let Some(stripped) = raw_name.strip_prefix("remotes/") {
            stripped.to_string()
        } else {
            raw_name.to_string()
        };
        branches.push(GitBranchInfo {
            name,
            current,
            is_remote,
        });
    }
    branches
}

#[tauri::command]
pub async fn git_status(
    workspace_root: Option<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<GitStatusResult> {
    if !is_git_available() {
        return Ok(GitStatusResult {
            git_available: false,
            is_repo: false,
            branch: String::new(),
            ahead: 0,
            behind: 0,
            staged: Vec::new(),
            unstaged: Vec::new(),
            untracked: Vec::new(),
        });
    }

    let repo_dir = match resolve_repo_dir(workspace_root, &workspace) {
        Ok(dir) => dir,
        Err(_) => {
            return Ok(GitStatusResult {
                git_available: true,
                is_repo: false,
                branch: String::new(),
                ahead: 0,
                behind: 0,
                staged: Vec::new(),
                unstaged: Vec::new(),
                untracked: Vec::new(),
            })
        }
    };

    let output = match run_git_cmd(&repo_dir, ["status", "--porcelain=v1", "-b", "-uall"]).await {
        Ok(out) => out,
        Err(_) => {
            return Ok(GitStatusResult {
                git_available: true,
                is_repo: false,
                branch: String::new(),
                ahead: 0,
                behind: 0,
                staged: Vec::new(),
                unstaged: Vec::new(),
                untracked: Vec::new(),
            })
        }
    };

    if !output.status.success() {
        return Ok(GitStatusResult {
            git_available: true,
            is_repo: false,
            branch: String::new(),
            ahead: 0,
            behind: 0,
            staged: Vec::new(),
            unstaged: Vec::new(),
            untracked: Vec::new(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_porcelain_v1(&stdout))
}

#[tauri::command]
pub async fn git_diff(
    workspace_root: Option<String>,
    path: Option<String>,
    staged: bool,
    workspace: tauri::State<'_, Workspace>,
) -> Result<String> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;

    let mut args = vec!["diff", "--no-color"];
    if staged {
        args.push("--cached");
    }

    if let Some(ref p) = path {
        if !p.is_empty() {
            validate_relative_path(p)?;
            args.push("--");
            args.push(p.as_str());
        }
    }

    let output = run_git_cmd(&repo_dir, &args).await?;
    let mut diff_text = String::from_utf8_lossy(&output.stdout).into_owned();

    // If diff is empty, not staged, and path is provided, check if untracked
    if diff_text.is_empty() && !staged {
        if let Some(ref p) = path {
            let full_file_path = repo_dir.join(p);
            if full_file_path.is_file() {
                let no_index_out = run_git_cmd(
                    &repo_dir,
                    ["diff", "--no-color", "--no-index", "--", "/dev/null", p.as_str()],
                )
                .await;
                if let Ok(no_idx) = no_index_out {
                    let text = String::from_utf8_lossy(&no_idx.stdout).into_owned();
                    if !text.is_empty() {
                        diff_text = text;
                    }
                }
            }
        }
    }

    if diff_text.len() > MAX_DIFF_BYTES {
        let mut limit = MAX_DIFF_BYTES;
        while limit > 0 && !diff_text.is_char_boundary(limit) {
            limit -= 1;
        }
        let mut truncated = diff_text;
        truncated.truncate(limit);
        truncated.push_str("\n\n[Diff truncated: exceeds 1 MiB limit]");
        Ok(truncated)
    } else {
        Ok(diff_text)
    }
}

#[tauri::command]
pub async fn git_stage(
    workspace_root: Option<String>,
    paths: Vec<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    let mut args = vec!["add", "-A"];
    if !paths.is_empty() {
        for p in &paths {
            validate_relative_path(p)?;
        }
        args.push("--");
        for p in &paths {
            args.push(p.as_str());
        }
    }
    let output = run_git_cmd(&repo_dir, &args).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(message(format!("Git stage failed: {}", err.trim())));
    }
    Ok(())
}

#[tauri::command]
pub async fn git_unstage(
    workspace_root: Option<String>,
    paths: Vec<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    for p in &paths {
        validate_relative_path(p)?;
    }
    let mut reset_args = vec!["reset", "HEAD"];
    if !paths.is_empty() {
        reset_args.push("--");
        for p in &paths {
            reset_args.push(p.as_str());
        }
    }
    let output = run_git_cmd(&repo_dir, &reset_args).await?;
    if !output.status.success() {
        // If HEAD does not exist (initial commit), fallback to git rm --cached
        let mut rm_args = vec!["rm", "--cached", "-r", "--"];
        if paths.is_empty() {
            rm_args.push(".");
        } else {
            for p in &paths {
                rm_args.push(p.as_str());
            }
        }
        let rm_output = run_git_cmd(&repo_dir, &rm_args).await?;
        if !rm_output.status.success() {
            let err = String::from_utf8_lossy(&rm_output.stderr);
            return Err(message(format!("Git unstage failed: {}", err.trim())));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn git_discard(
    workspace_root: Option<String>,
    paths: Vec<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    if paths.is_empty() {
        let _ = run_git_cmd(&repo_dir, ["reset", "HEAD", "--", "."]).await;
        let _ = run_git_cmd(&repo_dir, ["checkout", "--", "."]).await;
        let _ = run_git_cmd(&repo_dir, ["clean", "-fd"]).await;
        return Ok(());
    }

    for p in &paths {
        validate_relative_path(p)?;
        let file_path = repo_dir.join(p);

        // 1. Unstage any staged changes for this path
        let _ = run_git_cmd(&repo_dir, ["reset", "HEAD", "--", p.as_str()]).await;

        // 2. Try checking out tracked changes
        let out = run_git_cmd(&repo_dir, ["checkout", "--", p.as_str()]).await;
        if let Ok(o) = out {
            if o.status.success() {
                // Successfully restored tracked file — do NOT delete it!
                continue;
            }
        }

        // 3. Checkout failed (untracked file). Try git clean.
        let clean_out = run_git_cmd(&repo_dir, ["clean", "-fd", "--", p.as_str()]).await;
        if let Ok(co) = clean_out {
            if co.status.success() && !file_path.exists() {
                continue;
            }
        }

        // 4. Fallback deletion safely inside repo_dir
        if file_path.exists() && file_path.starts_with(&repo_dir) {
            if file_path.is_dir() {
                let _ = tokio::fs::remove_dir_all(&file_path).await;
            } else {
                let _ = tokio::fs::remove_file(&file_path).await;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn git_commit(
    workspace_root: Option<String>,
    message: String,
    amend: bool,
    workspace: tauri::State<'_, Workspace>,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    if message.trim().is_empty() {
        return Err(crate::error::message("Commit message cannot be empty"));
    }

    let mut args = vec!["commit"];
    if amend {
        args.push("--amend");
    }
    args.push("-m");
    args.push(&message);

    let output = run_git_cmd(&repo_dir, &args).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let msg = if !err.trim().is_empty() {
            err.trim()
        } else {
            out.trim()
        };
        return Err(crate::error::message(format!("Git commit failed: {msg}")));
    }
    Ok(())
}

#[tauri::command]
pub async fn git_push(
    workspace_root: Option<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<String> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    let push_future = run_git_cmd(&repo_dir, ["push"]);
    let output = match tokio::time::timeout(std::time::Duration::from_secs(30), push_future).await {
        Ok(res) => res?,
        Err(_) => return Err(crate::error::message("Git push timed out after 30 seconds")),
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let msg = if !stderr.trim().is_empty() {
            stderr.trim()
        } else {
            stdout.trim()
        };
        return Err(crate::error::message(format!("Git push failed: {msg}")));
    }
    let combined = format!("{}{}", stdout, stderr);
    Ok(if combined.trim().is_empty() {
        "Push successful".to_string()
    } else {
        combined.trim().to_string()
    })
}

#[tauri::command]
pub async fn git_pull(
    workspace_root: Option<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<String> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    let pull_future = run_git_cmd(&repo_dir, ["pull"]);
    let output = match tokio::time::timeout(std::time::Duration::from_secs(30), pull_future).await {
        Ok(res) => res?,
        Err(_) => return Err(crate::error::message("Git pull timed out after 30 seconds")),
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let msg = if !stderr.trim().is_empty() {
            stderr.trim()
        } else {
            stdout.trim()
        };
        return Err(crate::error::message(format!("Git pull failed: {msg}")));
    }
    let combined = format!("{}{}", stdout, stderr);
    Ok(if combined.trim().is_empty() {
        "Pull successful".to_string()
    } else {
        combined.trim().to_string()
    })
}

#[tauri::command]
pub async fn git_branches(
    workspace_root: Option<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<Vec<GitBranchInfo>> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    let output = run_git_cmd(&repo_dir, ["branch", "-a"]).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error::message(format!(
            "Git branches failed: {}",
            err.trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_git_branches(&stdout))
}

#[tauri::command]
pub async fn git_checkout(
    workspace_root: Option<String>,
    branch: String,
    workspace: tauri::State<'_, Workspace>,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    let output = run_git_cmd(&repo_dir, ["checkout", branch.as_str()]).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error::message(format!(
            "Git checkout failed: {}",
            err.trim()
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn git_create_branch(
    workspace_root: Option<String>,
    name: String,
    workspace: tauri::State<'_, Workspace>,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    let output = run_git_cmd(&repo_dir, ["checkout", "-b", name.as_str()]).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error::message(format!(
            "Git create branch failed: {}",
            err.trim()
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn git_init(
    workspace_root: Option<String>,
    workspace: tauri::State<'_, Workspace>,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(workspace_root, &workspace)?;
    let output = run_git_cmd(&repo_dir, ["init"]).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(crate::error::message(format!("Git init failed: {}", err.trim())));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_relative_path() {
        assert!(validate_relative_path("src/lib.rs").is_ok());
        assert!(validate_relative_path("README.md").is_ok());
        assert!(validate_relative_path("../outside.rs").is_err());
        assert!(validate_relative_path("src/../../outside.rs").is_err());
        assert!(validate_relative_path("/etc/passwd").is_err());
        assert!(validate_relative_path("").is_err());
    }

    #[test]
    fn test_parse_branch_header() {
        let (b, a, beh) = parse_branch_header("main...origin/main [ahead 1, behind 2]");
        assert_eq!(b, "main");
        assert_eq!(a, 1);
        assert_eq!(beh, 2);

        let (b, a, beh) = parse_branch_header("feature/foo...origin/feature/foo [ahead 3]");
        assert_eq!(b, "feature/foo");
        assert_eq!(a, 3);
        assert_eq!(beh, 0);

        let (b, a, beh) = parse_branch_header("feature/bar...origin/feature/bar [behind 5]");
        assert_eq!(b, "feature/bar");
        assert_eq!(a, 0);
        assert_eq!(beh, 5);

        let (b, a, beh) = parse_branch_header("No commits yet on main");
        assert_eq!(b, "main");
        assert_eq!(a, 0);
        assert_eq!(beh, 0);

        let (b, a, beh) = parse_branch_header("Initial commit on master");
        assert_eq!(b, "master");
        assert_eq!(a, 0);
        assert_eq!(beh, 0);

        let (b, a, beh) = parse_branch_header("HEAD (no branch)");
        assert_eq!(b, "HEAD");
        assert_eq!(a, 0);
        assert_eq!(beh, 0);

        let (b, a, beh) = parse_branch_header("dev");
        assert_eq!(b, "dev");
        assert_eq!(a, 0);
        assert_eq!(beh, 0);
    }

    #[test]
    fn test_parse_porcelain_v1() {
        let output = r#"## main...origin/main [ahead 2, behind 1]
M  src/lib.rs
 M src/main.rs
MM src/both.rs
A  src/new.rs
D  src/deleted.rs
?? untracked.txt
R  old.txt -> new.txt
"#;
        let res = parse_porcelain_v1(output);
        assert!(res.git_available);
        assert!(res.is_repo);
        assert_eq!(res.branch, "main");
        assert_eq!(res.ahead, 2);
        assert_eq!(res.behind, 1);

        // Staged
        assert_eq!(res.staged.len(), 5);
        assert_eq!(res.staged[0].path, "src/lib.rs");
        assert_eq!(res.staged[0].status, "modified");

        assert_eq!(res.staged[1].path, "src/both.rs");
        assert_eq!(res.staged[1].status, "modified");

        assert_eq!(res.staged[2].path, "src/new.rs");
        assert_eq!(res.staged[2].status, "added");

        assert_eq!(res.staged[3].path, "src/deleted.rs");
        assert_eq!(res.staged[3].status, "deleted");

        assert_eq!(res.staged[4].path, "new.txt");
        assert_eq!(res.staged[4].status, "renamed");
        assert_eq!(res.staged[4].old_path, Some("old.txt".to_string()));

        // Unstaged
        assert_eq!(res.unstaged.len(), 2);
        assert_eq!(res.unstaged[0].path, "src/main.rs");
        assert_eq!(res.unstaged[0].status, "modified");

        assert_eq!(res.unstaged[1].path, "src/both.rs");
        assert_eq!(res.unstaged[1].status, "modified");

        // Untracked
        assert_eq!(res.untracked.len(), 1);
        assert_eq!(res.untracked[0].path, "untracked.txt");
        assert_eq!(res.untracked[0].status, "untracked");
    }

    #[test]
    fn test_parse_git_branches() {
        let output = r#"* main
  feature/test
  remotes/origin/HEAD -> origin/main
  remotes/origin/main
  remotes/origin/feature/test
"#;
        let branches = parse_git_branches(output);
        assert_eq!(branches.len(), 4);

        assert_eq!(branches[0].name, "main");
        assert!(branches[0].current);
        assert!(!branches[0].is_remote);

        assert_eq!(branches[1].name, "feature/test");
        assert!(!branches[1].current);
        assert!(!branches[1].is_remote);

        assert_eq!(branches[2].name, "origin/main");
        assert!(!branches[2].current);
        assert!(branches[2].is_remote);

        assert_eq!(branches[3].name, "origin/feature/test");
        assert!(!branches[3].current);
        assert!(branches[3].is_remote);
    }

    #[test]
    fn test_clean_git_path() {
        assert_eq!(clean_git_path("foo/bar.txt"), "foo/bar.txt");
        assert_eq!(clean_git_path("\"foo bar.txt\""), "foo bar.txt");
    }

    #[test]
    fn test_utf8_safe_diff_truncation() {
        let multi_byte = "你好世界🌟".repeat(200_000); // multi-byte UTF-8 string
        if multi_byte.len() > MAX_DIFF_BYTES {
            let mut limit = MAX_DIFF_BYTES;
            while limit > 0 && !multi_byte.is_char_boundary(limit) {
                limit -= 1;
            }
            let mut truncated = multi_byte.clone();
            truncated.truncate(limit);
            assert!(truncated.is_char_boundary(limit));
        }
    }

    #[tokio::test]
    async fn test_git_discard_preserves_tracked_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path();

        let init_out = run_git_cmd(dir, ["init"]).await.unwrap();
        assert!(init_out.status.success());
        let _ = run_git_cmd(dir, ["config", "user.name", "TestUser"]).await;
        let _ = run_git_cmd(dir, ["config", "user.email", "test@example.com"]).await;

        let file_path = dir.join("tracked.txt");
        tokio::fs::write(&file_path, "Original content\n").await.unwrap();
        let _ = run_git_cmd(dir, ["add", "tracked.txt"]).await.unwrap();
        let _ = run_git_cmd(dir, ["commit", "-m", "commit original"]).await.unwrap();

        // Now modify the tracked file
        tokio::fs::write(&file_path, "Modified content\n").await.unwrap();

        // Discard changes
        let out = run_git_cmd(dir, ["checkout", "--", "tracked.txt"]).await.unwrap();
        assert!(out.status.success());

        // File MUST still exist and have "Original content\n", NOT deleted!
        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Original content\n");
    }

    #[tokio::test]
    async fn test_git_workflow_in_temp_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path();

        // 1. git init
        let init_out = run_git_cmd(dir, ["init"]).await.unwrap();
        assert!(init_out.status.success());

        // Configure dummy user for committing
        let _ = run_git_cmd(dir, ["config", "user.name", "TestUser"]).await;
        let _ = run_git_cmd(dir, ["config", "user.email", "test@example.com"]).await;

        // 2. Initial status: is_repo = true, untracked/staged empty
        let status_out = run_git_cmd(dir, ["status", "--porcelain=v1", "-b", "-uall"]).await.unwrap();
        let status = parse_porcelain_v1(&String::from_utf8_lossy(&status_out.stdout));
        assert!(status.git_available);
        assert!(status.is_repo);
        assert_eq!(status.staged.len(), 0);
        assert_eq!(status.untracked.len(), 0);

        // 3. Create a file
        let file_path = dir.join("hello.txt");
        tokio::fs::write(&file_path, "Hello world\n").await.unwrap();

        // 4. Status should show hello.txt in untracked
        let status_out = run_git_cmd(dir, ["status", "--porcelain=v1", "-b", "-uall"]).await.unwrap();
        let status = parse_porcelain_v1(&String::from_utf8_lossy(&status_out.stdout));
        assert_eq!(status.untracked.len(), 1);
        assert_eq!(status.untracked[0].path, "hello.txt");

        // 5. Stage hello.txt
        let stage_out = run_git_cmd(dir, ["add", "-A", "--", "hello.txt"]).await.unwrap();
        assert!(stage_out.status.success());

        let status_out = run_git_cmd(dir, ["status", "--porcelain=v1", "-b", "-uall"]).await.unwrap();
        let status = parse_porcelain_v1(&String::from_utf8_lossy(&status_out.stdout));
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].path, "hello.txt");
        assert_eq!(status.staged[0].status, "added");
        assert_eq!(status.untracked.len(), 0);

        // 6. Commit
        let commit_out = run_git_cmd(dir, ["commit", "-m", "Initial commit"]).await.unwrap();
        assert!(commit_out.status.success());

        let status_out = run_git_cmd(dir, ["status", "--porcelain=v1", "-b", "-uall"]).await.unwrap();
        let status = parse_porcelain_v1(&String::from_utf8_lossy(&status_out.stdout));
        assert_eq!(status.staged.len(), 0);
        assert_eq!(status.unstaged.len(), 0);
        assert_eq!(status.untracked.len(), 0);

        // 7. Modify file
        tokio::fs::write(&file_path, "Hello world modified\n").await.unwrap();
        let status_out = run_git_cmd(dir, ["status", "--porcelain=v1", "-b", "-uall"]).await.unwrap();
        let status = parse_porcelain_v1(&String::from_utf8_lossy(&status_out.stdout));
        assert_eq!(status.unstaged.len(), 1);
        assert_eq!(status.unstaged[0].path, "hello.txt");
        assert_eq!(status.unstaged[0].status, "modified");

        // 8. Create and checkout new branch
        let branch_out = run_git_cmd(dir, ["checkout", "-b", "feature-test"]).await.unwrap();
        assert!(branch_out.status.success());

        let branch_list_out = run_git_cmd(dir, ["branch", "-a"]).await.unwrap();
        let branches = parse_git_branches(&String::from_utf8_lossy(&branch_list_out.stdout));
        assert!(branches.iter().any(|b| b.name == "feature-test" && b.current));
    }
}
