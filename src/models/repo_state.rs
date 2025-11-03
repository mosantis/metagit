use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Number of commits per author on this branch
    #[serde(default)]
    pub commit_stats: HashMap<String, usize>,
    /// SHA of the last commit we processed (for incremental updates)
    #[serde(default)]
    pub last_commit_sha: Option<String>,
}

impl BranchInfo {
    /// Calculate the owner based on commit statistics
    /// Returns "Author" if single author, or "Author et al" if multiple significant contributors
    pub fn calculate_owner(&self) -> String {
        if self.commit_stats.is_empty() {
            return "unknown".to_string();
        }

        let total_commits: usize = self.commit_stats.values().sum();
        if total_commits == 0 {
            return "unknown".to_string();
        }

        // Find the author with the most commits
        let mut authors: Vec<_> = self.commit_stats.iter().collect();
        authors.sort_by(|a, b| b.1.cmp(a.1));

        let primary_author = authors[0].0;

        // Check if there are other significant contributors (>5% of commits)
        let threshold = (total_commits as f64 * 0.05).ceil() as usize;
        let has_other_contributors = authors.iter().skip(1).any(|(_, &count)| count >= threshold);

        if has_other_contributors {
            format!("{} et al", primary_author)
        } else {
            primary_author.to_string()
        }
    }

    /// Get the number of commits by the primary owner
    pub fn get_owner_commit_count(&self) -> usize {
        if self.commit_stats.is_empty() {
            return 0;
        }

        // Find the author with the most commits
        let mut authors: Vec<_> = self.commit_stats.iter().collect();
        authors.sort_by(|a, b| b.1.cmp(a.1));

        *authors[0].1
    }
}
