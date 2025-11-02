use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use git2::{BranchType, Repository};
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
