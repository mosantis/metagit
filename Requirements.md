# METAGIT

I want to build a command line tool (metagit) in rust to enhance git functionality when dealing with multiple repositories without having to deal with git submodules. The user would provide a .mgitconfig.json file in the current directory with the following structure:

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
  ]
}

- The command "mgit init" can do an in-depth inspection of the repositories in the current directory and generate a .mgitconfig.json file if it doesn't exist.

##GIT ENHANCEMENTS

- the tool should mimic the behavior of the following git commands but apply them to  repositories in scope. Initially, we should provide support for the following git commands: pull, push, sync (pull & push), status.

where:
- pull would do git pull for each repository in scope.
- push would do git push for each repository in scope.
- sync would do git pull and git push for each repository in scope.
- status would do git status for each repository in scope listing the branchs I am working and when was last updated.

e.g.

REPOSITORY                       BRANCH                                UPDATED
repo1                            main                                  Friday 5pm
repo2                            develop                               10 days ago

if modifier "-d" (detailed) would additionally show other active branches on each repo. E

REPOSITORY                       BRANCH                               UPDATED
repo1                            me:main                               Friday 5pm
repo1                            andy:feature_5678_search_all          10/21/2005
repo1                            lila:feature_5598_refactoring         10/21/2005
repo2                            me:main                               3 weeks ago
repo2                            me:develop                            10 days ago

the default order should be more recently updated first.

## TASK EXECUTION

Metagit should have the ability to execute tasks on repositories in scope.

For instance, apart from the "repositories" tag in the configuration file, Metagit should also support the "tasks" tag. For example, given the following configuration:

```json
{
  "tasks": [
    {
      "name" : "debug_build", [
        { "type": "sh", "repo": "repo1"  "cmd": "build.bat", args: ["-d"]},
        { "type": "sh", "repo": "repo2"  "cmd": "build.sh", args: ["-d"]},
        { "type": "sh", "repo": "repo3"  "cmd": "build.sh", args: ["-d"]},
        { "type": "sh", "repo": "repo4"  "cmd": "build.sh", args: ["-d"]},
        { "type": "sh", "repo": "repo5"  "cmd": "build.sh", args: ["-d"]},
      ]
    },
    {
      "name" : "clean", [
        { "repo": "repo1"  "cmd": "build.bat", args: ["-c"]},
        { "repo": "repo2"  "cmd": "build.sh", args: ["-c"]},
        { "repo": "repo3"  "cmd": "build.sh", args: ["-c"]},
        { "repo": "repo4"  "cmd": "build.sh", args: ["-c"]},
        { "repo": "repo5"  "cmd": "build.ps1", args: ["-c"]},
      ]
    },
  ]
}
```

The command `metagit run debug_build` will execute the sequence of tasks defined in the "debug_build" task.

Metagit should provide progress reports for each task (including the task name, the repository name, and the command being executed). like:
```
Executing "debug_build"...
  repo1  running...                  [build.sh -d]
  repo2  running...                  [build.sh -d]
  repo3  running...                  [build.sh -d]
  repo4  running...                  [build.sh -d]
  repo5  completed.                  [build.ps1 -c]
```

Metagit should be able to determine how to execute scripts (sh, bat, ps1, cmd, exe)
metagit won't invoke git command, it rather use a library to emulate git commands. it will also maintain an internal database to store the state of each repository to avoid resyncing every time. not sure but probably a no-sql database would help with this (how about leveldb?) Let's evaluate the pros and cons of using leveldb or suggest alternatives depending on requirements and rust integration, etc.
