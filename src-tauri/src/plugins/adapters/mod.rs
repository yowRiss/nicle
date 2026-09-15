pub mod manifest_adapter;
pub mod nine_router;

use crate::error::{message, Result};
use crate::plugins::manifest::PluginManifest;
use std::path::{Path, PathBuf};

pub enum AppAdapter {
    NineRouter,
    Manifest(PluginManifest),
}

pub fn get_adapter(app_id: &str) -> Result<AppAdapter> {
    match app_id {
        "9router" => Ok(AppAdapter::NineRouter),
        _ => Err(message(format!("No static adapter registered for app '{app_id}'"))),
    }
}

pub fn get_adapter_for_manifest(manifest: PluginManifest) -> AppAdapter {
    AppAdapter::Manifest(manifest)
}

impl AppAdapter {
    pub fn resolve_launch_config(
        &self,
        install_dir: &Path,
        port: u16,
    ) -> Result<nine_router::AdapterLaunchConfig> {
        match self {
            AppAdapter::NineRouter => nine_router::resolve_launch_config(install_dir, port),
            AppAdapter::Manifest(m) => manifest_adapter::resolve_launch_config(m, install_dir, port),
        }
    }

    pub fn check_readiness(&self, port: u16) -> bool {
        match self {
            AppAdapter::NineRouter => nine_router::check_readiness(port),
            AppAdapter::Manifest(m) => manifest_adapter::check_readiness(m, port),
        }
    }

    pub fn get_dashboard_url(&self, port: u16) -> String {
        match self {
            AppAdapter::NineRouter => nine_router::get_dashboard_url(port),
            AppAdapter::Manifest(m) => manifest_adapter::get_dashboard_url(m, port),
        }
    }

    pub fn get_app_data_paths(&self) -> Vec<PathBuf> {
        match self {
            AppAdapter::NineRouter => nine_router::get_app_data_paths(),
            AppAdapter::Manifest(m) => manifest_adapter::get_app_data_paths(m),
        }
    }
}

