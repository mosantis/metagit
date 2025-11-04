use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use git2::{BranchType, Cred, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, Status};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::{BranchInfo, RepoState};

/// Debug logging macro - only prints if debug is true
macro_rules! debug_log {
    ($debug:expr, $($arg:tt)*) => {
        if $debug {
            println!("{} {}", "  [DEBUG]".bright_black(), format!($($arg)*).bright_black());
        }
    };
}

/// Represents a unique author identity (name + email)
/// Stores names and emails in their original case, but uses case-insensitive comparison
#[derive(Debug, Clone)]
pub struct AuthorIdentity {
    pub name: String,
    pub email: String,
}

impl PartialEq for AuthorIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.name.to_lowercase() == other.name.to_lowercase() &&
        self.email.to_lowercase() == other.email.to_lowercase()
    }
}

impl Eq for AuthorIdentity {}

impl std::hash::Hash for AuthorIdentity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.to_lowercase().hash(state);
        self.email.to_lowercase().hash(state);
    }
}

/// Extract hostname from git URL (e.g., "git@github.com:..." -> "github.com")
fn extract_hostname(url: &str) -> Option<String> {
    // Handle SSH URLs like git@github.com:org/repo.git
    if url.starts_with("git@") || url.starts_with("ssh://") {
        let without_prefix = url.strip_prefix("git@").unwrap_or(url);
        let without_prefix = without_prefix.strip_prefix("ssh://").unwrap_or(without_prefix);

        if let Some(colon_pos) = without_prefix.find(':') {
            return Some(without_prefix[..colon_pos].to_string());
        } else if let Some(slash_pos) = without_prefix.find('/') {
            return Some(without_prefix[..slash_pos].to_string());
        }
    }

    // Handle HTTPS URLs
    if url.starts_with("https://") || url.starts_with("http://") {
        let without_protocol = url.strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);

        if let Some(slash_pos) = without_protocol.find('/') {
            return Some(without_protocol[..slash_pos].to_string());
        }
    }

    None
}

/// Expand ~ in path to home directory
fn expand_home(path: &str) -> PathBuf {
    if path.starts_with("~/") || path == "~" {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        PathBuf::from(home).join(&path[2..])
    } else {
        PathBuf::from(path)
    }
}

/// Check if SSH agent is running
fn is_ssh_agent_running() -> bool {
    // Check for SSH_AUTH_SOCK environment variable (works on all platforms)
    if env::var("SSH_AUTH_SOCK").is_ok() {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, also check if ssh-agent service is running
        // Or if Git's ssh-agent is available
        if env::var("GIT_SSH").is_ok() || env::var("SSH_AGENT_PID").is_ok() {
            return true;
        }

        // Check if Pageant is running (PuTTY's SSH agent)
        Command::new("cmd")
            .args(&["/C", "tasklist", "/FI", "IMAGENAME eq pageant.exe"])
            .output()
            .map(|o| {
                let output = String::from_utf8_lossy(&o.stdout);
                output.contains("pageant.exe")
            })
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Create remote callbacks with SSH authentication support
fn create_remote_callbacks<'a>(
    credentials: &'a HashMap<String, String>,
    remote_url: &'a str,
    debug: bool,
) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();

    debug_log!(debug, "Setting up SSH authentication for: {}", remote_url);

    if debug {
        // Check SSH agent status
        if is_ssh_agent_running() {
            debug_log!(debug, "SSH agent: RUNNING");
        } else {
            debug_log!(debug, "SSH agent: NOT DETECTED");
        }

        // Show environment variables
        if let Ok(sock) = env::var("SSH_AUTH_SOCK") {
            debug_log!(debug, "SSH_AUTH_SOCK: {}", sock);
        } else {
            debug_log!(debug, "SSH_AUTH_SOCK: Not set");
        }

        if let Ok(git_ssh) = env::var("GIT_SSH") {
            debug_log!(debug, "GIT_SSH: {}", git_ssh);
        }

        // Show configured credentials
        if credentials.is_empty() {
            debug_log!(debug, "No credentials configured in .mgitconfig.json");
        } else {
            debug_log!(debug, "Configured credentials for: {:?}", credentials.keys().collect::<Vec<_>>());
        }
    }

    callbacks.credentials(move |url, username_from_url, allowed_types| {
        let username = username_from_url.unwrap_or("git");

        debug_log!(debug, "Credentials requested for URL: {}", url);
        debug_log!(debug, "Username from URL: {:?}", username_from_url);
        debug_log!(debug, "Allowed auth types: {:?}", allowed_types);

        // Try SSH agent first
        debug_log!(debug, "Attempting SSH agent authentication...");
        if let Ok(cred) = Cred::ssh_key_from_agent(username) {
            debug_log!(debug, "✓ SSH agent authentication succeeded");
            return Ok(cred);
        }
        debug_log!(debug, "✗ SSH agent authentication failed");

        // Extract hostname from URL and look up configured credentials
        if let Some(hostname) = extract_hostname(remote_url) {
            debug_log!(debug, "Extracted hostname: {}", hostname);

            if let Some(key_path) = credentials.get(&hostname) {
                debug_log!(debug, "Found configured key for {}: {}", hostname, key_path);

                let private_key = expand_home(key_path);
                let public_key = PathBuf::from(format!("{}.pub", private_key.display()));

                debug_log!(debug, "Private key path: {}", private_key.display());
                debug_log!(debug, "Public key path: {}", public_key.display());

                if private_key.exists() {
                    debug_log!(debug, "✓ Private key exists");
                } else {
                    debug_log!(debug, "✗ Private key NOT FOUND at {}", private_key.display());
                }

                if public_key.exists() {
                    debug_log!(debug, "✓ Public key exists");
                } else {
                    debug_log!(debug, "✗ Public key NOT FOUND at {}", public_key.display());
                }

                if private_key.exists() {
                    debug_log!(debug, "Attempting SSH key authentication...");
                    match Cred::ssh_key(
                        username,
                        Some(&public_key),
                        &private_key,
                        None,
                    ) {
                        Ok(cred) => {
                            debug_log!(debug, "✓ SSH key authentication succeeded");
                            return Ok(cred);
                        }
                        Err(e) => {
                            debug_log!(debug, "✗ SSH key authentication failed: {}", e);
                        }
                    }
                } else {
                    debug_log!(debug, "Skipping SSH key auth (private key not found)");
                }
            } else {
                debug_log!(debug, "No credentials configured for hostname: {}", hostname);
                debug_log!(debug, "Available configured hosts: {:?}", credentials.keys().collect::<Vec<_>>());
            }
        } else {
            debug_log!(debug, "Failed to extract hostname from URL");
        }

        // As fallback, try default credential
        debug_log!(debug, "Attempting default credential fallback...");
        match Cred::default() {
            Ok(cred) => {
                debug_log!(debug, "✓ Default credential succeeded");
                Ok(cred)
            }
            Err(e) => {
                debug_log!(debug, "✗ Default credential failed: {}", e);
                debug_log!(debug, "❌ All authentication methods exhausted");
                Err(e)
            }
        }
    });

    callbacks
}

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

