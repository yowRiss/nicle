use crate::error::{message, Result};
use crate::plugins::adapters::nine_router::AdapterLaunchConfig;
use crate::plugins::manifest::{PluginManifest, RuntimeType};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn resolve_launch_config(
    manifest: &PluginManifest,
    install_dir: &Path,
    port: u16,
) -> Result<AdapterLaunchConfig> {
    let service = manifest
        .service
        .as_ref()
        .ok_or_else(|| message(format!("Plugin '{}' does not declare a service", manifest.id)))?;

    let entry_path = install_dir.join(&service.entry);
    if !entry_path.exists() {
        return Err(message(format!(
            "Plugin entry file '{}' does not exist in {}",
            service.entry,
            install_dir.display()
        )));
    }

    let runtime_type = manifest
        .runtime
        .as_ref()
        .map(|r| &r.runtime_type)
        .unwrap_or(&RuntimeType::Node);

    let (program, args) = match runtime_type {
        RuntimeType::Bun => {
            let prog = if cfg!(windows) { "bun.exe" } else { "bun" };
            (
                prog.to_string(),
                vec!["run".to_string(), entry_path.to_string_lossy().to_string()],
            )
        }
        RuntimeType::Node => {
            let prog = if cfg!(windows) { "node.exe" } else { "node" };
            (prog.to_string(), vec![entry_path.to_string_lossy().to_string()])
        }
        RuntimeType::Python => {
            let prog = if cfg!(windows) { "python.exe" } else { "python3" };
            (prog.to_string(), vec![entry_path.to_string_lossy().to_string()])
        }
        RuntimeType::Binary | RuntimeType::System => (
            entry_path.to_string_lossy().to_string(),
            vec![],
        ),
    };

    let mut env = vec![
        ("PORT".to_string(), port.to_string()),
        ("HOST".to_string(), "127.0.0.1".to_string()),
        ("HOSTNAME".to_string(), "127.0.0.1".to_string()),
        ("NICLE_PLUGIN_ID".to_string(), manifest.id.clone()),
    ];

    for (k, v) in &manifest.environment {
        env.push((k.clone(), v.clone()));
    }

    Ok(AdapterLaunchConfig {
        program,
        args,
        cwd: install_dir.to_path_buf(),
        env,
    })
}

pub fn check_readiness(manifest: &PluginManifest, port: u16) -> bool {
    let path = manifest
        .service
        .as_ref()
        .and_then(|s| s.readiness_path.as_deref())
        .unwrap_or("/");

    let addr = format!("127.0.0.1:{port}");
    let stream_result = TcpStream::connect_timeout(
        &match addr.parse() {
            Ok(a) => a,
            Err(_) => return false,
        },
        Duration::from_millis(600),
    );

    let mut stream = match stream_result {
        Ok(s) => s,
        Err(_) => return false,
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(600)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(600)));

    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUser-Agent: Nicle-Supervisor\r\nConnection: close\r\n\r\n"
    );

    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }

    let mut buf = [0u8; 512];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let response = String::from_utf8_lossy(&buf[..n]);
            response.starts_with("HTTP/1.0 2")
                || response.starts_with("HTTP/1.1 2")
                || response.starts_with("HTTP/1.0 3")
                || response.starts_with("HTTP/1.1 3")
        }
        _ => false,
    }
}

pub fn get_dashboard_url(manifest: &PluginManifest, port: u16) -> String {
    let path = manifest
        .service
        .as_ref()
        .and_then(|s| s.dashboard_path.as_deref())
        .unwrap_or("/");
    let path_clean = if path.starts_with('/') {
        path
    } else {
        &format!("/{path}")
    };
    format!("http://127.0.0.1:{port}{path_clean}")
}

pub fn get_app_data_paths(manifest: &PluginManifest) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    if let Some(home) = home {
        let candidate = home.join(format!(".{}", manifest.id));
        if candidate.exists() {
            paths.push(candidate);
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::{PluginType, ServiceSpec};

    #[test]
    fn test_manifest_dashboard_url() {
        let manifest = PluginManifest {
            id: "my-app".to_string(),
            name: "My App".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            author: None,
            homepage: None,
            plugin_type: PluginType::Service,
            runtime: None,
            install: None,
            service: Some(ServiceSpec {
                entry: "index.js".to_string(),
                default_port: 3000,
                readiness_path: Some("/health".to_string()),
                dashboard_path: Some("/app".to_string()),
                activity_path: None,
            }),
            commands: vec![],
            environment: Default::default(),
        };

        let url = get_dashboard_url(&manifest, 3000);
        assert_eq!(url, "http://127.0.0.1:3000/app");
    }
}
