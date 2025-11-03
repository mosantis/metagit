use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub repositories: Vec<Repository>,
    #[serde(default)]
    pub tasks: Vec<Task>,
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
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
