use crate::models::Config;
use crate::utils::{execute_script, icons, ScriptType};
use anyhow::{anyhow, Result};
use colored::*;
use std::path::Path;
use terminal_size::{terminal_size, Width};

/// Display a task execution header with black text on light grey background
fn display_task_header(task_name: &str, step_num: usize, total_steps: usize, cmd: &str) {
    // Get terminal width, default to 80 if not available
    let term_width = if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    };

    // ANSI escape code for black text on light grey background
    // \x1b[30m = black foreground
    // \x1b[47m = white/light grey background
    // \x1b[0m = reset
    let bg_start = "\x1b[30;47m";
    let bg_end = "\x1b[0m";

    // Line 1: Empty line with background
    println!("{}{}{}", bg_start, " ".repeat(term_width), bg_end);

    // Line 2: Executing "<task_name>"
    let line2 = format!("Executing \"{}\"", task_name);
    let padding = term_width.saturating_sub(line2.len());
    println!("{}{}{}{}", bg_start, line2, " ".repeat(padding), bg_end);

    // Line 3: Step X/Y: <cmd>
    let line3 = format!("Step {}/{}: {}", step_num, total_steps, cmd);
    let padding = term_width.saturating_sub(line3.len());
    println!("{}{}{}{}", bg_start, line3, " ".repeat(padding), bg_end);

    // Line 4: Empty line with background
    println!("{}{}{}", bg_start, " ".repeat(term_width), bg_end);

    println!(); // Add a blank line after the header
}

pub fn run_command(task_name: Option<&str>, detailed: bool) -> Result<()> {
    let config = Config::load_from_project()?;

    // If no task name provided, list all available tasks
    if task_name.is_none() {
        if config.tasks.is_empty() {
            println!("No tasks defined in .mgitconfig.json");
            println!("\nAdd tasks to your configuration file to use this command.");
            return Ok(());
        }

        if detailed {
            // Detailed view with colors and step information
            println!("{}", "Available tasks:".bold());
            println!();

            // Find max repo name length for alignment
            let max_repo_len = 20
                + config
                    .tasks
                    .iter()
                    .map(|t| t.steps.iter().map(|s| s.repo.len()).max().unwrap_or(0))
                    .max()
                    .unwrap_or(0);

            for task in &config.tasks {
                println!(
                    "  {} {}({}):",
                    "•".cyan(),
                    task.name.green().bold(),
                    task.steps.len()
                );

                for step in &task.steps {
                    let platform_info = if step.platform != "all" {
                        format!(" [{}]", step.platform.dimmed())
                    } else {
                        String::new()
                    };
                    println!(
                        "    - {:<width$} {}{}",
                        format!("{}:", step.repo.cyan()),
                        step.cmd,
                        platform_info,
                        width = max_repo_len
                    );
                }
                println!();
            }

            println!(
                "Run a task with: {} {} {}",
                "mgit run".bold(),
                "<task-name>".yellow(),
                "(e.g., mgit run build_all)".dimmed()
            );
        } else {
            // Simple view without colors
            println!("Available tasks:");
            println!();
            for task in &config.tasks {
                println!("  • {}", task.name);
            }
            println!();
            println!("Run a task with: mgit run <task-name>");
            println!("Use -d flag for detailed information: mgit run -d");
        }
        return Ok(());
    }

    let task_name = task_name.unwrap();

    // Find the task
    let task = config
        .tasks
        .iter()
        .find(|t| t.name == task_name)
        .ok_or_else(|| anyhow!("Task '{}' not found", task_name))?;

    // Filter steps to only those that match the current platform
    let steps_to_run: Vec<_> = task
        .steps
        .iter()
        .filter(|step| step.should_run_on_current_platform())
        .cloned()
        .collect();

    let total_steps = steps_to_run.len();

    // Execute tasks sequentially
    for (step_idx, step) in steps_to_run.iter().enumerate() {
        let repo_path = Path::new(&step.repo);

        // Build command display string
        let args_display = step.args.join(" ");
        let cmd_display = if args_display.is_empty() {
            step.cmd.clone()
        } else {
            format!("{} {}", step.cmd, args_display)
        };

        // Display the task header
        display_task_header(task_name, step_idx + 1, total_steps, &cmd_display);

        if !repo_path.exists() {
            let error_msg = format!("{} repository not found: {}", icons::status::error(), step.repo);
            println!("{}\n", error_msg.red());
            return Err(anyhow!("Repository not found: {}", step.repo));
        }

        // Determine script type
        // Priority: explicit type > inferred from extension
        let script_type = if !step.step_type.is_empty() {
            // Explicit type specified
            match step.step_type.as_str() {
                "sh" => ScriptType::Shell,
                "bat" | "cmd" => ScriptType::Batch,
                "ps1" => ScriptType::PowerShell,
                "exe" => ScriptType::Executable,
                _ => ScriptType::from_path(&step.cmd), // Unknown type, try to infer
            }
        } else {
            // No explicit type, infer from file extension
            ScriptType::from_path(&step.cmd)
        };

        // Execute
        match execute_script(
            script_type,
            &step.cmd,
            &step.args,
            repo_path,
            &config.shells,
        ) {
            Ok(mut child) => {
                // Use wait() for real-time output streaming
                match child.wait() {
                    Ok(status) => {
                        if status.success() {
                            println!("{} {}\n", icons::status::success(), "Completed".green());
                        } else {
                            let exit_code = status.code().unwrap_or(-1);
                            let error_msg = format!("{} script execution failed! (errcode: {})", icons::status::error(), exit_code);
                            println!("{}\n", error_msg.red());
                            return Err(anyhow!("Task '{}' failed at step {}/{}: {} (exit code: {})", task_name, step_idx + 1, total_steps, cmd_display, exit_code));
                        }
                    }
                    Err(e) => {
                        let error_msg = if e.to_string().contains("not found")
                            || e.to_string().contains("cannot find")
                        {
                            format!("{} script not found!", icons::status::error())
                        } else {
                            format!("{} {}", icons::status::error(), e)
                        };
                        println!("{}\n", error_msg.red());
                        return Err(anyhow!("Task '{}' failed at step {}/{}: {}", task_name, step_idx + 1, total_steps, e));
                    }
                }
            }
            Err(e) => {
                let error_msg = if e.to_string().contains("not found")
                    || e.to_string().contains("cannot find")
                {
                    format!("{} script not found!", icons::status::error())
                } else {
                    format!("{} {}", icons::status::error(), e)
                };
                println!("{}\n", error_msg.red());
                return Err(anyhow!("Task '{}' failed at step {}/{}: {}", task_name, step_idx + 1, total_steps, e));
            }
        }
    }

    println!("Task '{}' completed successfully!\n", task_name.green().bold());

    Ok(())
}
