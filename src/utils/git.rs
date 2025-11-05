use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use git2::{BranchType, Cred, FetchOptions, Oid, PushOptions, RemoteCallbacks, Repository, Status};
use std::cell::Cell;
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

/// Get the current branch name from a repository
/// Returns the branch name if on a branch, or "(detached)" if in detached HEAD state
fn get_current_branch(repo: &Repository) -> Result<String> {
    // Try to get the HEAD reference
    match repo.head() {
        Ok(head) => {
            // Check if HEAD is a symbolic reference (points to a branch)
            if head.is_branch() {
                // Get the full reference name (e.g., "refs/heads/master")
                if let Some(name) = head.name() {
                    // Strip "refs/heads/" prefix to get just the branch name
                    if let Some(branch_name) = name.strip_prefix("refs/heads/") {
                        return Ok(branch_name.to_string());
                    }
                }
                // Fallback to shorthand if strip_prefix fails
                Ok(head.shorthand().unwrap_or("(unknown)").to_string())
            } else {
                // Detached HEAD state
                Ok("(detached)".to_string())
            }
        }
        Err(_) => {
            // No HEAD (empty repository or corrupt)
            Ok("(no branch)".to_string())
        }
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

/// Check if we have valid SSH authentication available for the given remote URL
/// Returns Ok(()) if authentication is available, or an error with helpful suggestions
fn validate_ssh_auth(
    remote_url: &str,
    credentials: &HashMap<String, String>,
    debug: bool,
) -> Result<()> {
    // Only check SSH URLs
    if !remote_url.starts_with("git@") && !remote_url.starts_with("ssh://") {
        return Ok(()); // HTTPS or other protocols
    }

    let hostname = extract_hostname(remote_url);
    let has_ssh_agent = is_ssh_agent_running();

    debug_log!(debug, "Validating SSH authentication...");
    debug_log!(debug, "  SSH agent running: {}", has_ssh_agent);

    // If SSH agent is running, we're good
    if has_ssh_agent {
        debug_log!(debug, "  ✓ SSH agent available");
        return Ok(());
    }

    // Check if we have a configured key
    if let Some(host) = hostname.as_ref() {
        if let Some(key_path) = credentials.get(host) {
            let private_key = expand_home(key_path);
            let public_key = PathBuf::from(format!("{}.pub", private_key.display()));

            debug_log!(debug, "  Checking configured key: {}", key_path);
            debug_log!(debug, "    Private key: {}", private_key.display());
            debug_log!(debug, "    Public key: {}", public_key.display());

            // Check if both keys exist
            if private_key.exists() && public_key.exists() {
                debug_log!(debug, "  ✓ SSH keys found and valid");
                return Ok(());
            }

            // Keys are configured but don't exist - provide specific error
            let mut error_msg = format!(
                "SSH authentication will fail: Configured keys not found\n\n\
                 The key '{}' is configured in .mgitconfig.json but doesn't exist on disk.\n\n\
                 Please choose one of these solutions:\n\n",
                key_path
            );

            if !private_key.exists() {
                error_msg.push_str(&format!("  • Private key missing: {}\n", private_key.display()));
            }
            if !public_key.exists() {
                error_msg.push_str(&format!("  • Public key missing: {}\n", public_key.display()));
            }

            error_msg.push_str(&format!(
                "\nSolutions:\n\
                 1. Generate the missing SSH key:\n\
                    ssh-keygen -t ed25519 -f {}\n\n\
                 2. Update .mgitconfig.json to point to an existing key:\n\
                    \"credentials\": {{\n\
                      \"{}\": \"~/.ssh/id_rsa\"  (or your actual key path)\n\
                    }}\n\n\
                 3. Start SSH agent and add your key:\n\
                    ssh-add ~/.ssh/id_rsa\n\
                    (Then you won't need credentials in .mgitconfig.json)",
                private_key.display(),
                host
            ));

            return Err(anyhow::anyhow!(error_msg));
        }
    }

    // No SSH agent and no configured keys
    let hostname_str = hostname.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
    let error_msg = format!(
        "SSH authentication not configured\n\n\
         Repository URL: {}\n\
         Host: {}\n\n\
         No SSH authentication method is available. Please choose one solution:\n\n\
         Solution 1 - Use SSH agent (recommended):\n\
           • Start SSH agent: eval $(ssh-agent)\n\
           • Add your key: ssh-add ~/.ssh/id_rsa\n\
           • Verify keys: ssh-add -l\n\n\
         Solution 2 - Configure key in .mgitconfig.json:\n\
           Add this to your .mgitconfig.json file:\n\
           \"credentials\": {{\n\
             \"{}\": \"~/.ssh/id_rsa\"\n\
           }}\n\n\
         Solution 3 - Test SSH connection:\n\
           ssh -T git@{}\n\
           (This will help verify your SSH setup)",
        remote_url, hostname_str, hostname_str, hostname_str
    );

    Err(anyhow::anyhow!(error_msg))
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

    // Track callback attempts to prevent infinite loops
    let attempt_counter = Cell::new(0);

    callbacks.credentials(move |url, username_from_url, allowed_types| {
        // Increment and check attempt counter to prevent infinite loops
        let attempts = attempt_counter.get() + 1;
        attempt_counter.set(attempts);

        debug_log!(debug, "Credentials requested for URL: {} (attempt {})", url, attempts);
        debug_log!(debug, "Username from URL: {:?}", username_from_url);
        debug_log!(debug, "Allowed auth types: {:?}", allowed_types);

        // Prevent infinite loop - bail out after max attempts
        const MAX_ATTEMPTS: usize = 3;
        if attempts > MAX_ATTEMPTS {
            debug_log!(debug, "❌ Maximum authentication attempts ({}) exceeded", MAX_ATTEMPTS);
            return Err(git2::Error::from_str(&format!(
                "Authentication failed after {} attempts. Please check your SSH setup:\n\
                 1. Ensure SSH agent is running and has your key: ssh-add -l\n\
                 2. Add your key to the agent: ssh-add ~/.ssh/id_rsa\n\
                 3. Or configure credentials in .mgitconfig.json",
                MAX_ATTEMPTS
            )));
        }

        let username = username_from_url.unwrap_or("git");

        // Try SSH agent first (only if it's actually running)
        if is_ssh_agent_running() {
            debug_log!(debug, "Attempting SSH agent authentication...");
            if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                debug_log!(debug, "✓ SSH agent authentication succeeded");
                return Ok(cred);
            }
            debug_log!(debug, "✗ SSH agent authentication failed");
        } else {
            debug_log!(debug, "Skipping SSH agent (not running)");
        }

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
    // Load config to get user aliases for owner inference
    use crate::models::Config;
    let config = Config::load_from_project().unwrap_or_else(|_| Config {
        repositories: Vec::new(),
        tasks: Vec::new(),
        shells: Default::default(),
        credentials: HashMap::new(),
        users: HashMap::new(),
        config_dir: None,
    });

    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open repository at {:?}", repo_path))?;

    let current_branch = get_current_branch(&repo)?;

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

        // Infer owner from current git user (same logic as refresh/cache)
        let owner = match get_current_user() {
            Ok(user_name) => normalize_author(&user_name, &config.users),
            Err(_) => "Unknown".to_string(),
        };

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

/// Get the current commit SHA for a branch
pub fn get_branch_commit_sha(repo_path: &Path, branch_name: &str) -> Result<String> {
    let repo = Repository::open(repo_path)?;
    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    let reference = branch.get();
    let oid = reference.target()
        .with_context(|| format!("Branch '{}' has no target", branch_name))?;
    Ok(oid.to_string())
}

/// Get branch info with stats for a specific branch
/// This is used for on-demand caching when status command encounters a new current branch
pub fn get_branch_info_with_stats(
    repo_path: &Path,
    branch_name: &str,
    user_aliases: &HashMap<String, Vec<String>>,
) -> Result<BranchInfo> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open repository at {:?}", repo_path))?;

    // Find the branch
    let branch = repo.find_branch(branch_name, BranchType::Local)
        .with_context(|| format!("Branch '{}' not found", branch_name))?;

    // Get the branch reference
    let reference = branch.get();
    let branch_oid = reference.target()
        .with_context(|| format!("Branch '{}' has no target", branch_name))?;

    // Collect commit stats
    let (commit_stats, last_sha, last_updated) =
        collect_branch_stats(&repo, branch_name, branch_oid, user_aliases)?;

    // Calculate owner based on commit stats, or use current user if no commits
    let owner = if commit_stats.is_empty() {
        // No commits on this branch yet - infer owner as current git user
        match get_current_user() {
            Ok(user_name) => {
                // Normalize the user name through the alias system
                normalize_author(&user_name, user_aliases)
            }
            Err(_) => {
                // Fallback if we can't get git user
                "Unknown".to_string()
            }
        }
    } else {
        // Use commit stats to calculate owner
        let temp_branch = BranchInfo {
            name: branch_name.to_string(),
            owner: String::new(), // Will be calculated
            last_updated,
            commit_stats: commit_stats.clone(),
            last_commit_sha: Some(last_sha.clone()),
        };
        temp_branch.calculate_owner()
    };

    Ok(BranchInfo {
        name: branch_name.to_string(),
        owner,
        last_updated,
        commit_stats,
        last_commit_sha: Some(last_sha),
    })
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

    let current_branch = get_current_branch(&repo)?;

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

        // Calculate owner based on commit stats, or use current user if no commits
        let owner = if commit_stats.is_empty() {
            // No commits on this branch yet - infer owner as current git user
            match get_current_user() {
                Ok(user_name) => {
                    // Normalize the user name through the alias system
                    normalize_author(&user_name, user_aliases)
                }
                Err(_) => {
                    // Fallback if we can't get git user
                    "Unknown".to_string()
                }
            }
        } else {
            // Use commit stats to calculate owner
            let temp_branch = BranchInfo {
                name: name.clone(),
                owner: String::new(), // Will be calculated
                last_updated,
                commit_stats: commit_stats.clone(),
                last_commit_sha: Some(last_sha.clone()),
            };
            temp_branch.calculate_owner()
        };

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
    let branch_name = get_current_branch(&repo)?;

    debug_log!(debug, "Repository: {:?}", repo_path);
    debug_log!(debug, "Current branch: {}", branch_name);

    // Load config for credentials
    use crate::models::Config;
    let config = Config::load_from_project().unwrap_or_else(|_| Config {
        repositories: Vec::new(),
        tasks: Vec::new(),
        shells: Default::default(),
        credentials: HashMap::new(),
        users: HashMap::new(),
        config_dir: None,
    });

    // Get remote URL
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().unwrap_or("");

    debug_log!(debug, "Remote URL: {}", remote_url);

    // Validate SSH authentication early to provide helpful error messages
    validate_ssh_auth(remote_url, &config.credentials, debug)?;

    // Setup SSH callbacks for fetch
    let callbacks = create_remote_callbacks(&config.credentials, remote_url, debug);
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    debug_log!(debug, "Starting fetch operation...");

    // Fetch
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&[branch_name.as_str()], Some(&mut fetch_options), None)?;

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

    let branch_name = get_current_branch(&repo)?;

    debug_log!(debug, "Repository: {:?}", repo_path);
    debug_log!(debug, "Current branch: {}", branch_name);

    // Load config for credentials
    use crate::models::Config;
    let config = Config::load_from_project().unwrap_or_else(|_| Config {
        repositories: Vec::new(),
        tasks: Vec::new(),
        shells: Default::default(),
        credentials: HashMap::new(),
        users: HashMap::new(),
        config_dir: None,
    });

    // Get remote URL
    let remote = repo.find_remote("origin")?;
    let remote_url = remote.url().unwrap_or("");

    debug_log!(debug, "Remote URL: {}", remote_url);

    // Validate SSH authentication early to provide helpful error messages
    validate_ssh_auth(remote_url, &config.credentials, debug)?;

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

/// Result of repository repair operation
#[derive(Debug, Default)]
pub struct RepairResult {
    pub fixed_fetch_head: bool,
    pub removed_corrupted_refs: Vec<String>,
    pub fsck_errors: Vec<String>,
    pub needs_attention: bool,
}

impl RepairResult {
    pub fn has_fixes(&self) -> bool {
        self.fixed_fetch_head || !self.removed_corrupted_refs.is_empty()
    }
}

/// Attempt to repair common git repository corruption issues
pub fn repair_repository(repo_path: &Path) -> Result<RepairResult> {
    let mut result = RepairResult::default();
    let git_dir = repo_path.join(".git");

    if !git_dir.exists() {
        return Err(anyhow::anyhow!("Not a git repository"));
    }

    // 1. Check and fix FETCH_HEAD corruption
    let fetch_head = git_dir.join("FETCH_HEAD");
    if fetch_head.exists() {
        // Try to read FETCH_HEAD - if it fails, it's corrupted
        match std::fs::read_to_string(&fetch_head) {
            Ok(content) => {
                // Check if content looks corrupted (empty, binary data, etc.)
                if content.is_empty() || content.contains('\0') {
                    std::fs::remove_file(&fetch_head)
                        .context("Failed to remove corrupted FETCH_HEAD")?;
                    result.fixed_fetch_head = true;
                }
            }
            Err(_) => {
                // Cannot read file - likely corrupted
                std::fs::remove_file(&fetch_head)
                    .context("Failed to remove corrupted FETCH_HEAD")?;
                result.fixed_fetch_head = true;
            }
        }
    }

    // 2. Check for corrupted loose references in .git/refs
    let refs_dir = git_dir.join("refs");
    if refs_dir.exists() {
        check_and_fix_refs(&refs_dir, &mut result)?;
    }

    // 3. Run git fsck to detect other issues
    let fsck_output = Command::new("git")
        .args(&["-C", repo_path.to_str().unwrap(), "fsck", "--no-progress"])
        .output();

    if let Ok(output) = fsck_output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Collect error/warning messages
        for line in stderr.lines().chain(stdout.lines()) {
            if line.contains("error:") || line.contains("fatal:") {
                result.fsck_errors.push(line.to_string());
                result.needs_attention = true;
            }
        }
    }

    Ok(result)
}

/// Recursively check and fix corrupted references
fn check_and_fix_refs(refs_dir: &Path, result: &mut RepairResult) -> Result<()> {
    if !refs_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(refs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively check subdirectories
            check_and_fix_refs(&path, result)?;
        } else if path.is_file() {
            // Check if reference file is valid
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    // Valid ref should be a 40-char hex SHA or a symbolic ref
                    let trimmed = content.trim();
                    if !is_valid_ref_content(trimmed) {
                        // Corrupted reference - remove it
                        let rel_path = path.strip_prefix(refs_dir.parent().unwrap().parent().unwrap())
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();

                        std::fs::remove_file(&path)
                            .with_context(|| format!("Failed to remove corrupted ref: {}", rel_path))?;

                        result.removed_corrupted_refs.push(rel_path);
                    }
                }
                Err(_) => {
                    // Cannot read file - likely corrupted
                    let rel_path = path.strip_prefix(refs_dir.parent().unwrap().parent().unwrap())
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    std::fs::remove_file(&path)
                        .with_context(|| format!("Failed to remove corrupted ref: {}", rel_path))?;

                    result.removed_corrupted_refs.push(rel_path);
                }
            }
        }
    }

    Ok(())
}

/// Check if reference content is valid
fn is_valid_ref_content(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }

    // Check for symbolic refs (e.g., "ref: refs/heads/main")
    if content.starts_with("ref:") {
        return true;
    }

    // Check for SHA-1 (40 hex chars) or SHA-256 (64 hex chars)
    let len = content.len();
    (len == 40 || len == 64) && content.chars().all(|c| c.is_ascii_hexdigit())
}
