use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use git2::{BranchType, Oid, Repository};
use std::collections::HashMap;
use std::path::Path;

use crate::models::{BranchInfo, RepoState};

pub fn get_repo_state(repo_path: &Path, repo_name: &str) -> Result<RepoState> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open repository at {:?}", repo_path))?;

    let head = repo.head()?;
    let current_branch = head.shorthand().unwrap_or("(detached)").to_string();

    let mut branches = Vec::new();

    // Get all local branches
    for branch in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let name = branch.name()?.unwrap_or("(invalid utf8)").to_string();

        // Get the last commit time for this branch
        let reference = branch.get();
        let commit = reference.peel_to_commit()?;
        let time = commit.time();
        let timestamp = time.seconds();
        let last_updated = DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now);

        // Try to extract owner from branch name (e.g., "feature/user/something")
        let owner = extract_owner(&name);

        branches.push(BranchInfo {
            name: name.clone(),
            owner,
            last_updated,
            commit_stats: HashMap::new(),
            last_commit_sha: None,
        });
    }

    // Sort branches by last updated (most recent first)
    branches.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    let last_updated = branches
        .first()
        .map(|b| b.last_updated)
        .unwrap_or_else(Utc::now);

    Ok(RepoState {
        name: repo_name.to_string(),
        current_branch,
        last_updated,
        branches,
    })
}

fn extract_owner(branch_name: &str) -> String {
    // Simple heuristic: if branch contains a slash, take the first part
    // Otherwise just return "me" for now
    if let Some(pos) = branch_name.find('/') {
        branch_name[..pos].to_string()
    } else {
        "me".to_string()
    }
}

/// Find the main branch (master or main)
fn find_main_branch(repo: &Repository) -> Option<Oid> {
    // Try "master" first, then "main"
    for branch_name in &["master", "main"] {
        let ref_name = format!("refs/heads/{}", branch_name);
        if let Ok(reference) = repo.find_reference(&ref_name) {
            if let Some(oid) = reference.target() {
                return Some(oid);
            }
        }
    }
    None
}

/// Collect commit statistics for a branch
/// Only counts commits that are NOT in the main branch (master/main)
/// Returns (commit_stats, last_commit_sha, last_updated_time)
fn collect_branch_stats(
    repo: &Repository,
    branch_name: &str,
    branch_oid: Oid,
) -> Result<(HashMap<String, usize>, String, DateTime<Utc>)> {
    let mut commit_stats = HashMap::new();
    let mut revwalk = repo.revwalk()?;

    // Start from the branch tip
    revwalk.push(branch_oid)?;

    // Find and hide commits from main branch (to only count unmerged commits)
    // Skip this for the main branch itself
    let main_branch_names = ["master", "main"];
    if !main_branch_names.contains(&branch_name) {
        if let Some(main_oid) = find_main_branch(repo) {
            // Hide all commits in main branch
            revwalk.hide(main_oid)?;
        }
    }

    // Note: We don't use incremental updates (last_commit_sha) for unmerged commits
    // because the main branch can change, making the old counts invalid.
    // We always recalculate unmerged commits from scratch.

    let mut last_commit_time = Utc::now();
    let mut last_sha = branch_oid.to_string();
    let mut first_commit = true;

    // Walk through commits
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Get author name
        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();

        // Increment commit count for this author
        *commit_stats.entry(author_name).or_insert(0) += 1;

        // Capture the time of the first (most recent) commit
        if first_commit {
            let time = commit.time();
            last_commit_time = DateTime::from_timestamp(time.seconds(), 0).unwrap_or_else(Utc::now);
            last_sha = oid.to_string();
            first_commit = false;
        }
    }

    Ok((commit_stats, last_sha, last_commit_time))
}

/// Refresh repository state with commit statistics
/// Note: Always recalculates from scratch to ensure accurate unmerged commit counts
pub fn refresh_repo_state(
    repo_path: &Path,
    repo_name: &str,
    _previous_state: Option<&RepoState>,
) -> Result<RepoState> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open repository at {:?}", repo_path))?;

    let head = repo.head()?;
    let current_branch = head.shorthand().unwrap_or("(detached)").to_string();

    let mut branches = Vec::new();

    // Get all local branches
    for branch in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let name = branch.name()?.unwrap_or("(invalid utf8)").to_string();

        // Get the branch reference
        let reference = branch.get();
        let branch_oid = reference.target().context("Branch has no target")?;

        // Collect commit stats (only unmerged commits from main branch)
        // We always recalculate from scratch since main branch can change
        let (commit_stats, last_sha, last_updated) =
            collect_branch_stats(&repo, &name, branch_oid)?;

        // Calculate owner based on commit stats
        let temp_branch = BranchInfo {
            name: name.clone(),
            owner: String::new(), // Will be calculated
            last_updated,
            commit_stats: commit_stats.clone(),
            last_commit_sha: Some(last_sha.clone()),
        };

        let owner = temp_branch.calculate_owner();

        branches.push(BranchInfo {
            name,
            owner,
            last_updated,
            commit_stats,
            last_commit_sha: Some(last_sha),
        });
    }

    // Sort branches by last updated (most recent first)
    branches.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    let last_updated = branches
        .first()
        .map(|b| b.last_updated)
        .unwrap_or_else(Utc::now);

    Ok(RepoState {
        name: repo_name.to_string(),
        current_branch,
        last_updated,
        branches,
    })
}

