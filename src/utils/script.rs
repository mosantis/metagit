use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

pub enum ScriptType {
    Shell,
    Batch,
    PowerShell,
    Executable,
}

impl ScriptType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "sh" => ScriptType::Shell,
            "bat" | "cmd" => ScriptType::Batch,
            "ps1" => ScriptType::PowerShell,
            "exe" => ScriptType::Executable,
            _ => ScriptType::Shell, // Default to shell
        }
    }

    pub fn from_path(path: &str) -> Self {
        if let Some(ext) = Path::new(path).extension() {
            Self::from_extension(ext.to_str().unwrap_or(""))
        } else {
            ScriptType::Executable
        }
    }
}

pub fn execute_script(
    script_type: ScriptType,
    script_path: &str,
    args: &[String],
    working_dir: &Path,
) -> Result<std::process::Child> {
    let mut cmd = match script_type {
        ScriptType::Shell => {
            // Check if script_path is a file or a command
            let full_path = working_dir.join(script_path);
            if full_path.exists() {
                // It's a file, execute it directly
                let mut c = Command::new("sh");
                c.arg(script_path);
                c
            } else {
                // It's a command, use sh -c to execute
                let mut c = Command::new("sh");
                c.arg("-c");
                // Build the full command with args
                let mut full_cmd = script_path.to_string();
                for arg in args {
                    full_cmd.push(' ');
                    full_cmd.push_str(arg);
                }
                c.arg(full_cmd);
                c
            }
        }
        ScriptType::Batch => {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(script_path);
            c.args(args);
            c
        }
        ScriptType::PowerShell => {
            let mut c = Command::new("powershell");
            c.arg("-File").arg(script_path);
            c.args(args);
            c
        }
        ScriptType::Executable => {
            let mut c = Command::new(script_path);
            c.args(args);
            c
        }
    };

    // Only add args for shell scripts that are files
    if matches!(script_type, ScriptType::Shell) {
        let full_path = working_dir.join(script_path);
        if full_path.exists() {
            cmd.args(args);
        }
    }

    cmd.current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn()?;
    Ok(child)
}
