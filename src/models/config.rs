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
    /// Saved tags: maps tag name to repository branches
    /// Example: "release-1.0" -> {"frontend" -> "release/1.0", "backend" -> "release/1.0"}
    #[serde(default)]
    pub tags: HashMap<String, HashMap<String, String>>,
    /// Directory where the config file was loaded from (used to resolve relative paths)
    /// Not serialized - this is metadata about where we loaded from
    #[serde(skip)]
    pub config_dir: Option<std::path::PathBuf>,
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
        dirs::home_dir().map(|home| home.join(".mgitconfig.yaml"))
    }

    /// Resolve a repository path relative to the config file's directory
    /// If config_dir is not set, returns the path as-is
    pub fn resolve_repo_path(&self, repo_name: &str) -> std::path::PathBuf {
        if let Some(config_dir) = &self.config_dir {
            config_dir.join(repo_name)
        } else {
            std::path::PathBuf::from(repo_name)
        }
    }

    /// Get the database path relative to the config file's directory
    /// Returns ".mgitdb" in the same directory as .mgitconfig.yaml
    pub fn get_db_path(&self) -> std::path::PathBuf {
        if let Some(config_dir) = &self.config_dir {
            config_dir.join(".mgitdb")
        } else {
            std::path::PathBuf::from(".mgitdb")
        }
    }

    /// Search for .mgitconfig.yaml starting from current directory and walking up
    /// Stops at $HOME (does not use $HOME/.mgitconfig.yaml as project config)
    pub fn find_project_config() -> Option<std::path::PathBuf> {
        use std::env;

        // Get home directory to know when to stop
        let home_dir = dirs::home_dir()?;

        // Start from current directory
        let mut current_dir = env::current_dir().ok()?;

        loop {
            // Check if .mgitconfig.yaml exists in current directory
            let config_path = current_dir.join(".mgitconfig.yaml");
            if config_path.exists() {
                // Don't use $HOME/.mgitconfig.yaml as project config
                if current_dir != home_dir {
                    return Some(config_path);
                }
            }

            // Try to go up one directory
            match current_dir.parent() {
                Some(parent) => {
                    // Stop if we've reached home directory
                    if current_dir == home_dir {
                        break;
                    }
                    current_dir = parent.to_path_buf();
                }
                None => break, // Reached filesystem root
            }
        }

        None
    }

    /// Load configuration by discovering project config (searching upward from current directory)
    /// Falls back to global config if no project config is found
    pub fn load_from_project() -> anyhow::Result<Self> {
        // Try to find project config by searching upward
        if let Some(project_config_path) = Self::find_project_config() {
            // Use the discovered project config path
            return Self::load(project_config_path.to_str().unwrap_or(".mgitconfig.yaml"));
        }

        // No project config found - error out
        anyhow::bail!("No .mgitconfig.yaml found in current directory or parent directories.\nRun 'mgit init' to create one.")
    }

    /// Load configuration with fallback hierarchy:
    /// 1. Try local project config
    /// 2. If not found or if only loading shells, try global config
    /// 3. Fall back to defaults
    pub fn load(path: &str) -> anyhow::Result<Self> {
        // Get the directory containing the config file for resolving relative paths
        let config_path = std::path::Path::new(path);
        let config_dir = config_path.parent().map(|p| p.to_path_buf());

        // Try to load local config
        let local_config = if config_path.exists() {
            let content = std::fs::read_to_string(path)?;
            let mut config: Config = serde_yaml::from_str(&content)?;
            config.config_dir = config_dir.clone();
            Some(config)
        } else {
            None
        };

        // Try to load global config for shell settings
        let global_config = if let Some(global_path) = Self::global_config_path() {
            if global_path.exists() {
                match std::fs::read_to_string(&global_path) {
                    Ok(content) => serde_yaml::from_str::<Config>(&content).ok(),
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
                let config: Config = serde_yaml::from_str(&content)?;
                return Ok(Some(config));
            }
        }
        Ok(None)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Normalize a user name or email to its canonical form
    /// Returns the canonical name if a match is found, otherwise returns the input unchanged
    #[allow(dead_code)]
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

    /// Check if an author identity (name or email) is already mapped in users config
    #[allow(dead_code)]
    pub fn is_author_mapped(&self, author_name: &str, author_email: &str) -> bool {
        let name_lower = author_name.to_lowercase();
        let email_lower = author_email.to_lowercase();

        for (canonical, aliases) in &self.users {
            // Check if name matches canonical
            if canonical.to_lowercase() == name_lower {
                return true;
            }

            // Check if name or email matches any alias
            for alias in aliases {
                let alias_lower = alias.to_lowercase();
                if alias_lower == name_lower || alias_lower == email_lower {
                    return true;
                }
            }
        }

        false
    }

    /// Add new unmapped author identities to the users section
    /// Uses case-insensitive matching to avoid duplicates
    /// Returns true if a new entry or alias was added, false if it already existed
    pub fn add_unmapped_authors(&mut self, name: String, email: String) -> bool {
        let name_lower = name.to_lowercase();
        let email_lower = email.to_lowercase();

        // First pass: find which canonical entry (if any) this name/email belongs to
        let mut target_canonical: Option<String> = None;
        let mut should_add_email = false;
        let mut should_add_name = false;

        for (canonical_key, aliases) in &self.users {
            // Check if name matches canonical key
            if canonical_key.to_lowercase() == name_lower {
                target_canonical = Some(canonical_key.clone());
                // Check if we need to add the email
                should_add_email = !email_lower.is_empty() &&
                                 !aliases.iter().any(|a| a.to_lowercase() == email_lower);
                break;
            }

            // Check if name matches any alias
            if aliases.iter().any(|a| a.to_lowercase() == name_lower) {
                target_canonical = Some(canonical_key.clone());
                // Check if we need to add the email
                should_add_email = !email_lower.is_empty() &&
                                 !aliases.iter().any(|a| a.to_lowercase() == email_lower);
                break;
            }

            // Check if email matches any alias (and email is not empty)
            if !email_lower.is_empty() && aliases.iter().any(|a| a.to_lowercase() == email_lower) {
                target_canonical = Some(canonical_key.clone());
                // Check if we need to add the name
                should_add_name = canonical_key.to_lowercase() != name_lower &&
                                !aliases.iter().any(|a| a.to_lowercase() == name_lower);
                break;
            }
        }

        // Second pass: perform the mutation if we found a matching entry
        if let Some(canonical) = target_canonical {
            let mut added = false;
            if let Some(aliases) = self.users.get_mut(&canonical) {
                if should_add_email {
                    aliases.push(email);
                    added = true;
                }
                if should_add_name {
                    aliases.push(name);
                    added = true;
                }
            }
            return added;
        }

        // Neither name nor email is mapped anywhere, create new entry
        self.users.insert(name, vec![email]);
        true
    }
}
