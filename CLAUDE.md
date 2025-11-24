# Project Context

## What is MetaGit (mgit)?

MetaGit is a Rust CLI tool that manages multiple git repositories without using git submodules. It provides unified git operations, task automation, and state management across repos.

**Latest Activity**: Save/restore feature implemented (commit 1f86aa0, Nov 18, 2025)

## Key Features

- **Multi-repo Git Operations**: `mgit pull/push/sync` across all repos
- **Status Dashboard**: `mgit status -d` shows branches, ownership, commit counts, sync status
- **Task Runner**: Execute cross-platform build/test/deploy tasks with `mgit run <task>`
- **Save/Restore**: `mgit save <tag>` and `mgit restore <tag>` for branch state snapshots
- **Variable Substitution**: Environment vars, `-DVAR=VALUE` flags, `$(HOME)`, `$(CWD)`, `$(PROJECT_DIR)`
- **User Normalization**: Auto-discovers and normalizes author identities
- **SSH Authentication**: Configurable SSH keys per git hosting service
- **Nerd Font Support**: Enhanced icons when `NERD_FONT=1` environment variable is set

## Recent Developments

1. **Save/Restore** - Full implementation with reserved tags for master/main
2. **Variable Substitution** - Support for env vars and custom variables in tasks
3. **Improved Time Display** - Better relative time formatting
4. **Branch Ownership** - Enhanced handling for branches with 0 commits
5. **Fixed mgitdb Location** - Corrected database path resolution

## Project Structure

```
src/
├── main.rs               # CLI entry point (clap)
├── commands/             # Command implementations
│   ├── init.rs          # Repository scanning
│   ├── status.rs        # Status display (247 lines)
│   ├── pull.rs, push.rs, sync.rs
│   ├── refresh.rs       # Statistics caching (181 lines)
│   ├── save.rs          # Save branch states (125 lines)
│   ├── restore.rs       # Restore branch states (214 lines)
│   └── run.rs           # Task execution (252 lines)
├── db/                   # sled database wrapper
├── models/               # config.rs (380 lines), repo_state.rs
└── utils/                # git.rs (1,149 lines), icons.rs, time.rs, script.rs, vars.rs
```

**Total**: ~3,456 lines of Rust code

## Key Dependencies

- **git2 0.19**: libgit2 bindings for git operations
- **sled 0.34**: Embedded database for caching
- **clap 4.5**: CLI argument parsing
- **serde_json 1.0**: Configuration serialization
- **chrono 0.4**: Time handling
- **colored 2.1**: Terminal colors

## Configuration Files

- **`.mgitconfig.yaml`**: Project-level config (repositories, tasks, shells)
- **`~/.mgitconfig.yaml`**: Global config (credentials, user aliases, shells)
- **`.mgitdb/`**: sled database cache directory

## Common Workflows

```bash
mgit init              # Scan and create .mgitconfig.yaml
mgit refresh           # Update cache with branch statistics
mgit status            # Quick view of all repos
mgit status -d         # Detailed view with ownership
mgit pull              # Update all repos
mgit run <task>        # Execute defined tasks
mgit save v1.0         # Save current branch state
mgit restore v1.0      # Restore to saved state
```

## Testing

The project includes test repositories in `test-repos/` for validating functionality across platforms (Windows, Linux, macOS).

---

# About libraries and frameworks

When referencing a library, specially in rust, check the release cadence and history of vulnerabilities. Ensure that the library is actively maintained and regularly updated to address security issues. Consider the library's community support and engagement, as well as its overall quality and reliability. Evaluate the library's documentation, test coverage, and user feedback to gauge its maturity and suitability for your project. Additionally, assess the library's compatibility with our project's dependencies and ecosystem. Finally, consider the library's licensing terms and compatibility with your project's licensing requirements.

# About implementation

Prefer robust and well-tested implementations over complex ones. Avoid unnecessary abstractions and favor simplicity and readability. Ensure that the implementation is efficient and scalable, and consider the performance implications of your design choices. Additionally, ensure that the implementation is well-documented and easy to understand, and consider the readability and maintainability of your code.

# About testing

Ensure that the implementation is well-tested and covers all possible scenarios. Use a variety of testing techniques, such as unit tests, integration tests, and end-to-end tests, to ensure that the implementation is robust and reliable. Additionally, ensure that the tests are well-documented and easy to understand, and consider the readability and maintainability of your tests.

# About documentation

Ensure that the implementation is well-documented and covers all possible scenarios. Use a variety of documentation techniques, such as inline comments, docstrings, and README files, to ensure that the implementation is robust and reliable. Additionally, ensure that the documentation is well-documented and easy to understand, and consider the readability and maintainability of your documentation.

# About security

Ensure that the implementation is secure and covers all possible scenarios. Use a variety of security techniques, such as input validation, access control, and encryption, to ensure that the implementation is robust and reliable. Additionally, ensure that the security is well-documented and easy to understand, and consider the readability and maintainability of your security implementation.

# About performance

Ensure that the implementation is efficient and scalable, and consider the performance implications of your design choices. Additionally, ensure that the performance is well-documented and easy to understand, and consider the readability and maintainability of your performance implementation.

# About scalability

Ensure that the implementation is scalable and can handle increasing loads and data volumes. Use a variety of scalability techniques, such as load balancing, caching, and sharding, to ensure that the implementation is robust and reliable. Additionally, ensure that the scalability is well-documented and easy to understand, and consider the readability and maintainability of your scalability implementation.

Consider the following latency numbers every programmer should know:

L1 cache reference ......................... 0.5 ns
Branch mispredict ............................ 5 ns
L2 cache reference ........................... 7 ns
Mutex lock/unlock ........................... 25 ns
Main memory reference ...................... 100 ns
Compress 1K bytes with Zippy ............. 3,000 ns  =   3 µs
Send 2K bytes over 1 Gbps network ....... 20,000 ns  =  20 µs
SSD random read ........................ 150,000 ns  = 150 µs
Read 1 MB sequentially from memory ..... 250,000 ns  = 250 µs
Round trip within same datacenter ...... 500,000 ns  = 0.5 ms
Read 1 MB sequentially from SSD* ..... 1,000,000 ns  =   1 ms
Disk seek ........................... 10,000,000 ns  =  10 ms
Read 1 MB sequentially from disk .... 20,000,000 ns  =  20 ms
Send packet CA->Netherlands->CA .... 150,000,000 ns  = 150 ms

Consider the following scalability techniques:

- Load balancing: Distribute incoming requests across multiple servers to ensure that no single server becomes overloaded.
- Caching: Store frequently accessed data in memory or on disk to reduce the number of database queries.
- Sharding: Divide the data into smaller, more manageable chunks and distribute them across multiple servers.
