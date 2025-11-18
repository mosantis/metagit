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
- **Save and restore branch states**: Save current branches to named tags and restore them later (reserved tags `master`/`main` for quick switching)
- **SSH authentication**: Configure SSH keys per Git hosting service for private repository access
- **Debug mode**: Detailed logging for troubleshooting connection and credential issues
- **User normalization**: Automatically discover and normalize author identities across repositories
- **Task execution**: Define and execute custom tasks across multiple repositories with real-time progress
- **Variable substitution**: Use environment variables, predefined variables (HOME, CWD, PROJECT_DIR), and user-defined variables in tasks
- **Cross-platform support**: Platform-specific task steps for Windows, Linux, and macOS
- **Configurable shells**: Choose your preferred shell executables (bash, zsh, pwsh, etc.)
- **Global and project configuration**: Set user-wide defaults in `~/.mgitconfig.json`, override per-project
- **Local state caching**: Uses an embedded database (sled) to cache repository state
- **Branch ownership tracking**: See who owns each branch and commit statistics
- **Detailed status views**: See all branches with ownership, commit counts, and sync status
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

# 2. Initialize configuration (auto-runs refresh)
mgit init

# 3. Check repository status
mgit status

# 4. Show detailed status with commit counts
mgit status -d

# 5. Pull all repositories
mgit pull

# 6. Refresh statistics after pulling
mgit refresh

# 7. Run a custom task
mgit run build_all
```

## Usage

### Initialize

Scan the current directory for git repositories and create a `.mgitconfig.json`:

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
📁 REPOSITORY                 🕒 UPDATED            ⎇ BRANCH
  backend                      2 hours ago           main
  frontend                     10 days ago           develop
```

Branch names are color-coded based on sync status:
- **Green**: Fully synced with remote
- **Red**: Has uncommitted changes or unpushed commits
- **Yellow**: Has remote commits that need to be pulled

For detailed status showing commit counts and ownership:

```bash
mgit status -d
```

Output:
```
📁 REPOSITORY                 ● COMMITS  👤 OWNER                   🕒 UPDATED            ⎇ BRANCH
  backend                      8          John et al                2 hours ago           main
  frontend                     3          Alice                     10 days ago           develop
```

Show all branches (not just current branch):

```bash
mgit status -a
```

Output (always detailed):
```
📁 REPOSITORY                 ● COMMITS  👤 OWNER                   🕒 UPDATED            ⎇ BRANCH
  backend                      8          John et al                2 hours ago           main
  backend                      5          Alice                     3 weeks ago           feature-auth
  frontend                     3          Alice                     10 days ago           develop
  frontend                     0          Bob                       2 months ago          bugfix-123
```

**Notes**:
- Commit counts show only unmerged commits (not yet in main/master)
- Branch ownership is calculated from commit statistics
- "et al" suffix indicates multiple contributors (>5% threshold)
- Use `mgit refresh` to update statistics after pulling changes

### Refresh

Refresh repository states and collect commit statistics:

```bash
mgit refresh
```

This command:
- Analyzes all branches in all repositories
- Collects commit statistics per author
- Calculates branch ownership
- Auto-discovers author identities and adds them to `.mgitconfig.json`
- Caches results in the state database for fast status queries

Output:
```
Refreshing repository states...

  ✓ 📁 backend                        3 branches, 45 commits analyzed
  ✓ 📁 frontend                       2 branches, 28 commits analyzed

Successfully refreshed 2 repositories
Added 3 new author aliases to .mgitconfig.json
```

**When to refresh**:
- After `mgit init` (runs automatically)
- After pulling changes to see updated commit statistics
- When you want to discover new author identities
- When branch ownership changes

### Git Operations

```bash
# Pull all repositories
mgit pull

# Push all repositories
mgit push

# Sync (pull and push) all repositories
mgit sync
```

#### Debug Mode

Troubleshoot connection and credential issues with the `--debug` flag:

```bash
mgit pull --debug
mgit push --debug
mgit sync --debug
```

