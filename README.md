# MetaGit (mgit)

A command-line tool written in Rust to enhance git functionality when dealing with multiple repositories, without the complexity of git submodules.

## Features

- **Multi-repository management**: Manage multiple git repositories from a single configuration
- **Git operations**: Pull, push, sync, and check status across all repositories
- **Task execution**: Define and execute custom tasks across multiple repositories
- **Local state caching**: Uses an embedded database (sled) to cache repository state
- **Detailed status views**: See all branches and their last update times
- **Beautiful icons and visual feedback**:
  - Standard Unicode icons work out-of-the-box in all terminals
  - Enhanced Nerd Font icons for a premium terminal experience
  - Color-coded output for better readability
  - See [ICONS.md](ICONS.md) for full details and setup instructions

## Installation

### Build from source

```bash
cargo build --release
# Binary will be at target/release/mgit
```

You can then copy it to your PATH:
```bash
cp target/release/mgit ~/.local/bin/
# or
sudo cp target/release/mgit /usr/local/bin/
```

## Usage

### Initialize

Scan the current directory for git repositories and create a `.mgit_config.json`:

```bash
mgit init
```

This will detect all git repositories in subdirectories and create a configuration file like:

```json
{
  "repositories": [
    {
      "name": "repo1",
      "url": "https://github.com/user/repo1.git"
    },
    {
      "name": "repo2",
      "url": "https://github.com/user/repo2.git"
    }
  ],
  "tasks": []
}
```

### Status

Check the status of all repositories:

```bash
mgit status
```

Output:
```
REPOSITORY                     BRANCH                                UPDATED
⚡ repo1                        ⎇ main                                2 hours ago
⚡ repo2                        ⎇ develop                             10 days ago
```

With Nerd Fonts enabled (`NERD_FONT=1`):
```
REPOSITORY                     BRANCH                                UPDATED
 repo1                         main                                2 hours ago
 repo2                         develop                             10 days ago
```

For detailed status showing all branches:

```bash
mgit status -d
```

Output:
```
REPOSITORY                     BRANCH                               UPDATED
⚡ repo1                        ⎇ me:main                             2 hours ago
⚡ repo1                        ⎇ andy:feature_5678_search_all        10/21/2005
⚡ repo1                        ⎇ lila:feature_5598_refactoring       10/21/2005
⚡ repo2                        ⎇ me:main                             3 weeks ago
⚡ repo2                        ⎇ me:develop                          10 days ago
```

### Pull

Pull all repositories:

```bash
mgit pull
```

### Push

Push all repositories:

```bash
mgit push
```

### Sync

Sync (pull and push) all repositories:

```bash
mgit sync
```

### Task Execution

Define tasks in `.mgit_config.json`:

```json
{
  "repositories": [
    {
      "name": "repo1",
      "url": "https://github.com/user/repo1.git"
    },
    {
      "name": "repo2",
      "url": "https://github.com/user/repo2.git"
    }
  ],
  "tasks": [
    {
      "name": "debug_build",
      "steps": [
        { "type": "sh", "repo": "repo1", "cmd": "build.sh", "args": ["-d"] },
        { "type": "sh", "repo": "repo2", "cmd": "build.sh", "args": ["-d"] }
      ]
    },
    {
      "name": "clean",
      "steps": [
        { "type": "sh", "repo": "repo1", "cmd": "build.sh", "args": ["-c"] },
        { "type": "sh", "repo": "repo2", "cmd": "build.sh", "args": ["-c"] }
      ]
    }
  ]
}
```

Then run a task:

```bash
mgit run debug_build
```

The tool will display progress for each step:

```
Executing "debug_build"...

  ⚙ repo1                running...           [build.sh -d]
  ⚙ repo2                running...           [build.sh -d]
```

Upon completion, you'll see status icons:
```
Executing "debug_build"...

  ✓ repo1                completed            [build.sh -d]
  ✓ repo2                completed            [build.sh -d]
```

With Nerd Fonts enabled, the icons are even more distinctive:
```
  repo1                running...           [build.sh -d]
   repo2                completed            [build.sh -d]
```

#### Supported Script Types

The `type` field in task steps supports:
- `sh` - Shell scripts (default)
- Automatically detected from file extension:
  - `.sh` - Shell
  - `.bat`, `.cmd` - Windows Batch
  - `.ps1` - PowerShell
  - `.exe` - Executable

## Configuration File

The `.mgit_config.json` file structure:

```json
{
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
          "repo": "repository-name",
          "cmd": "script-or-command",
          "args": ["arg1", "arg2"]
        }
      ]
    }
  ]
}
```

## Architecture

### Key Technologies

- **git2**: Rust bindings for libgit2 - used for all git operations
- **sled**: Embedded database for caching repository state
- **clap**: Command-line argument parsing
- **chrono**: Date/time handling with serde support
- **colored**: Terminal output coloring

### Why sled?

We chose sled as the embedded database for several reasons:
- **Lightweight**: No external dependencies or server required
- **Fast**: In-memory caching with disk persistence
- **Simple API**: Easy key-value storage perfect for caching repo state
- **Rust-native**: Pure Rust implementation with excellent error handling
- **ACID compliance**: Ensures data integrity

### Design Decisions

1. **Library vs Command Execution**: Uses `git2` library instead of shelling out to git commands for better performance and error handling
2. **State Caching**: Maintains local database to avoid re-scanning repositories on every command
3. **Sequential Task Execution**: Tasks execute sequentially for simplicity and clear output (parallel execution could be added later)
4. **Vendored Dependencies**: Uses vendored OpenSSL and libgit2 for easier cross-platform builds

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Project Structure

```
src/
  commands/     - Command implementations (init, status, pull, push, sync, run)
  db/          - Database layer using sled
  models/      - Data structures (Config, RepoState, etc.)
  utils/       - Utility functions (git operations, time formatting, script execution)
  main.rs      - CLI entry point
```

## License

MIT

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.