/// Normalize a user name or email to its canonical form
fn normalize_author(author: &str, user_aliases: &HashMap<String, Vec<String>>) -> String {
    let author_lower = author.to_lowercase();

    // Check each canonical user and their aliases
    for (canonical, aliases) in user_aliases {
        // Check if matches canonical name
        if canonical.to_lowercase() == author_lower {
            return canonical.clone();
        }

        // Check if matches any alias
        for alias in aliases {
            if alias.to_lowercase() == author_lower {
                return canonical.clone();
            }
        }
    }

    // No match found, return original
    author.to_string()
}

/// Collect all unique author identities from all branches in a repository
/// Returns a set of author identities (name + email pairs)
pub fn collect_all_author_identities(repo_path: &Path) -> Result<HashSet<AuthorIdentity>> {
    let repo = Repository::open(repo_path)?;
    let mut identities = HashSet::new();

    // Iterate through all branches
    let branches = repo.branches(Some(BranchType::Local))?;

    for branch_result in branches {
        let (branch, _) = branch_result?;
        let branch_ref = branch.get();

        if let Some(branch_oid) = branch_ref.target() {
            let mut revwalk = repo.revwalk()?;
            revwalk.push(branch_oid)?;

            // Walk through all commits in this branch
            for oid_result in revwalk {
                if let Ok(oid) = oid_result {
                    if let Ok(commit) = repo.find_commit(oid) {
                        let author = commit.author();
                        let name = author.name().unwrap_or("Unknown").to_string();
                        let email = author.email().unwrap_or("").to_string();

                        // Only add if we have both name and email
                        if !name.is_empty() && !email.is_empty() {
                            identities.insert(AuthorIdentity { name, email });
                        }
                    }
                }
            }
        }
    }

    Ok(identities)
}

