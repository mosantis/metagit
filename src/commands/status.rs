use anyhow::Result;
use chrono::{Duration, Utc};
use colored::*;
use std::path::Path;

use crate::db::StateDb;
use crate::models::Config;
use crate::utils::{format_relative_time, get_current_user, get_repo_state, icons};

pub fn status_command(detailed: bool, all: bool) -> Result<()> {
    let config = Config::load(".mgitconfig.json")?;
    let db = StateDb::open(".mgitdb")?;

    // Get current user for filtering
    let current_user = get_current_user().ok();

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

    // Filter branches based on flags
    if !all {
        let thirty_days_ago = Utc::now() - Duration::days(30);

        if detailed {
            // -d flag: show all branches with activity in last 30 days
            for state in all_states.iter_mut() {
                state.branches.retain(|b| b.last_updated > thirty_days_ago);
            }
            // Remove repositories with no branches after filtering
            all_states.retain(|state| !state.branches.is_empty());
        } else if let Some(ref user) = current_user {
            // Default (simple view): show only repos where current user has worked on current branch
            all_states.retain(|state| {
                // Find the current branch
                if let Some(current_branch) = state.branches.iter().find(|b| b.name == state.current_branch) {
                    // Check if user has commits on the current branch
                    current_branch.commit_stats.contains_key(user)
                        && current_branch.commit_stats.get(user).copied().unwrap_or(0) > 0
                } else {
                    false
                }
            });
        }
    }

    if detailed {
        // Get icons for header
        let folder_icon = icons::files::folder();
        let owner_icon = icons::git::owner();
        let time_icon = icons::status::info();
        let branch_icon = icons::git::branch();

        // Print header for detailed view with OWNER column
        println!(
            "{:<28} {:<15} {:<20} {}",
            format!("{} REPOSITORY", folder_icon).bold(),
            format!("{} OWNER", owner_icon).bold(),
            format!("{} UPDATED", time_icon).bold(),
            format!("{} BRANCH", branch_icon).bold()
        );

        // Detailed view: show all branches
        for state in all_states {
            for (idx, branch) in state.branches.iter().enumerate() {
                let repo_name = if idx == 0 {
                    state.name.clone()
                } else {
                    String::new()
                };

                let branch_display = if branch.name == state.current_branch {
                    branch.name.green().to_string()
                } else {
                    branch.name.to_string()
                };

                println!(
                    "  {:<28} {:<15} {:<20} {}",
                    repo_name,
                    branch.owner,
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

        // Print header for simple view without OWNER column
        println!(
            "{:<28} {:<20} {}",
            format!("{} REPOSITORY", folder_icon).bold(),
            format!("{} UPDATED", time_icon).bold(),
            format!("{} BRANCH", branch_icon).bold()
        );

        // Simple view: show only current branch
        for state in all_states {
            let branch_display = state.current_branch.green().to_string();
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