pub fn pull_repo(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)?;

    // Get the current branch
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD");

    // Fetch
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&[branch_name], None, None)?;

    // Get fetch head
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

    // Merge
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.0.is_up_to_date() {
        return Ok("Already up-to-date".to_string());
    } else if analysis.0.is_fast_forward() {
        // Fast-forward merge
        let refname = format!("refs/heads/{}", branch_name);
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        return Ok("Fast-forwarded".to_string());
    } else if analysis.0.is_normal() {
        return Ok("Normal merge required (not implemented)".to_string());
    }

    Ok("Unknown state".to_string())
}

pub fn push_repo(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)?;

    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD");

    let mut remote = repo.find_remote("origin")?;
    let refspec = format!("refs/heads/{}", branch_name);

    remote.push(&[&refspec], None)?;

    Ok(format!("Pushed {}", branch_name))
}

pub fn is_git_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

pub fn get_repo_url(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)?;
    let remote = repo.find_remote("origin")?;
    let url = remote.url().unwrap_or("(no url)").to_string();
    Ok(url)
}

/// Get the current git user's name from global config
#[allow(dead_code)]
pub fn get_current_user() -> Result<String> {
    let config = git2::Config::open_default()?;
    let name = config
        .get_string("user.name")
        .context("Failed to get user.name from git config")?;
    Ok(name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchStatus {
    /// Branch is fully synced (green)
    Synced,
    /// Branch has local changes or commits to push (red)
    NeedsPush,
    /// Branch has remote commits to pull (orange)
    NeedsPull,
}

/// Check if repository has uncommitted changes
pub fn has_uncommitted_changes(repo_path: &Path) -> Result<bool> {
    let repo = Repository::open(repo_path)?;

    // Check for changes in working directory and index
    let statuses = repo.statuses(None)?;

    // If there are any status entries, we have uncommitted changes
    Ok(!statuses.is_empty())
}

/// Get the sync status of a branch relative to its remote
/// Returns (commits_ahead, commits_behind)
pub fn get_branch_sync_status(repo_path: &Path, branch_name: &str) -> Result<(usize, usize)> {
    let repo = Repository::open(repo_path)?;

    // Get local branch reference
    let local_ref_name = format!("refs/heads/{}", branch_name);
    let local_ref = match repo.find_reference(&local_ref_name) {
        Ok(r) => r,
        Err(_) => return Ok((0, 0)), // Branch doesn't exist locally
    };

    let local_oid = match local_ref.target() {
        Some(oid) => oid,
        None => return Ok((0, 0)),
    };

    // Try to find remote tracking branch
    let remote_ref_name = format!("refs/remotes/origin/{}", branch_name);
    let remote_oid = match repo.find_reference(&remote_ref_name) {
        Ok(remote_ref) => match remote_ref.target() {
            Some(oid) => oid,
            None => return Ok((0, 0)),
        },
        Err(_) => return Ok((0, 0)), // No remote tracking branch
    };

    // Use git2 to count commits ahead and behind
    let (ahead, behind) = repo.graph_ahead_behind(local_oid, remote_oid)?;

    Ok((ahead, behind))
}

/// Determine the overall status of a branch for coloring
pub fn get_branch_status(repo_path: &Path, branch_name: &str) -> Result<BranchStatus> {
    // Check for uncommitted changes first
    if has_uncommitted_changes(repo_path)? {
        return Ok(BranchStatus::NeedsPush);
    }

    // Check sync status with remote
    let (ahead, behind) = get_branch_sync_status(repo_path, branch_name)?;

    if behind > 0 {
        // Has remote commits to pull (takes priority)
        Ok(BranchStatus::NeedsPull)
    } else if ahead > 0 {
        // Has local commits to push
        Ok(BranchStatus::NeedsPush)
    } else {
        // Fully synced
        Ok(BranchStatus::Synced)
    }
}
