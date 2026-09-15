use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPlugin {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub publisher: String,
    pub npm_package: String,
    pub pinned_version: String,
    pub runtime_requirement: String,
    pub default_port: u16,
    pub homepage: String,
    pub documentation: String,
    pub permissions_disclosure: String,
    pub install_scripts_required: bool,
}

pub fn get_catalog() -> Vec<CatalogPlugin> {
    vec![
        CatalogPlugin {
            id: "9router".to_string(),
            name: "9Router".to_string(),
            tagline: "Local AI routing and gateway companion app".to_string(),
            description: "A local companion service that routes LLM requests across multiple AI providers, manages credentials, and tracks usage. Runs locally as an isolated Node.js process.".to_string(),
            publisher: "decolua".to_string(),
            npm_package: "9router".to_string(),
            pinned_version: "0.5.69".to_string(),
            runtime_requirement: "Node.js >= 18.0.0, npm >= 9.0.0".to_string(),
            default_port: 20128,
            homepage: "https://github.com/decolua/9router".to_string(),
            documentation: "https://github.com/decolua/9router/blob/master/cli/README.md".to_string(),
            permissions_disclosure: "Executes as a supervised local process with user permissions. Binds exclusively to the loopback interface (127.0.0.1). Provider keys and dashboard settings remain in 9Router's own local data storage.".to_string(),
            install_scripts_required: false,
        },
        CatalogPlugin {
            id: "discord-presence".to_string(),
            name: "Discord Rich Presence".to_string(),
            tagline: "Live Discord status for active file, workspace, line count, and coding time".to_string(),
            description: "A supervised local companion service connecting to your local Discord desktop client via Discord IPC. Displays your active workspace, current file, language icons, cursor coordinates, and elapsed coding time on your Discord profile with zero external dependencies and full privacy controls.".to_string(),
            publisher: "Nicle Community".to_string(),
            npm_package: "discord-presence".to_string(),
            pinned_version: "1.0.0".to_string(),
            runtime_requirement: "Node.js >= 18.0.0 or Bun >= 1.0.0".to_string(),
            default_port: 3070,
            homepage: "https://github.com/nicle-ide/nicle".to_string(),
            documentation: "https://github.com/nicle-ide/nicle/tree/main/plugins/discord-presence".to_string(),
            permissions_disclosure: "Connects to local Discord client via IPC socket/pipe (127.0.0.1 loopback only). Never transmits code content or connects to external servers.".to_string(),
            install_scripts_required: false,
        },
    ]
}

pub fn find_catalog_plugin(id: &str) -> Option<CatalogPlugin> {
    get_catalog().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_genuine_9router_entry() {
        let entry = find_catalog_plugin("9router");
        assert!(entry.is_some(), "9Router must exist in curated catalog");
        let plugin = entry.unwrap();
        assert_eq!(plugin.name, "9Router");
        assert_eq!(plugin.npm_package, "9router");
        assert_eq!(plugin.pinned_version, "0.5.69");
        assert_eq!(plugin.default_port, 20128);
        assert!(!plugin.install_scripts_required);
        assert!(plugin.permissions_disclosure.contains("127.0.0.1"));
    }

    #[test]
    fn catalog_contains_discord_presence_entry() {
        let entry = find_catalog_plugin("discord-presence");
        assert!(entry.is_some(), "Discord Rich Presence must exist in catalog");
        let plugin = entry.unwrap();
        assert_eq!(plugin.name, "Discord Rich Presence");
        assert_eq!(plugin.pinned_version, "1.0.0");
        assert_eq!(plugin.default_port, 3070);
        assert!(!plugin.install_scripts_required);
        assert!(plugin.permissions_disclosure.contains("127.0.0.1"));
    }

    #[test]
    fn catalog_unknown_returns_none() {
        assert!(find_catalog_plugin("non-existent-plugin").is_none());
    }
}
