use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoState {
    pub name: String,
    pub current_branch: String,
    pub last_updated: DateTime<Utc>,
    pub branches: Vec<BranchInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub owner: String, // Could be "me" or author name
    pub last_updated: DateTime<Utc>,
}
