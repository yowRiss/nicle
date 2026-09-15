use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub available: bool,
    pub node_version: Option<String>,
    pub npm_version: Option<String>,
    pub error: Option<String>,
}

fn check_command_version(cmd: &str) -> Option<String> {
    let output = Command::new(cmd).arg("-v").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn parse_major_version(v: &str) -> Option<u32> {
    let trimmed = v.trim_start_matches('v');
    let mut parts = trimmed.split('.');
    parts.next()?.parse::<u32>().ok()
}

pub fn check_node_runtime() -> RuntimeStatus {
    let node_v = check_command_version(if cfg!(windows) { "node.exe" } else { "node" });
    let npm_v = check_command_version(if cfg!(windows) { "npm.cmd" } else { "npm" });

    let node_major = node_v.as_deref().and_then(parse_major_version);
    let npm_major = npm_v.as_deref().and_then(parse_major_version);

    if node_v.is_none() {
        return RuntimeStatus {
            available: false,
            node_version: None,
            npm_version: npm_v,
            error: Some("Node.js is not installed or not found in system PATH. Install Node.js 18+ to use this companion app.".to_string()),
        };
    }

    if npm_v.is_none() {
        return RuntimeStatus {
            available: false,
            node_version: node_v,
            npm_version: None,
            error: Some("npm is not installed or not found in system PATH. Install npm 9+ to manage companion apps.".to_string()),
        };
    }

    if let Some(major) = node_major {
        if major < 18 {
            return RuntimeStatus {
                available: false,
                node_version: node_v,
                npm_version: npm_v,
                error: Some(format!("Node.js version {major} is unsupported. Node.js 18+ is required.")),
            };
        }
    }

    if let Some(major) = npm_major {
        if major < 9 {
            return RuntimeStatus {
                available: false,
                node_version: node_v,
                npm_version: npm_v,
                error: Some(format!("npm version {major} is unsupported. npm 9+ is required.")),
            };
        }
    }

    RuntimeStatus {
        available: true,
        node_version: node_v,
        npm_version: npm_v,
        error: None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BunRuntimeStatus {
    pub available: bool,
    pub bun_version: Option<String>,
    pub error: Option<String>,
}

pub fn check_bun_runtime() -> BunRuntimeStatus {
    let bun_v = check_command_version(if cfg!(windows) { "bun.exe" } else { "bun" });
    if bun_v.is_none() {
        return BunRuntimeStatus {
            available: false,
            bun_version: None,
            error: Some("Bun is not installed or not found in system PATH. Install Bun (https://bun.sh) to run this plugin.".to_string()),
        };
    }

    let major = bun_v.as_deref().and_then(parse_major_version);
    if let Some(major_ver) = major {
        if major_ver < 1 {
            return BunRuntimeStatus {
                available: false,
                bun_version: bun_v,
                error: Some(format!("Bun version {major_ver} is unsupported. Bun 1.0+ is required.")),
            };
        }
    }

    BunRuntimeStatus {
        available: true,
        bun_version: bun_v,
        error: None,
    }
}

pub fn check_runtime_for_type(
    runtime_type: &crate::plugins::manifest::RuntimeType,
    _min_version: Option<&str>,
) -> (bool, Option<String>, Option<String>) {
    use crate::plugins::manifest::RuntimeType;
    match runtime_type {
        RuntimeType::Bun => {
            let status = check_bun_runtime();
            (status.available, status.bun_version, status.error)
        }
        RuntimeType::Node => {
            let status = check_node_runtime();
            (status.available, status.node_version, status.error)
        }
        RuntimeType::Python => {
            let python_cmd = if cfg!(windows) { "python.exe" } else { "python3" };
            let ver = check_command_version(python_cmd);
            if let Some(v) = ver {
                (true, Some(v), None)
            } else {
                (false, None, Some("Python 3 is not found in PATH".to_string()))
            }
        }
        RuntimeType::Binary | RuntimeType::System => (true, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_major_version_works() {
        assert_eq!(parse_major_version("v22.23.1"), Some(22));
        assert_eq!(parse_major_version("v18.0.0"), Some(18));
        assert_eq!(parse_major_version("10.9.8"), Some(10));
        assert_eq!(parse_major_version("invalid"), None);
    }

    #[test]
    fn system_node_runtime_is_detected() {
        let status = check_node_runtime();
        assert!(status.available, "System Node.js runtime should be detected and compatible");
        assert!(status.node_version.is_some());
        assert!(status.npm_version.is_some());
    }

    #[test]
    fn system_bun_runtime_is_detected() {
        let status = check_bun_runtime();
        assert!(status.available, "System Bun runtime should be detected: {:?}", status.error);
        assert!(status.bun_version.is_some());
    }
}

