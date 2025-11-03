use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::db::StateDb;
use crate::models::Config;
use crate::utils::{format_relative_time, get_branch_status, get_repo_state, icons, BranchStatus};

/// Color a branch name based on its sync status
fn color_branch(branch_name: &str, status: BranchStatus) -> ColoredString {
    match status {
        BranchStatus::Synced => branch_name.green(),
        BranchStatus::NeedsPush => branch_name.red(),
        BranchStatus::NeedsPull => branch_name.yellow(), // Using yellow for orange
    }
}

/// Format owner name with " et al" in darker gray
fn format_owner(owner: &str) -> String {
    if owner.ends_with(" et al") {
        let base = &owner[..owner.len() - 6]; // Remove " et al"
        format!("{}{}", base, " et al".bright_black())
    } else {
        owner.to_string()
    }
}

pub fn status_command(detailed: bool, all: bool) -> Result<()> {
    let config = Config::load(".mgitconfig.json")?;
    let db = StateDb::open(".mgitdb")?;

    // -a flag always forces detailed view
    let detailed = detailed || all;

    let mut all_states = Vec::new();

    // Collect all repository states
    for repo_config in &config.repositories {
        let repo_path = Path::new(&repo_config.name);

        if !repo_path.exists() {
            eprintln!("Warning: Repository '{}' not found", repo_config.name);
            continue;
        }

        // Try to load from database first (will have better ownership info if refreshed)
        let state = match db.get_repo_state(&repo_config.name) {
            Ok(Some(db_state)) => {
                // Use database state if available
                db_state
            }
            _ => {
                // Fall back to reading from git if no database entry
                match get_repo_state(repo_path, &repo_config.name) {
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

        all_states.push(state);
    }

    // Sort by last updated (most recent first)
    all_states.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    // Filter branches based on -a flag
    if !all {
        // Without -a: show only current branch (for both simple and detailed views)
        for state in all_states.iter_mut() {
            let current_branch_name = state.current_branch.clone();
            state.branches.retain(|b| b.name == current_branch_name);
        }
    }
    // With -a: show all branches (no filtering)

    if detailed {
        // Get icons for header
        let folder_icon = icons::files::folder();
        let commit_icon = icons::git::commit();
        let owner_icon = icons::git::owner();
        let time_icon = icons::status::info();
        let branch_icon = icons::git::branch();

        // Print header for detailed view with COMMITS and OWNER columns
        println!(
            "{:<28} {:<10} {:<25} {:<20} {}",
            format!("{} REPOSITORY", folder_icon).bold(),
            format!("{} COMMITS", commit_icon).bold(),
            format!("{} OWNER", owner_icon).bold(),
            format!("{} UPDATED", time_icon).bold(),
            format!("{} BRANCH", branch_icon).bold()
        );

        // Detailed view: show all branches
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
                    format_owner(&branch.owner),
                    format_relative_time(branch.last_updated),
                    branch_display
                );
            }
        }
    } else {
        // Get icons for header
        let folder_icon = icons::files::folder();
        let time_icon = icons::status::info();
        let branch_icon = icons::git::branch();

        // Print header for simple view without COMMITS or OWNER columns
        println!(
            "{:<28} {:<20} {}",
            format!("{} REPOSITORY", folder_icon).bold(),
            format!("{} UPDATED", time_icon).bold(),
            format!("{} BRANCH", branch_icon).bold()
        );

        // Simple view: show only current branch
        for state in all_states {
            let repo_path = Path::new(&state.name);

            // Get branch status for coloring
            let branch_status =
                get_branch_status(repo_path, &state.current_branch).unwrap_or(BranchStatus::Synced);

            let branch_display = color_branch(&state.current_branch, branch_status).to_string();

            println!(
                "  {:<28} {:<20} {}",
                state.name,
                format_relative_time(state.last_updated),
                branch_display
            );
        }
    }

    Ok(())
}
