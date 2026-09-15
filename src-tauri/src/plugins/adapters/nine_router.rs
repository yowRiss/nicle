use crate::error::{message, Result};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct AdapterLaunchConfig {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

pub fn resolve_launch_config(install_dir: &Path, port: u16) -> Result<AdapterLaunchConfig> {
    let pkg_root = install_dir.join("node_modules").join("9router");
    if !pkg_root.exists() {
        return Err(message(format!(
            "9Router package root not found at {}",
            pkg_root.display()
        )));
    }

    let app_dir = pkg_root.join("app");
    let custom_server = app_dir.join("custom-server.js");
    let server_js = app_dir.join("server.js");
    let cli_js = pkg_root.join("cli.js");

    let (script, cwd) = if custom_server.exists() {
        (custom_server, app_dir.clone())
    } else if server_js.exists() {
        (server_js, app_dir.clone())
    } else if cli_js.exists() {
        (cli_js, pkg_root.clone())
    } else {
        return Err(message("No runnable 9Router entrypoint found"));
    };

    let program = if cfg!(windows) { "node.exe" } else { "node" }.to_string();
    let args = vec![
        "--dns-result-order=ipv4first".to_string(),
        "--max-old-space-size=6144".to_string(),
        script.to_string_lossy().to_string(),
    ];

    let mut env = Vec::new();
    env.push(("PORT".to_string(), port.to_string()));
    env.push(("HOSTNAME".to_string(), "127.0.0.1".to_string()));

    let app_nm = app_dir.join("node_modules");
    let root_nm = install_dir.join("node_modules");
    let delimiter = if cfg!(windows) { ";" } else { ":" };
    let node_path = format!("{}{}{}", app_nm.display(), delimiter, root_nm.display());
    env.push(("NODE_PATH".to_string(), node_path));

    Ok(AdapterLaunchConfig {
        program,
        args,
        cwd,
        env,
    })
}

pub fn check_readiness(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    let Ok(mut stream) = TcpStream::connect_timeout(
        &match addr.parse() {
            Ok(sa) => sa,
            Err(_) => return false,
        },
        Duration::from_millis(500),
    ) else {
        return false;
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(1000)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(1000)));

    let request = format!(
        "GET /dashboard HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut buf = [0u8; 512];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let response = String::from_utf8_lossy(&buf[..n]);
            // Accept 200 OK or 307/302/308 redirect
            response.starts_with("HTTP/1.1 200")
                || response.starts_with("HTTP/1.1 307")
                || response.starts_with("HTTP/1.1 302")
                || response.starts_with("HTTP/1.1 308")
                || response.starts_with("HTTP/1.0 200")
        }
        _ => false,
    }
}

pub fn get_dashboard_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/dashboard")
}

pub fn get_app_data_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join(".9router"));
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            paths.push(PathBuf::from(appdata).join("9router"));
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_url_uses_loopback_only() {
        let url = get_dashboard_url(20128);
        assert_eq!(url, "http://127.0.0.1:20128/dashboard");
        assert!(url.starts_with("http://127.0.0.1:"));
    }

    #[test]
    fn app_data_paths_point_to_user_home() {
        let paths = get_app_data_paths();
        assert!(!paths.is_empty(), "Data paths must include user's 9router directory");
        assert!(paths[0].to_string_lossy().contains(".9router"));
    }
}
