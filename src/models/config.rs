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
}

fn default_type() -> String {
    "sh".to_string()
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