Debug mode provides detailed information about:
- Repository and branch being accessed
- Remote URL
- SSH agent status (running/not detected)
- Hostname extraction from URL
- Configured credentials lookup
- SSH key paths and file existence
- Authentication attempts and results
- Detailed error messages for each authentication method

Example debug output:
```
🔍 DEBUG MODE ENABLED

Pulling repositories...

backend                          [DEBUG] Repository: "backend"
  [DEBUG] Current branch: main
  [DEBUG] Remote URL: git@github.com:user/backend.git
  [DEBUG] Setting up SSH authentication for: git@github.com:user/backend.git
  [DEBUG] SSH agent: RUNNING
  [DEBUG] Credentials requested for URL: git@github.com:user/backend.git
  [DEBUG] Username from URL: Some("git")
  [DEBUG] Attempting SSH agent authentication...
  [DEBUG] ✓ SSH agent authentication succeeded
Already up-to-date
```

**When to use debug mode**:
- Authentication failures with SSH
- Investigating credential configuration issues
- Verifying SSH key paths are correct
- Checking if SSH agent is running
- Understanding which authentication method is being used

**Note**: MetaGit uses vendored libssh2 and does **not** read `~/.ssh/config`. Use SSH agent or configure credentials in `.mgitconfig.json`. See [SSH Credentials Configuration](#ssh-credentials-configuration) for details.

### Save and Restore Branch States

MetaGit allows you to save and restore the current branch of all repositories using tags.

#### Save Current Branches

Save the current branch state of all repositories to a named tag:

```bash
mgit save <tag-name>
```

Example:
```bash
# Before starting a new feature, save your current state
mgit save before-feature-x

# Output:
# 🕒 Saving current branches to tag 'before-feature-x'...
#
#   ✓ frontend - main
#   ✓ backend - develop
#   ✓ shared-lib - main
#
# ✓ Tag 'before-feature-x' saved successfully! (3 repositories, 0 errors)
```

The tag is saved to `.mgitconfig.json` in the `tags` section:

```json
{
  "tags": {
    "before-feature-x": {
      "frontend": "main",
      "backend": "develop",
      "shared-lib": "main"
    }
  }
}
```

#### Restore Saved Branches

Restore all repositories to a previously saved branch state:

```bash
mgit restore <tag-name>
```

Example:
```bash
# Restore to the saved state
mgit restore before-feature-x

# Output:
# 🕒 Restoring branches from tag 'before-feature-x'...
#
#   ✓ frontend - already on main
#   ✓ backend - switched to develop
#   ✓ shared-lib - already on main
#
# ✓ Tag 'before-feature-x' restored! (3 repositories, 0 errors)
```

#### Reserved Tags: `master` and `main`

Two special tags are reserved and work without needing to be saved:

- `mgit restore master` - Switches all repositories to their `master` branch
- `mgit restore main` - Switches all repositories to their `main` branch

MetaGit automatically detects whether each repository uses `master` or `main` as the default branch.

```bash
# Switch all repos to their default branch (main or master)
mgit restore main

# Output:
# 🕒 Restoring branches from tag 'main'...
#
# 🕒 Using reserved tag 'main' - will switch to default branch (master/main) for each repository
#
#   ✓ frontend - switched to main
#   ✓ backend - switched to master
#   ✓ shared-lib - switched to main
```

**Note**: You cannot save to reserved tag names:
```bash
mgit save master
# Error: Tag 'master' is reserved and cannot be saved. Reserved tags: 'master', 'main'
```

#### Common Use Cases

**1. Before starting a new feature:**
```bash
mgit save stable-state
# Work on feature branches
mgit restore stable-state  # Return to known-good state
```

**2. Quick context switching:**
```bash
# Save current work state
mgit save working-on-feature-a

# Switch to different feature
mgit save working-on-feature-b

# Switch back and forth
mgit restore working-on-feature-a
mgit restore working-on-feature-b
```

**3. Release snapshots:**
```bash
mgit save release-v1.0
mgit save release-v2.0
# Later, restore exact branch state for debugging old releases
mgit restore release-v1.0
```

**4. Return to default branches:**
```bash
# Quick way to get all repos back to main/master
mgit restore main
```

## Task Execution

Define tasks in `.mgitconfig.json`:

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

### Variable Substitution

MetaGit supports variable substitution in task definitions, allowing you to use environment variables, predefined variables, and user-defined variables in your `cmd`, `args`, and `platform` fields.

#### Variable Syntax

Variables can be referenced using either syntax:
- `$(VAR)` - Dollar sign with parentheses
- `${VAR}` - Dollar sign with curly braces
- `~` - Tilde expands to `$(HOME)` (only at the beginning of paths)

#### Predefined Variables

MetaGit provides several predefined variables:

- `$(HOME)` - User's home directory
- `$(CWD)` - Current working directory where `mgit` was invoked
- `$(PROJECT_DIR)` - Directory containing the `.mgitconfig.json` file

All environment variables are also available for use.

#### User-Defined Variables

You can define custom variables via the `-D` flag when running tasks:

```bash
mgit run build -DVERSION=1.2.3 -DENV=production
```

#### Examples

**Using predefined variables:**
```json
{
  "name": "deploy",
  "steps": [
    {
      "repo": "app",
      "type": "sh",
      "cmd": "$(PROJECT_DIR)/scripts/deploy.sh",
      "args": ["--config", "~/config/deploy.json"]
    }
  ]
}
```

**Using environment variables:**
```json
{
  "name": "build",
  "steps": [
    {
      "repo": "backend",
      "cmd": "build.sh",
      "args": ["--output", "$(HOME)/builds"]
    }
  ]
}
```

**Using user-defined variables:**
```json
{
  "name": "release",
  "steps": [
    {
      "repo": "app",
      "cmd": "release.sh",
      "args": ["--version", "$(VERSION)", "--env", "$(ENV)"]
    }
  ]
}
```

Run with:
```bash
mgit run release -DVERSION=2.0.0 -DENV=staging
```

**Platform-specific with variables:**
```json
{
  "name": "cross_build",
  "steps": [
    {
      "platform": "$(BUILD_PLATFORM)",
      "repo": "app",
      "cmd": "build.sh",
      "args": ["$(BUILD_FLAGS)"]
    }
  ]
}
```

Run with:
```bash
mgit run cross_build -DBUILD_PLATFORM=linux -DBUILD_FLAGS=--release
```

**Mixed syntax example:**
```json
{
  "name": "test",
  "steps": [
    {
      "repo": "backend",
      "cmd": "${PROJECT_DIR}/scripts/test.sh",
      "args": ["--report-dir", "$(CWD)/reports", "--config", "~/test-config.json"]
    }
  ]
}
```

#### Error Handling

If you reference an undefined variable, MetaGit will display a clear error:
```
Error: Undefined variable: $(MISSING_VAR)
```

Invalid `-D` flag format will also produce an error:
```bash
mgit run build -DINVALID
# Error: Invalid variable definition 'INVALID'. Expected format: VAR=VALUE
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
- Error: ❌
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

### Configuration Hierarchy

MetaGit supports two levels of configuration:

1. **Global Configuration** (`~/.mgitconfig.json`): User-wide defaults, especially for shell preferences
2. **Project Configuration** (`.mgitconfig.json`): Project-specific settings

The configuration hierarchy works as follows:
- Project settings take precedence over global settings
- Global shell configurations are used if not specified in the project
- Default values are used if neither is specified

### Configuration File Structure

The `.mgitconfig.json` file structure (same for both global and project configs):

```json
{
  "shells": {
    "sh": "bash",
    "cmd": "cmd",
    "powershell": "pwsh"
  },
  "credentials": {
    "github.com": "~/.ssh/id_github",
    "gitlab.com": "~/.ssh/id_gitlab"
  },
  "users": {
    "John": ["John Doe", "JD", "john@example.com"],
    "Alice": ["Alice Smith", "alice@company.com"]
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

**Credentials Configuration** (optional):
- Maps Git hosting service hostnames to SSH private key paths
- Supports `~` for home directory expansion
- Falls back to SSH agent if not specified
- See [SSH Credentials Configuration](#ssh-credentials-configuration) for details

**Users Configuration** (optional):
- Maps canonical usernames to arrays of aliases (names and emails)
- Auto-populated by `mgit refresh` with discovered authors
- Enables commit attribution normalization across different identities
- Case-insensitive matching
- See [User Normalization](#user-normalization) for details

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

### SSH Credentials Configuration

MetaGit supports SSH authentication for private repositories using the `credentials` field. This allows you to specify which SSH key to use for each Git hosting service.

**Why configure SSH credentials?**
- Access private repositories without password prompts
- Use different SSH keys for different services (GitHub, GitLab, Bitbucket, etc.)
- Use organization-specific keys for work vs personal projects
- Avoid relying on SSH agent configuration

#### Credentials Structure

```json
{
  "credentials": {
    "github.com": "~/.ssh/id_github",
    "gitlab.com": "~/.ssh/id_gitlab",
    "bitbucket.org": "~/.ssh/id_bitbucket"
  }
}
```

#### How it works

1. MetaGit extracts the hostname from repository URLs (e.g., `git@github.com:user/repo.git` → `github.com`)
2. Looks up the hostname in the `credentials` map
3. Uses the specified SSH private key for authentication
4. Falls back to SSH agent if no specific key is configured

#### Important: ~/.ssh/config Support

**MetaGit uses vendored libssh2 which does NOT read `~/.ssh/config`**. This means SSH config features like Host aliases, IdentityFile per host, ProxyJump, etc. are not automatically applied.

**Workaround options**:

**Option 1: Use SSH Agent** (Recommended)
```bash
# Start SSH agent and add your key
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_rsa

# Now mgit will use the agent
mgit pull
```

The SSH agent approach works with your `~/.ssh/config` because you add keys manually, and those keys can be configured in your SSH config.

**Option 2: Explicitly Configure Keys in .mgitconfig.json**

Instead of relying on SSH config, map each hostname to its key:
```json
{
  "credentials": {
    "github.com": "~/.ssh/id_github",
    "work-gitlab.example.com": "~/.ssh/id_work"
  }
}
```

**Option 3: Build with System Libraries** (Advanced)

For full `~/.ssh/config` support, you can build from source using system libraries instead of vendored ones. This requires libgit2 and openssl development libraries installed on your system:

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install libgit2-dev libssl-dev

# Edit Cargo.toml and change:
# git2 = { version = "0.19", default-features = false, features = ["vendored-openssl", "vendored-libgit2", "ssh"] }
# to:
# git2 = "0.19"

# Build
cargo build --release
```

**Trade-offs**:
- Vendored (default): ✅ Portable, ❌ No SSH config support
- System libs: ✅ Full SSH config support, ❌ Requires system dependencies

#### Examples

**Single key for all services**:
```json
{
  "credentials": {
    "github.com": "~/.ssh/id_rsa"
  }
}
```

**Different keys per service**:
```json
{
  "credentials": {
    "github.com": "~/.ssh/id_ed25519_github",
    "gitlab.company.com": "~/.ssh/id_rsa_work",
    "bitbucket.org": "~/.ssh/id_ed25519_personal"
  }
}
```

**Windows paths** (use forward slashes or escaped backslashes):
```json
{
  "credentials": {
    "github.com": "C:/Users/YourName/.ssh/id_ed25519"
  }
}
```

**Tilde expansion**: The `~` character is automatically expanded to your home directory on all platforms.

**SSH key requirements**:
- Both private key (`id_rsa`) and public key (`id_rsa.pub`) must exist
- Keys must have proper permissions (600 for private key on Linux/macOS)
- Passphrase-protected keys work if your SSH agent has them loaded

### User Normalization

The `users` field allows you to normalize multiple author identities to canonical usernames. This is useful when the same person commits using different names or email addresses.

**Why normalize users?**
- Consolidate commit statistics for the same person across different identities
- Handle name changes (e.g., marriage, legal name changes)
- Unify work and personal email addresses
- Normalize typos in author names
- Simplify branch ownership display

#### Auto-Discovery

When you run `mgit refresh`, MetaGit automatically discovers all author identities in your repositories and adds them to the `users` section:

```bash
$ mgit refresh
Refreshing repository states...
  ✓ 📁 backend    1 branches, 3 commits analyzed
  ✓ 📁 frontend   1 branches, 2 commits analyzed

Successfully refreshed 4 repositories
Added 4 new author aliases to .mgitconfig.json
```

This creates entries like:
```json
{
  "users": {
    "Alice Johnson": ["alice@example.com"],
    "Bob Smith": ["bob.smith@company.com"],
    "Charlie Brown": ["charlie@dev.com"]
  }
}
```

#### Manual Normalization

After auto-discovery, you can manually edit the configuration to group multiple identities:

```json
{
  "users": {
    "John": [
      "John Crammer",
      "JC",
      "john.crammer@company.com",
      "jc@personal.com"
    ],
    "Alice": [
      "Alice Smith",
      "Alice Johnson",
      "alice@example.com",
      "alice.johnson@company.com"
    ]
  }
}
```

**How it works**:
1. The key (`"John"`) is the canonical username that will be displayed
2. The array contains all aliases (names and emails) for that person
3. Matching is case-insensitive
4. Both author names and email addresses are checked

#### Example Use Case

**Before normalization** - `mgit status -a` shows:
```
📁 REPOSITORY     ● COMMITS  👤 OWNER
  backend         5          John Crammer et al
  frontend        3          JC et al
```

**Configuration**:
```json
{
  "users": {
    "John": ["John Crammer", "JC", "john.crammer@company.com", "jc@personal.com"]
  }
}
```

**After normalization** - `mgit refresh && mgit status -a` shows:
```
📁 REPOSITORY     ● COMMITS  👤 OWNER
  backend         8          John et al
  frontend        8          John et al
```

Now all commits by "John Crammer" and "JC" are correctly attributed to "John" and counted together.

### Global Configuration

You can set user-wide defaults in `~/.mgitconfig.json` (in your home directory). This is especially useful for shell preferences, credentials, and user normalizations that you want to use across all projects.

**Create global configuration**:

```bash
# Linux/macOS
cat > ~/.mgitconfig.json << 'EOF'
{
  "shells": {
    "sh": "bash",
    "powershell": "pwsh"
  },
  "credentials": {
    "github.com": "~/.ssh/id_ed25519",
    "gitlab.com": "~/.ssh/id_rsa"
  },
  "users": {
    "John": ["John Doe", "JD", "john@example.com", "john.doe@company.com"]
  }
}
EOF

# Windows (PowerShell)
@'
{
  "shells": {
    "sh": "C:\\Program Files\\Git\\bin\\bash.exe",
    "powershell": "C:\\Program Files\\PowerShell\\7\\pwsh.exe"
  },
  "credentials": {
    "github.com": "~/.ssh/id_ed25519"
  }
}
'@ | Out-File -FilePath "$env:USERPROFILE\.mgitconfig.json" -Encoding utf8
```

**How it works**:

1. MetaGit first loads the project's `.mgitconfig.json`
2. If shell settings have default values, it looks for `~/.mgitconfig.json`
3. Global shell settings are applied if not overridden locally
4. You can override global settings in any project by specifying shells locally

**Example**:

Global config (`~/.mgitconfig.json`):
```json
{
  "shells": {
    "powershell": "pwsh"
  }
}
```

Project config (`.mgitconfig.json`):
```json
{
  "repositories": [...],
  "tasks": [...]
}
```

In this case, PowerShell scripts will use `pwsh` (from global config) even though the project config doesn't specify it.

**Override example**:

Global config:
```json
{
  "shells": {
    "powershell": "pwsh"
  }
}
```

Project config:
```json
{
  "shells": {
    "powershell": "powershell"
  },
  "repositories": [...],
  "tasks": [...]
}
```

In this case, the project explicitly uses Windows PowerShell (`powershell`), overriding the global preference for PowerShell Core (`pwsh`).

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