/// Collect commit statistics for a branch
/// Only counts commits that are NOT in the main branch (master/main)
/// Returns (commit_stats, last_commit_sha, last_updated_time)
fn collect_branch_stats(
    repo: &Repository,
    branch_name: &str,
    branch_oid: Oid,
    user_aliases: &HashMap<String, Vec<String>>,
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

        // Get author name and normalize it
        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown");
        let author_email = author.email().unwrap_or("");

        // Try to normalize using name first, then email if name doesn't match
        let normalized_name = normalize_author(author_name, user_aliases);
        let normalized_name = if normalized_name == author_name && !author_email.is_empty() {
            // Name wasn't normalized, try email
            normalize_author(author_email, user_aliases)
        } else {
            normalized_name
        };

        // Increment commit count for this author
        *commit_stats.entry(normalized_name).or_insert(0) += 1;

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
    user_aliases: &HashMap<String, Vec<String>>,
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
            collect_branch_stats(&repo, &name, branch_oid, user_aliases)?;

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

pub fn pull_repo(repo_path: &Path, debug: bool) -> Result<String> {
    let repo = Repository::open(repo_path)?;

    // Get the current branch
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD");

    debug_log!(debug, "Repository: {:?}", repo_path);
    debug_log!(debug, "Current branch: {}", branch_name);

    // Load config for credentials
    use crate::models::Config;
    let config = Config::load(".mgitconfig.json").unwrap_or_else(|_| Config {
        repositories: Vec::new(),
        tasks: Vec::new(),
        shells: Default::default(),
        credentials: HashMap::new(),
        users: HashMap::new(),
    });

    // Get remote URL
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().unwrap_or("");

    debug_log!(debug, "Remote URL: {}", remote_url);

    // Check if we have proper SSH setup
    if debug && remote_url.starts_with("git@") {
        let hostname = extract_hostname(remote_url);
        let has_ssh_agent = is_ssh_agent_running();
        let has_configured_key = hostname.as_ref().and_then(|h| config.credentials.get(h)).is_some();

        if !has_ssh_agent && !has_configured_key {
            debug_log!(debug, "⚠️  WARNING: SSH URL detected but no authentication method available!");
            debug_log!(debug, "   Solutions:");
            debug_log!(debug, "   1. Start SSH agent and add your key: ssh-add ~/.ssh/id_rsa");
            if let Some(h) = hostname {
                debug_log!(debug, "   2. Configure credentials in .mgitconfig.json:");
                debug_log!(debug, "      \"credentials\": {{");
                debug_log!(debug, "        \"{}\": \"~/.ssh/id_rsa\"", h);
                debug_log!(debug, "      }}");
            }
        }
    }

    // Setup SSH callbacks for fetch
    let callbacks = create_remote_callbacks(&config.credentials, remote_url, debug);
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    debug_log!(debug, "Starting fetch operation...");

    // Fetch
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&[branch_name], Some(&mut fetch_options), None)?;

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

pub fn push_repo(repo_path: &Path, debug: bool) -> Result<String> {
    let repo = Repository::open(repo_path)?;

    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD");

    debug_log!(debug, "Repository: {:?}", repo_path);
    debug_log!(debug, "Current branch: {}", branch_name);

    // Load config for credentials
    use crate::models::Config;
    let config = Config::load(".mgitconfig.json").unwrap_or_else(|_| Config {
        repositories: Vec::new(),
        tasks: Vec::new(),
        shells: Default::default(),
        credentials: HashMap::new(),
        users: HashMap::new(),
    });

    // Get remote URL
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().unwrap_or("");

    debug_log!(debug, "Remote URL: {}", remote_url);

    // Check if we have proper SSH setup
    if debug && remote_url.starts_with("git@") {
        let hostname = extract_hostname(remote_url);
        let has_ssh_agent = is_ssh_agent_running();
        let has_configured_key = hostname.as_ref().and_then(|h| config.credentials.get(h)).is_some();

        if !has_ssh_agent && !has_configured_key {
            debug_log!(debug, "⚠️  WARNING: SSH URL detected but no authentication method available!");
            debug_log!(debug, "   Solutions:");
            debug_log!(debug, "   1. Start SSH agent and add your key: ssh-add ~/.ssh/id_rsa");
            if let Some(h) = hostname {
                debug_log!(debug, "   2. Configure credentials in .mgitconfig.json:");
                debug_log!(debug, "      \"credentials\": {{");
                debug_log!(debug, "        \"{}\": \"~/.ssh/id_rsa\"", h);
                debug_log!(debug, "      }}");
            }
        }
    }

    // Setup SSH callbacks for push
    let callbacks = create_remote_callbacks(&config.credentials, remote_url, debug);
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    debug_log!(debug, "Starting push operation...");

    let mut remote = repo.find_remote("origin")?;
    let refspec = format!("refs/heads/{}", branch_name);

    remote.push(&[&refspec], Some(&mut push_options))?;

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

    // Check if there are any changes that would need to be committed before pushing
    // We ignore untracked files (WT_NEW) since they don't affect push status
    for entry in statuses.iter() {
        let status = entry.status();

        // Check for staged changes (anything in the index)
        if status.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        ) {
            return Ok(true);
        }

        // Check for unstaged changes to tracked files (but NOT untracked files)
        if status.intersects(
            Status::WT_MODIFIED
                | Status::WT_DELETED
                | Status::WT_TYPECHANGE
                | Status::WT_RENAMED,
        ) {
            return Ok(true);
        }
    }

    Ok(false)
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
