use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub repositories: Vec<Repository>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub shells: ShellConfig,
    /// SSH credentials: maps hostname (e.g., "github.com") to SSH private key path (e.g., "~/.ssh/id_github")
    #[serde(default)]
    pub credentials: HashMap<String, String>,
    /// User aliases: maps canonical user name to list of aliases (names and emails)
    /// Example: "John" -> ["John Crammer", "JC", "john.crammer@company.com"]
    #[serde(default)]
    pub users: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShellConfig {
    /// Shell executable to use for .sh scripts (default: "sh" on Unix, "bash" if available)
    #[serde(default = "default_shell")]
    pub sh: String,
    /// Command prompt executable to use for .bat/.cmd scripts (default: "cmd")
    #[serde(default = "default_cmd")]
    pub cmd: String,
    /// PowerShell executable to use for .ps1 scripts (default: "powershell")
    #[serde(default = "default_powershell")]
    pub powershell: String,
}

fn default_shell() -> String {
    "sh".to_string()
}

fn default_cmd() -> String {
    "cmd".to_string()
}

fn default_powershell() -> String {
    "powershell".to_string()
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            sh: default_shell(),
            cmd: default_cmd(),
            powershell: default_powershell(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repository {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub name: String,
    pub steps: Vec<TaskStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskStep {
    #[serde(rename = "type", default = "default_type")]
    pub step_type: String,
    pub repo: String,
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Platform(s) this step should run on: "windows", "linux", "macos", or "all" (default)
    #[serde(default = "default_platform")]
    pub platform: String,
}

fn default_type() -> String {
    String::new() // Empty string means infer from extension
}

fn default_platform() -> String {
    "all".to_string()
}

impl TaskStep {
    /// Check if this step should run on the current platform
    pub fn should_run_on_current_platform(&self) -> bool {
        if self.platform == "all" {
            return true;
        }

        let current_platform = std::env::consts::OS;

        // Handle comma-separated platform list (e.g., "windows,linux")
        self.platform
            .split(',')
            .map(|s| s.trim())
            .any(|p| p == current_platform || p == "all")
    }
}

impl Config {
    /// Get the path to the global configuration file in user's home directory
    pub fn global_config_path() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".mgitconfig.json"))
    }

    /// Load configuration with fallback hierarchy:
    /// 1. Try local project config
    /// 2. If not found or if only loading shells, try global config
    /// 3. Fall back to defaults
    pub fn load(path: &str) -> anyhow::Result<Self> {
        // Try to load local config
        let local_config = if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            Some(serde_json::from_str::<Config>(&content)?)
        } else {
            None
        };

        // Try to load global config for shell settings
        let global_config = if let Some(global_path) = Self::global_config_path() {
            if global_path.exists() {
                match std::fs::read_to_string(&global_path) {
                    Ok(content) => serde_json::from_str::<Config>(&content).ok(),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        // Merge configurations: local takes precedence, but use global shells and credentials if local doesn't specify
        match (local_config, global_config) {
            (Some(mut local), Some(global)) => {
                // If local config has default shells, use global shells
                if local.shells.sh == "sh" && global.shells.sh != "sh" {
                    local.shells.sh = global.shells.sh;
                }
                if local.shells.cmd == "cmd" && global.shells.cmd != "cmd" {
                    local.shells.cmd = global.shells.cmd;
                }
                if local.shells.powershell == "powershell" && global.shells.powershell != "powershell" {
                    local.shells.powershell = global.shells.powershell;
                }
                // Merge credentials from global config (global credentials as fallback)
                for (host, key_path) in global.credentials {
                    local.credentials.entry(host).or_insert(key_path);
                }
                // Merge users from global config (global users as fallback)
                for (canonical, aliases) in global.users {
                    local.users.entry(canonical).or_insert(aliases);
                }
                Ok(local)
            }
            (Some(local), None) => Ok(local),
            (None, _) => anyhow::bail!("Configuration file '{}' not found", path),
        }
    }

    /// Load only global configuration
    #[allow(dead_code)]
    pub fn load_global() -> anyhow::Result<Option<Self>> {
        if let Some(global_path) = Self::global_config_path() {
            if global_path.exists() {
                let content = std::fs::read_to_string(&global_path)?;
                let config: Config = serde_json::from_str(&content)?;
                return Ok(Some(config));
            }
        }
        Ok(None)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Normalize a user name or email to its canonical form
    /// Returns the canonical name if a match is found, otherwise returns the input unchanged
    pub fn normalize_user(&self, author: &str) -> String {
        // Check each canonical user and their aliases
        for (canonical, aliases) in &self.users {
            // Case-insensitive comparison
            let author_lower = author.to_lowercase();

            // Check if the author matches the canonical name itself
            if canonical.to_lowercase() == author_lower {
                return canonical.clone();
            }

            // Check if the author matches any alias
            for alias in aliases {
                if alias.to_lowercase() == author_lower {
                    return canonical.clone();
                }
            }
        }

        // No match found, return original
        author.to_string()
    }
}
