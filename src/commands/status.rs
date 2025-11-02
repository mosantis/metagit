use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::db::StateDb;
use crate::models::Config;
use crate::utils::{format_relative_time, get_repo_state};

pub fn status_command(detailed: bool) -> Result<()> {
    let config = Config::load(".mgit_config.json")?;
    let db = StateDb::open(".mgit_db")?;

    let mut all_states = Vec::new();

    // Collect all repository states
    for repo_config in &config.repositories {
        let repo_path = Path::new(&repo_config.name);

        if !repo_path.exists() {
            eprintln!("Warning: Repository '{}' not found", repo_config.name);
            continue;
        }

        match get_repo_state(repo_path, &repo_config.name) {
            Ok(state) => {
                // Save to database
                let _ = db.save_repo_state(&state);
                all_states.push(state);
            }
            Err(e) => {
                eprintln!("Error reading repository '{}': {}", repo_config.name, e);
            }
        }
    }

    // Sort by last updated (most recent first)
    all_states.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    // Print header
    println!(
        "{:<30} {:<40} {}",
        "REPOSITORY".bold(),
        "BRANCH".bold(),
        "UPDATED".bold()
    );

    if detailed {
        // Detailed view: show all branches
        for state in all_states {
            for (idx, branch) in state.branches.iter().enumerate() {
                let repo_name = if idx == 0 {
                    state.name.clone()
                } else {
                    String::new()
                };

                let branch_display = if branch.name == state.current_branch {
                    format!("{}:{}", branch.owner, branch.name)
                        .green()
                        .to_string()
                } else {
                    format!("{}:{}", branch.owner, branch.name)
                };

                println!(
                    "{:<30} {:<40} {}",
                    repo_name,
                    branch_display,
                    format_relative_time(branch.last_updated)
                );
            }
        }
    } else {
        // Simple view: show only current branch
        for state in all_states {
            let branch_display = state.current_branch.green().to_string();
            println!(
                "{:<30} {:<40} {}",
                state.name,
                branch_display,
                format_relative_time(state.last_updated)
            );
        }
    }

    Ok(())
}
