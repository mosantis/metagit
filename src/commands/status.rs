use anyhow::Result;
use colored::*;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

use crate::db::StateDb;
use crate::models::Config;
use crate::utils::{format_relative_time, get_repo_state, icons};

pub fn status_command(detailed: bool) -> Result<()> {
    let config = Config::load(".mgitconfig.json")?;
    let db = StateDb::open(".mgitdb")?;

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

    if detailed {
        // Get icons for header
        let folder_icon = icons::files::folder();
        let owner_icon = icons::git::owner();
        let time_icon = icons::status::info();
        let branch_icon = icons::git::branch();

        // Calculate how much extra space icons take (emojis = 1 extra, nerd fonts = 0 extra)
        let folder_extra = folder_icon.width().saturating_sub(1);
        let owner_extra = owner_icon.width().saturating_sub(1);
        let time_extra = time_icon.width().saturating_sub(1);

        // Create padding strings to compensate for icon width
        let folder_pad = " ".repeat(folder_extra);
        let owner_pad = " ".repeat(owner_extra);
        let time_pad = " ".repeat(time_extra);

        // Column widths (same for header and data)
        let repo_col_width: usize = 28;
        let owner_col_width: usize = 15;
        let updated_col_width: usize = 20;

        // Print header for detailed view with OWNER column
        println!(
            "{}{} {:<repo_col_width$} {}{} {:<owner_col_width$} {}{} {:<updated_col_width$} {} {}",
            folder_icon,
            folder_pad,
            "REPOSITORY".bold(),
            owner_icon,
            owner_pad,
            "OWNER".bold(),
            time_icon,
            time_pad,
            "UPDATED".bold(),
            branch_icon,
            "BRANCH".bold(),
            repo_col_width = repo_col_width,
            owner_col_width = owner_col_width,
            updated_col_width = updated_col_width
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
                    "  {:<repo_col_width$} {:<owner_col_width$} {:<updated_col_width$} {}",
                    repo_name,
                    branch.owner,
                    format_relative_time(branch.last_updated),
                    branch_display,
                    repo_col_width = repo_col_width,
                    owner_col_width = owner_col_width,
                    updated_col_width = updated_col_width
                );
            }
        }
    } else {
        // Get icons for header
        let folder_icon = icons::files::folder();
        let time_icon = icons::status::info();
        let branch_icon = icons::git::branch();

        // Calculate how much extra space icons take (emojis = 1 extra, nerd fonts = 0 extra)
        let folder_extra = folder_icon.width().saturating_sub(1);
        let time_extra = time_icon.width().saturating_sub(1);

        // Create padding strings to compensate for icon width
        let folder_pad = " ".repeat(folder_extra);
        let time_pad = " ".repeat(time_extra);

        // Column widths (same for header and data)
        let repo_col_width: usize = 28;
        let updated_col_width: usize = 20;

        // Print header for simple view without OWNER column
        println!(
            "{}{} {:<repo_col_width$} {}{} {:<updated_col_width$} {} {}",
            folder_icon,
            folder_pad,
            "REPOSITORY".bold(),
            time_icon,
            time_pad,
            "UPDATED".bold(),
            branch_icon,
            "BRANCH".bold(),
            repo_col_width = repo_col_width,
            updated_col_width = updated_col_width
        );

        // Simple view: show only current branch
        for state in all_states {
            let branch_display = state.current_branch.green().to_string();
            println!(
                "  {:<repo_col_width$} {:<updated_col_width$} {}",
                state.name,
                format_relative_time(state.last_updated),
                branch_display,
                repo_col_width = repo_col_width,
                updated_col_width = updated_col_width
            );
        }
    }

    Ok(())
}
