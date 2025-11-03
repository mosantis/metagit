# MetaGit (mgit)

A command-line tool written in Rust to enhance git functionality when dealing with multiple repositories, without the complexity of git submodules.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage](#usage)
- [Task Execution](#task-execution)
- [Cross-Platform Support](#cross-platform-support)
- [Icon Support](#icon-support)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Development](#development)
- [Testing](#testing)

## Features

- **Multi-repository management**: Manage multiple git repositories from a single configuration
- **Git operations**: Pull, push, sync, and check status across all repositories
- **Task execution**: Define and execute custom tasks across multiple repositories with real-time progress
- **Cross-platform support**: Platform-specific task steps for Windows, Linux, and macOS
- **Configurable shells**: Choose your preferred shell executables (bash, zsh, pwsh, etc.)
- **Local state caching**: Uses an embedded database (sled) to cache repository state
- **Detailed status views**: See all branches and their last update times
- **Beautiful icons and visual feedback**:
  - Standard Unicode icons work out-of-the-box in all terminals
  - Enhanced Nerd Font icons for a premium terminal experience
  - Color-coded output for better readability
  - Flicker-free progress updates
  - Unicode-aware column alignment
- **Smart script type inference**: Automatically detects script types from extensions (.bat, .ps1, .sh, etc.)
- **Error handling**: Post-mortem logs for failed tasks with captured output
- **Ad-hoc commands**: Execute arbitrary commands in addition to script files

## Installation

### Build from source

```bash
cargo build --release
# Binary will be at target/release/mgit
```

You can then copy it to your PATH:
```bash
# Linux/macOS
cp target/release/mgit ~/.local/bin/
# or
sudo cp target/release/mgit /usr/local/bin/

# Windows
copy target\release\mgit.exe C:\Windows\System32\
```

## Quick Start

```bash
# 1. Navigate to a directory containing multiple git repositories
cd my-projects/

# 2. Initialize configuration
mgit init

# 3. Check repository status
mgit status

# 4. Pull all repositories
mgit pull

# 5. Run a custom task
mgit run build_all
```

## Usage

### Initialize

Scan the current directory for git repositories and create a `.mgit_config.json`:

```bash
mgit init
```

This will detect all git repositories in subdirectories and create a configuration file.

You can also start with the example configuration file provided in `example-config.json` and customize it to your needs.

### Status

Check the status of all repositories:

```bash
mgit status
```

Output:
```
📁 REPOSITORY                   🕒 UPDATED              ⎇ BRANCH
  repo1                        2 hours ago           main
  repo2                        10 days ago           develop
```

For detailed status showing all branches:

```bash
mgit status -d
```

Output:
```
📁 REPOSITORY                   🕒 UPDATED              ⎇ BRANCH
  repo1                        2 hours ago           me:main
  repo1                        10/21/2005            andy:feature_5678_search_all
  repo2                        3 weeks ago           me:main
  repo2                        10 days ago           me:develop
```

### Git Operations

```bash
# Pull all repositories
mgit pull

# Push all repositories
mgit push

# Sync (pull and push) all repositories
mgit sync
```

## Task Execution

Define tasks in `.mgit_config.json`:

```json
{
  "repositories": [
    {
      "name": "frontend",
      "url": "https://github.com/user/frontend.git"
    },
    {
      "name": "backend",
      "url": "https://github.com/user/backend.git"
    }
  ],
  "tasks": [
    {
      "name": "build_all",
      "steps": [
        { "repo": "frontend", "cmd": "build.bat", "args": [] },
        { "repo": "backend", "cmd": "build.sh", "args": [] }
      ]
    }
  ]
}
```

Run a task:

```bash
mgit run build_all
```

The tool displays real-time progress:

```
Executing "build_all"...

  frontend             ⚙ running...                   build.bat
  backend              ⏳ waiting...                   build.sh
```

### Supported Script Types

Scripts are automatically detected by extension, or you can specify the `type` field:

- `sh` - Shell scripts (Linux/macOS)
- `bat` or `cmd` - Windows Batch/CMD scripts
- `ps1` - PowerShell scripts
- `exe` - Executables

**Auto-detection example** (type field optional):
```json
{
  "repo": "app",
  "cmd": "build.bat",
  "args": []
}
```

**Explicit type** (for ad-hoc commands):
```json
{
  "type": "cmd",
  "repo": "app",
  "cmd": "echo",
  "args": ["Building application..."]
}
```

### Error Handling

When tasks fail, MetaGit displays detailed post-mortem logs:

```
════════════════════════════════════════════════════════════════════════════════
Failed Task Logs
════════════════════════════════════════════════════════════════════════════════

▶ frontend build.bat
────────────────────────────────────────────────────────────────────────────────
Output:
  Building frontend...
  ERROR: Dependency conflict detected!
  Build failed!

Errors:
  npm ERR! peer dep missing: react@^18.0.0

════════════════════════════════════════════════════════════════════════════════
```

## Cross-Platform Support

MetaGit supports platform-specific task steps, allowing a single task to work across Windows, Linux, and macOS with different commands for each platform.

### Platform Field

Add a `platform` field to any task step:

- `"windows"` - Windows only
- `"linux"` - Linux only
- `"macos"` - macOS only
- `"linux,macos"` - Multiple platforms (comma-separated)
- `"all"` - All platforms (default)

### Cross-Platform Example

```json
{
  "name": "build",
  "steps": [
    {
      "platform": "windows",
      "repo": "app",
      "cmd": "build.bat",
      "args": []
    },
    {
      "platform": "linux,macos",
      "type": "sh",
      "repo": "app",
      "cmd": "build.sh",
      "args": []
    },
    {
      "platform": "all",
      "type": "cmd",
      "repo": "shared",
      "cmd": "echo",
      "args": ["Build complete!"]
    }
  ]
}
```

When you run this task:
- On **Windows**: Only the Windows and "all" steps execute
- On **Linux**: Only the Linux and "all" steps execute
- On **macOS**: Only the macOS and "all" steps execute

### Common Cross-Platform Patterns

#### Different Build Tools
```json
{
  "platform": "windows",
  "type": "cmd",
  "repo": "backend",
  "cmd": "msbuild",
  "args": ["/p:Configuration=Release"]
},
{
  "platform": "linux,macos",
  "type": "sh",
  "repo": "backend",
  "cmd": "make",
  "args": ["release"]
}
```

#### Package Managers
```json
{
  "platform": "linux",
  "type": "sh",
  "repo": "app",
  "cmd": "apt-get",
  "args": ["update"]
},
{
  "platform": "macos",
  "type": "sh",
  "repo": "app",
  "cmd": "brew",
  "args": ["update"]
},
{
  "platform": "windows",
  "type": "cmd",
  "repo": "app",
  "cmd": "choco",
  "args": ["upgrade", "all"]
}
```

## Icon Support

MetaGit supports beautiful icons in terminal output with automatic Nerd Font detection.

### Enabling Nerd Font Icons

Set the environment variable:

```bash
export NERD_FONT=1
# or
export USE_NERD_FONT=1
```

Per-command usage:
```bash
NERD_FONT=1 mgit status
```

### Icon Sets

**Without Nerd Fonts (Default)**:
- Folder/Repository: 📁
- Time/Updated: 🕒
- Branch: ⎇
- Success: ✓
- Error: ✗
- Warning: ⚠
- Waiting: ⏳
- Running: ⚙

**With Nerd Fonts Enabled**:
- Folder/Repository:  (Folder icon)
- Time/Updated:  (Clock icon)
- Branch:  (Git branch icon)
- Success:  (Check circle)
- Error:  (Times circle)
- Warning:  (Exclamation triangle)
- Waiting:  (Clock icon)
- Running:  (Cog icon)

### Installing Nerd Fonts

1. Visit https://www.nerdfonts.com/
2. Download a font (recommended: JetBrainsMono Nerd Font, FiraCode Nerd Font, or Hack Nerd Font)
3. Install the font on your system
4. Configure your terminal to use the Nerd Font
5. Set `export NERD_FONT=1` in your shell profile (~/.bashrc, ~/.zshrc, etc.)

### Making it Permanent

Add to your shell profile:

```bash
# ~/.bashrc or ~/.zshrc
export NERD_FONT=1
```

Then reload:
```bash
source ~/.bashrc  # or ~/.zshrc
```

## Configuration

### Configuration File Structure

The `.mgit_config.json` file structure:

```json
{
  "shells": {
    "sh": "bash",
    "cmd": "cmd",
    "powershell": "pwsh"
  },
  "repositories": [
    {
      "name": "repository-directory-name",
      "url": "git-repository-url"
    }
  ],
  "tasks": [
    {
      "name": "task-name",
      "steps": [
        {
          "type": "sh",
          "platform": "all",
          "repo": "repository-name",
          "cmd": "script-or-command",
          "args": ["arg1", "arg2"]
        }
      ]
    }
  ]
}
```

### Field Descriptions

**Shell Configuration** (optional):
- `sh`: Shell executable for `.sh` scripts - defaults to `"sh"` (use `"bash"`, `"zsh"`, etc. if needed)
- `cmd`: Command prompt executable for `.bat`/`.cmd` scripts - defaults to `"cmd"`
- `powershell`: PowerShell executable for `.ps1` scripts - defaults to `"powershell"` (use `"pwsh"` for PowerShell Core)

**Repository Fields**:
- `name`: Directory name of the repository
- `url`: Git remote URL

**Task Step Fields**:
- `type`: Script type (`sh`, `bat`, `cmd`, `ps1`, `exe`) - optional, auto-detected from extension
- `platform`: Target platform (`windows`, `linux`, `macos`, `all`, or comma-separated) - optional, defaults to `all`
- `repo`: Repository name (must match a repository's name)
- `cmd`: Script file or command to execute
- `args`: Array of arguments to pass

### Shell Configuration Examples

You can specify shells either by name (if in PATH) or by absolute path for more control.

**Why use full paths?**
- Ensure a specific shell version is used
- Avoid conflicts when multiple versions are installed
- Provide explicit configuration for CI/CD environments
- Work around PATH issues in restricted environments

**Using PowerShell Core (pwsh) instead of Windows PowerShell**:
```json
{
  "shells": {
    "powershell": "pwsh"
  }
}
```

**Using bash on Linux/macOS**:
```json
{
  "shells": {
    "sh": "bash"
  }
}
```

**Using zsh on macOS**:
```json
{
  "shells": {
    "sh": "zsh"
  }
}
```

**Full paths for specific shell versions (Windows)**:
```json
{
  "shells": {
    "sh": "C:\\Program Files\\Git\\bin\\bash.exe",
    "cmd": "C:\\Windows\\System32\\cmd.exe",
    "powershell": "C:\\Program Files\\PowerShell\\7\\pwsh.exe"
  }
}
```

**Full paths on Linux**:
```json
{
  "shells": {
    "sh": "/bin/bash",
    "powershell": "/usr/bin/pwsh"
  }
}
```

**Full paths on macOS (Homebrew installations)**:
```json
{
  "shells": {
    "sh": "/opt/homebrew/bin/bash",
    "powershell": "/opt/homebrew/bin/pwsh"
  }
}
```

**Mixed approach (names and paths)**:
```json
{
  "shells": {
    "sh": "bash",
    "powershell": "C:\\Program Files\\PowerShell\\7\\pwsh.exe"
  }
}
```

## Architecture

### Key Technologies

- **git2**: Rust bindings for libgit2 - used for all git operations
- **sled**: Embedded database for caching repository state
- **clap**: Command-line argument parsing
- **chrono**: Date/time handling with serde support
- **colored**: Terminal output coloring
- **unicode-width**: Proper column alignment with Unicode/emoji icons

### Why sled?

- **Lightweight**: No external dependencies or server required
- **Fast**: In-memory caching with disk persistence
- **Simple API**: Easy key-value storage perfect for caching repo state
- **Rust-native**: Pure Rust implementation with excellent error handling
- **ACID compliance**: Ensures data integrity

### Design Decisions

1. **Library vs Command Execution**: Uses `git2` library instead of shelling out to git commands for better performance and error handling
2. **State Caching**: Maintains local database to avoid re-scanning repositories on every command
3. **Sequential Task Execution**: Tasks execute sequentially for simplicity and clear output
4. **Vendored Dependencies**: Uses vendored OpenSSL and libgit2 for easier cross-platform builds
5. **Change-based Display Updates**: Only redraws lines that have changed, eliminating flicker
6. **Unicode-aware Column Alignment**: Uses `unicode-width` to calculate actual display width of icons and emojis, ensuring perfect column alignment regardless of whether standard Unicode or Nerd Font icons are used

### Project Structure

```
src/
  commands/     - Command implementations (init, status, pull, push, sync, run)
  db/          - Database layer using sled
  models/      - Data structures (Config, RepoState, etc.)
  utils/       - Utility functions (git operations, icons, time formatting, script execution)
  main.rs      - CLI entry point
```

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running Tests

A comprehensive test environment is available:

```bash
cd test-repos

# Initialize configuration
../target/release/mgit init

# Check status
../target/release/mgit status
../target/release/mgit status -d

# Run tasks
../target/release/mgit run build_all
../target/release/mgit run test_all
../target/release/mgit run cross_platform_build
```

## Testing

### Test Environment

The `test-repos/` directory contains a complete test setup:

- **4 Git Repositories**: frontend, backend, shared-lib, tools
- **Multiple Script Types**: .bat, .cmd, .ps1 scripts for testing
- **Various Tasks**: build, test, lint, deploy, CI pipeline
- **Failure Scenarios**: Scripts that fail for testing error handling
- **Cross-Platform Tasks**: Platform-specific steps

### Available Test Tasks

```bash
# Basic operations
mgit run build_all      # Build all projects
mgit run test_all       # Run all tests
mgit run lint           # Run linter

# Deployment
mgit run deploy_staging     # Deploy to staging
mgit run deploy_production  # Deploy to production

# CI/CD
mgit run ci_pipeline    # Complete CI pipeline (build → test → lint)

# Error handling
mgit run build_with_failure   # Test single failure
mgit run multiple_failures    # Test multiple failures
mgit run missing_script       # Test missing file error

# Cross-platform
mgit run cross_platform_build # Platform-specific steps
mgit run adhoc_commands       # Ad-hoc command execution
```

## Troubleshooting

### Tasks hang or don't complete

Ensure scripts have proper exit codes:
- Batch/CMD: `exit /b 0`
- PowerShell: `exit 0`
- Shell: `exit 0`

### PowerShell execution policy errors

MetaGit automatically uses `-ExecutionPolicy Bypass`, but if you have system restrictions, you may need to adjust your PowerShell policy.

### Icons not displaying correctly

1. Install a Nerd Font
2. Configure your terminal to use it
3. Set `NERD_FONT=1` environment variable

### Platform-specific steps not running

Check that the `platform` field matches your OS:
- Windows: `"windows"`
- Linux: `"linux"`
- macOS: `"macos"`

## Future Enhancements

- Parallel task execution
- Interactive TUI mode
- Task dependency graphs
- Watch mode for file changes
- Remote configuration support
- Task templates
- Repository filters

## License

MIT
