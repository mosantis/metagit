use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::db::StateDb;
use crate::models::Config;
use crate::utils::{format_relative_time, get_branch_commit_sha, get_branch_info_with_stats, get_branch_status, get_repo_state, icons, BranchStatus};

/// Color a branch name based on its sync status
fn color_branch(branch_name: &str, status: BranchStatus) -> ColoredString {
    match status {
        BranchStatus::Synced => branch_name.green(),
        BranchStatus::NeedsPush => branch_name.red(),
        BranchStatus::NeedsPull => branch_name.yellow(), // Using yellow for orange
    }
}

#[allow(dead_code)]
/// Format owner name with " et al" in darker gray
fn format_owner(owner: &str) -> String {
    if owner.ends_with("*") {
        let base = &owner[..owner.len() - 1]; // Remove "*"
        format!("{}{}", base, "*".bright_yellow())
    } else {
        owner.to_string()
    }
}

pub fn status_command(all: bool) -> Result<()> {
    let config = Config::load_from_project()?;
    let db_path = config.get_db_path();
    let db = StateDb::open(db_path.to_str().unwrap_or(".mgitdb"))?;

    let mut all_states = Vec::new();

    // Collect all repository states
    for repo_config in &config.repositories {
        let repo_path = config.resolve_repo_path(&repo_config.name);

        if !repo_path.exists() {
            eprintln!("Warning: Repository '{}' not found", repo_config.name);
            continue;
        }

        // Try to load from database first (will have better ownership info if refreshed)
        let mut state = match db.get_repo_state(&repo_config.name) {
            Ok(Some(db_state)) => {
                // Use database state for branch stats
                db_state
            }
            _ => {
                // Fall back to reading from git if no database entry
                match get_repo_state(&repo_path, &repo_config.name) {
                    Ok(state) => {
                        // Save to database
                        let _ = db.save_repo_state(&state);
                        state
                    }
                    Err(e) => {
                        eprintln!("Error reading repository '{}': {}", repo_config.name, e);
                        continue;
                    }
                }
            }
        };

        // SMART CACHING: Always update current_branch from live git state
        // Check if master/main changed - if so, invalidate ALL branches
        match get_repo_state(&repo_path, &repo_config.name) {
            Ok(live_state) => {
                let current_branch = live_state.current_branch;

                // Always update the current_branch to live value
                state.current_branch = current_branch.clone();

                if current_branch == "(detached)" || current_branch == "(no branch)" {
                    // Skip special branch states - no stats to calculate
                } else {
                    // Determine base branch (master or main)
                    let base_branch = if get_branch_commit_sha(&repo_path, "master").is_ok() {
                        "master"
                    } else if get_branch_commit_sha(&repo_path, "main").is_ok() {
                        "main"
                    } else {
                        "" // No base branch found
                    };

                    // Check if base branch (master/main) has changed
                    let base_branch_changed = if !base_branch.is_empty() {
                        let current_base_sha = get_branch_commit_sha(&repo_path, base_branch).ok();
                        let cached_base = state.branches.iter().find(|b| b.name == base_branch);

                        match (cached_base, current_base_sha) {
                            (Some(cached), Some(cur_sha)) => {
                                match &cached.last_commit_sha {
                                    Some(cached_sha) => cached_sha != &cur_sha,
                                    None => true, // No cached SHA - recalculate
                                }
                            }
                            _ => true, // Either not cached or can't get SHA - recalculate
                        }
                    } else {
                        false // No base branch - don't invalidate all
                    };

                    if base_branch_changed {
                        // Base branch changed - recalculate ALL branches
                        let mut new_branches = Vec::new();
                        let mut latest_updated = state.last_updated;

                        // Recalculate all cached branches
                        for cached_branch in &state.branches {
                            match get_branch_info_with_stats(&repo_path, &cached_branch.name, &config.users) {
                                Ok(branch_info) => {
                                    if branch_info.last_updated > latest_updated {
                                        latest_updated = branch_info.last_updated;
                                    }
                                    new_branches.push(branch_info);
                                }
                                Err(e) => {
                                    eprintln!("Warning: Could not recalculate stats for branch '{}' in '{}': {}",
                                             cached_branch.name, repo_config.name, e);
                                }
                            }
                        }

                        // Update state with recalculated branches
                        state.branches = new_branches;
                        state.last_updated = latest_updated;
                        let _ = db.save_repo_state(&state);
                    } else {
                        // Base branch hasn't changed - only check current branch
                        let cached_branch = state.branches.iter().find(|b| b.name == current_branch);
                        let current_sha = get_branch_commit_sha(&repo_path, &current_branch).ok();

                        let needs_recalculation = if let Some(cached) = cached_branch {
                            // Branch exists in cache - check if it has changed
                            match (&cached.last_commit_sha, &current_sha) {
                                (Some(cached_sha), Some(cur_sha)) => cached_sha != cur_sha,
                                _ => true, // Recalculate if we can't compare SHAs
                            }
                        } else {
                            // Branch not in cache - needs calculation
                            true
                        };

                        if needs_recalculation {
                            // Calculate or recalculate stats for this branch
                            match get_branch_info_with_stats(&repo_path, &current_branch, &config.users) {
                                Ok(branch_info) => {
                                    // Remove old cached version if it exists
                                    state.branches.retain(|b| b.name != current_branch);

                                    // Add updated branch info
                                    state.branches.push(branch_info.clone());

                                    // Update state's last_updated to this branch's last_updated
                                    state.last_updated = branch_info.last_updated;

                                    // Save updated state back to database
                                    let _ = db.save_repo_state(&state);
                                }
                                Err(e) => {
                                    eprintln!("Warning: Could not calculate stats for branch '{}' in '{}': {}",
                                             current_branch, repo_config.name, e);
                                }
                            }
                        } else {
                            // Branch is cached and hasn't changed - use cached stats
                            if let Some(branch_info) = state.branches.iter().find(|b| b.name == current_branch) {
                                state.last_updated = branch_info.last_updated;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not read current branch for '{}': {}", repo_config.name, e);
            }
        }

        all_states.push(state);
    }

    // Sort by last updated (most recent first)
    all_states.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    // Filter branches based on -a flag
    if !all {
        // Without -a: show only current branch
        for state in all_states.iter_mut() {
            let current_branch_name = state.current_branch.clone();
            state.branches.retain(|b| b.name == current_branch_name);
        }
    }
    // With -a: show all branches (no filtering)

    // Get icons for header
    let folder_icon = icons::files::folder();
    let commit_icon = icons::git::commit();
    let owner_icon = icons::git::owner();
    let time_icon = icons::status::info();
    let branch_icon = icons::git::branch();

    // Print header with all columns
    println!(
        "{:<28} {:<10} {:<25} {:<20} {}",
        format!("{} REPOSITORY", folder_icon).bold(),
        format!("{} COMMITS", commit_icon).bold(),
        format!("{} OWNER", owner_icon).bold(),
        format!("{} UPDATED", time_icon).bold(),
        format!("{} BRANCH", branch_icon).bold()
    );

    // Display all repositories
    for state in all_states {
        let repo_path = Path::new(&state.name);

        for (idx, branch) in state.branches.iter().enumerate() {
            let repo_name = if idx == 0 {
                state.name.clone()
            } else {
                String::new()
            };

            // Get branch status for coloring
            let branch_status =
                get_branch_status(repo_path, &branch.name).unwrap_or(BranchStatus::Synced);

            let branch_display = color_branch(&branch.name, branch_status).to_string();

            // Get commit count for the owner
            let commit_count = branch.get_owner_commit_count();

            println!(
                "  {:<28} {:<10} {:<25} {:<20} {}",
                repo_name,
                commit_count,
                branch.owner,
                format_relative_time(branch.last_updated),
                branch_display
            );
        }
    }

    Ok(())
}
