use anyhow::Result;

use crate::models::ToolCall;
use crate::tools::{
    append_file, build_project, change_dir, commit_types_help, copy_file, create_venv,
    current_dir, debug_code, delete_file, docker_build, docker_compose, docker_control, docker_exec,
    docker_images, docker_inspect, docker_logs, docker_network_inspect, docker_network_ls,
    docker_prune, docker_ps, docker_pull, docker_run, docker_stats, docker_volume_ls,
    docker_volume_rm, docs_fetch, edit_file, env_list, format_code, generate_dockerfile,
    generate_github_actions, generate_pr_template, get_env, git_add, git_blame,
    git_branch_delete, git_branch_list, git_changed_files, git_checkout, git_clone,
    git_commit, git_commit_all, git_config, git_config_global, git_current_branch,
    git_diff, git_fetch, git_init, git_log, git_merge, git_pull, git_push, git_rebase,
    git_remote_add, git_remote_branches, git_remote_list, git_root, git_show, git_staged_files,
    git_stash, git_status, git_tag, git_tag_list, glob_files, grep_files, lint, list_dir,
    make_dir, move_file, pkg_info, pkg_install, pkg_list, pkg_remove, pkg_search, pkg_update,
    pkg_upgrade, pkg_versions_bulk, process_list, project_init, read_file, research, run_code,
    run_tests, set_env, sysinfo, todo_read, todo_write, web_fetch, web_search, write_file,
};

/// Tool execution result
#[derive(Debug)]
pub struct ToolResult {
    #[allow(dead_code)]
    pub tool_name: String,
    pub output: String,
    pub success: bool,
}

impl ToolResult {
    pub fn ok(name: impl Into<String>, output: impl Into<String>) -> Self {
        Self { tool_name: name.into(), output: output.into(), success: true }
    }

    pub fn err(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self { tool_name: name.into(), output: error.into(), success: false }
    }
}

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
     .replace("\\t", "\t")
     .replace("\\r", "\r")
     .replace("\\\\", "\\")
}

// ─── System prompt ─────────────────────────────────────────────────────────────────────────

pub fn tool_descriptions() -> &'static str {
    r#"You are a full-stack AI agent implemented in Rust. You support all stages of software development.
When using libraries/frameworks, always check the latest information first with research or pkg_info,
and analyze official docs and blogs simultaneously to apply the latest best practices.

=== Tool usage format ===
TOOL: <tool_name> <args>

=== File system ===

TOOL: read_file <path>
TOOL: write_file <path> <content>          (use \n for newlines)
TOOL: append_file <path> <content>
TOOL: edit_file <path>
<<<OLD>>>old content<<<NEW>>>new content<<<END>>>
TOOL: delete_file <path>               (delete file/directory)
TOOL: move_file <src> <dst>            (move/rename)
TOOL: copy_file <src> <dst>            (copy)
TOOL: mkdir <path>                     (create directory, including parents)
TOOL: list_dir [path]
TOOL: glob <pattern>                   (e.g. src/**/*.rs)
TOOL: grep <pattern> [path]            (-i flag: case-insensitive)
TOOL: current_dir                      (show current working directory)
TOOL: change_dir <path>               (change working directory)

=== Code execution ===

TOOL: run_code <language>
<code>
Supported: python/python3, javascript/js, typescript/ts, rust, go, bash/sh,
           ruby, php, perl, lua, r, java, c, c++/cpp, kotlin, swift, scala

TOOL: debug_code <language>
<code>

TOOL: shell <command>
  Supports pipes and redirects. Example: TOOL: shell "ls -la | head -20"

=== Git repository management ===

TOOL: git_init [path]                  (init repo + create .gitignore)
TOOL: git_clone <url> [path]
TOOL: git_status [path]
TOOL: git_diff [path] [staged]        (staged=true: diff staged area)
TOOL: git_log [path] [n=10]           (recent n commits)
TOOL: git_show [path] <ref>

TOOL: git_add [path] [files...]       (stage all if no files given)
TOOL: git_commit [path] <message>    (Conventional Commits format recommended)
TOOL: git_commit_all [path] <message> (add -A + commit in one step)
TOOL: git_stash [path] [pop|list|drop]

TOOL: git_branch [path]               (list branches)
TOOL: git_branch [path] <name> create (create new branch)
TOOL: git_checkout [path] <branch>
TOOL: git_checkout [path] <branch> create  (create and switch to new branch)
TOOL: git_merge [path] <branch> [no-ff=true]
TOOL: git_rebase [path] <branch>
TOOL: git_branch_delete [path] <branch> [force=true]

TOOL: git_remote_add [path] <name> <url>
TOOL: git_remote_list [path]
TOOL: git_push [path] [remote=origin] [branch=HEAD] [upstream=true]
TOOL: git_pull [path] [remote=origin] [branch]
TOOL: git_fetch [path] [remote=origin]

TOOL: git_tag [path] <tag> [message]
TOOL: git_tag_list [path]
TOOL: git_config [path] <key> <value>
TOOL: git_config_global <key> <value>
TOOL: commit_types                    (list Conventional Commits types)
TOOL: git_blame [path] <file>         (show last modifier per line)
TOOL: git_root [path]                 (repo root path)
TOOL: git_changed_files [path]        (list changed files)
TOOL: git_staged_files [path]         (list staged files)
TOOL: git_remote_branches [path] [remote=origin]  (list remote branches)

=== Project scaffolding ===

TOOL: project_init <type> <name> [path]
  types: rust, rust-lib, python, node, typescript, react, react-ts,
         vue, next, django, flask, fastapi, go, express, cpp, deno

TOOL: github_actions <project_type> <path>  (generate CI/CD workflow)
TOOL: pr_template <path>                    (generate PR/issue templates)

=== Packages / system ===

TOOL: pkg_install <manager> <package>  (apt, pip, npm, cargo, gem, go, snap...)
TOOL: pkg_remove <manager> <package>
TOOL: pkg_upgrade <manager> <package>  (upgrade a specific package)
TOOL: pkg_update <manager>             (refresh package index)
TOOL: pkg_list <manager>
TOOL: pkg_search <manager> <query>
TOOL: sysinfo
TOOL: process_list [filter]
TOOL: env_list [filter]                (list env vars, optional filter)
TOOL: get_env <key>                    (get env var)
TOOL: set_env <key> <value>           (set env var)

=== Web ===

TOOL: web_fetch <URL>
TOOL: web_search <query>

=== Research (gather up-to-date information) ===

TOOL: research <query> [pages=3]
  → Search + concurrently fetch top N pages and merge the analysis
  e.g. TOOL: research "fastapi best practices 2024" 3
  e.g. TOOL: research "react 19 new features" 2

TOOL: docs_fetch <URL> [max_chars=6000]
  → Fetch official docs URL directly + clean HTML
  e.g. TOOL: docs_fetch https://docs.astro.build/en/getting-started/
  e.g. TOOL: docs_fetch https://react.dev/reference/react/hooks 8000

TOOL: pkg_info <ecosystem> <package>
  → Fetch latest version, dependencies, downloads, etc.
  Supported: npm, pip/pypi, cargo/crates, go, gem/ruby
  e.g. TOOL: pkg_info npm react
  e.g. TOOL: pkg_info pip fastapi
  e.g. TOOL: pkg_info cargo tokio

TOOL: pkg_versions <ecosystem> <pkg1> <pkg2> ...
  → Concurrently fetch latest versions of multiple packages

=== Code quality ===

TOOL: lint <language> <path>
  → Check code quality (rustfmt/clippy, ruff/flake8, eslint, golangci-lint, etc.)

TOOL: format <language> <path>
  → Auto-format code (cargo fmt, black/ruff, prettier, gofmt, etc.)

TOOL: test <language> <path> [filter]
  → Run tests (cargo test, pytest, jest, go test, etc.)

TOOL: build <language> <path>
  → Build the project

TOOL: create_venv <path> [name=.venv]
  → Create a Python virtual environment

TOOL: nvm_use <version>               (switch Node version, e.g. nvm_use 20, nvm_use --lts)

=== Docker / containers ===

TOOL: docker_ps [all]                 (list containers)
TOOL: docker_images                   (list images)
TOOL: docker_pull <image>
TOOL: docker_build <tag> <context> [dockerfile]
TOOL: docker_run <image> [opts] [cmd]
TOOL: docker_control <stop|start|restart|rm> <container>
TOOL: docker_logs <container> [tail=50]
TOOL: docker_exec <container> <cmd>
TOOL: docker_compose <up|down|build|ps|logs> <path> [detach=true]
TOOL: docker_inspect <container|image>
TOOL: docker_stats                    (container resource usage)
TOOL: docker_network_ls               (list networks)
TOOL: docker_network_inspect <network>
TOOL: docker_volume_ls                (list volumes)
TOOL: docker_volume_rm <volume>
TOOL: docker_prune [all]              (clean up unused resources)
TOOL: dockerfile <language> <project_name> <path>
  → Auto-generate Dockerfile + docker-compose.yml

=== Sub-agents ===

TOOL: sub_agent <task>
  → Run a sub-agent in an isolated context (max 15 turns)
  e.g. TOOL: sub_agent "Implement JWT auth in FastAPI project"

TOOL: parallel_agent <t1> | <t2> | <t3>
  → Process multiple tasks in parallel
  e.g. TOOL: parallel_agent "implement backend API" | "implement frontend UI" | "write tests"

=== TODO ===

TOOL: todo_read
TOOL: todo_write
[{"id":"1","content":"task","status":"pending","priority":"high"}]

=== Git convention guide ===

Conventional Commits format:
  <type>(<scope>): <description>

  feat(auth): add OAuth2 login
  fix(api): resolve NPE in user endpoint
  refactor(db): extract repository pattern
  docs(readme): update setup guide

Branch naming convention:
  feature/<name>   new feature
  fix/<name>       bug fix
  hotfix/<name>    urgent fix
  release/<ver>    release prep
  docs/<name>      documentation

=== Rules ===
- File edits: read_file → edit_file (exact string replacement)
- New repo: project_init → git_init → git_commit_all "chore: initial setup"
- Commits must use Conventional Commits format
- Before using an external library: check latest version/best-practices with research + pkg_info
- Complex tasks: plan with todo_write → execute step by step
- After writing code: lint → test → commit (recommended order)
- For Docker deployments: auto-generate Dockerfile with the dockerfile tool
- Typing EXIT alone exits"#
}

// ─── Tool dispatch ────────────────────────────────────────────────────────────────────────────

pub async fn dispatch_tool(call: &ToolCall) -> ToolResult {
    let result: Result<String> = match call.name.as_str() {

        // ── File system ───────────────────────────────────────────────────
        "read_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            read_file(path).map(|c| format!("=== {} ===\n{}", path, c))
        }
        "write_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let content = unescape(&call.args[1..].join(" "));
            write_file(path, &content).map(|_| format!("Saved: {}", path))
        }
        "append_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let content = unescape(&call.args[1..].join(" "));
            append_file(path, &content).map(|_| format!("Appended: {}", path))
        }
        "edit_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let old = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let new = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            edit_file(path, old, new)
        }
        "delete_file" | "remove_file" | "rm_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            delete_file(path).map(|_| format!("Deleted: {}", path))
        }
        "move_file" | "rename_file" | "mv" => {
            let src = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dst = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            move_file(src, dst).map(|_| format!("Moved: {} → {}", src, dst))
        }
        "copy_file" | "cp" => {
            let src = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dst = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            copy_file(src, dst).map(|bytes| format!("Copied: {} → {} ({} bytes)", src, dst, bytes))
        }
        "mkdir" | "make_dir" => {
            let path = call.args.join(" ");
            make_dir(path.trim()).map(|_| format!("Directory created: {}", path.trim()))
        }
        "current_dir" | "pwd" => {
            current_dir().map(|d| format!("Current directory: {}", d))
        }
        "change_dir" | "cd" => {
            let path = call.args.join(" ");
            change_dir(path.trim()).map(|_| format!("Changed directory: {}", path.trim()))
        }
        "list_dir" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            list_dir(path).map(|items| items.join("\n"))
        }
        "glob" => {
            let pattern = call.args.join(" ");
            glob_files(pattern.trim()).map(|files| {
                if files.is_empty() { "No matching files".into() }
                else { format!("{} file(s)\n{}", files.len(), files.join("\n")) }
            })
        }
        "grep" => {
            let (pat, path) = if call.args.first().map(|s| s.as_str()) == Some("-i") {
                (format!("-i {}", call.args.get(1).map(|s| s.as_str()).unwrap_or("")),
                 call.args.get(2).map(|s| s.as_str()).unwrap_or(".").to_string())
            } else {
                (call.args.first().map(|s| s.as_str()).unwrap_or("").to_string(),
                 call.args.get(1).map(|s| s.as_str()).unwrap_or(".").to_string())
            };
            grep_files(&pat, &path).map(|lines| {
                if lines.is_empty() { "No matches found".into() }
                else { format!("{} result(s)\n{}", lines.len(), lines.join("\n")) }
            })
        }

        // ── Code execution ──────────────────────────────────────────────────
        "run_code" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let code = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            run_code(lang, code).map(|r| r.to_string())
        }
        "debug_code" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let code = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            debug_code(lang, code).map(|r| r.to_string())
        }
        "shell" => {
            let cmd = call.args.join(" ");
            crate::tools::system::run_shell(&cmd).map(|r| r.to_string())
        }

        // ── Git ──────────────────────────────────────────────────────────────
        "git_init" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_init(path).map(|r| {
                let _ = crate::tools::git_tool::git_setup_defaults(path);
                r.output
            })
        }
        "git_clone" => {
            let url = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dest = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            git_clone(url, dest).map(|r| r.output)
        }
        "git_status" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_status(path).map(|r| r.output)
        }
        "git_diff" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let staged = call.args.get(1).map(|s| s == "staged" || s == "true").unwrap_or(false);
            git_diff(path, staged).map(|r| r.output)
        }
        "git_log" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let n = call.args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10usize);
            git_log(path, n).map(|r| r.output)
        }
        "git_show" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let git_ref = call.args.get(1).map(|s| s.as_str()).unwrap_or("HEAD");
            git_show(path, git_ref).map(|r| r.output)
        }
        "git_add" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let files: Vec<&str> = call.args[1..].iter().map(|s| s.as_str()).collect();
            git_add(path, &files).map(|r| r.output)
        }
        "git_commit" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let msg = call.args[1..].join(" ");
            git_commit(path, &msg, false).map(|r| r.output)
        }
        "git_commit_all" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let msg = call.args[1..].join(" ");
            git_commit_all(path, &msg).map(|r| r.output)
        }
        "git_stash" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let subcmd = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            git_stash(path, subcmd).map(|r| r.output)
        }
        "git_branch" | "git_branch_list" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            // git_branch <path> <name> [create] → create if name is given
            let maybe_name = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let create_flag = call.args.get(2)
                .map(|s| s == "true" || s == "create" || s == "create=true")
                .unwrap_or(false);

            if !maybe_name.is_empty()
                && maybe_name != "list"
                && (create_flag || call.args.get(2).map(|s| s.contains("create")).unwrap_or(false))
            {
                git_checkout(path, maybe_name, true).map(|r| r.output)
            } else if !maybe_name.is_empty() && maybe_name.starts_with('-') {
                // -a flag or similar: list
                git_branch_list(path).map(|r| r.output)
            } else {
                git_branch_list(path).map(|r| r.output)
            }
        }
        "git_checkout" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            // AI may use -b flag: git_checkout <path> -b <branch>
            let (branch, create) = if call.args.get(1).map(|s| s.as_str()) == Some("-b") {
                (call.args.get(2).map(|s| s.as_str()).unwrap_or(""), true)
            } else {
                let branch = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
                let create = call.args.get(2).map(|s| {
                    s == "true" || s == "create" || s == "-b" || s.contains("create")
                }).unwrap_or(false);
                (branch, create)
            };
            git_checkout(path, branch, create).map(|r| r.output)
        }
        "git_merge" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let branch = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let no_ff = call.args.get(2).map(|s| s == "no-ff" || s == "true").unwrap_or(false);
            git_merge(path, branch, no_ff).map(|r| r.output)
        }
        "git_rebase" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let branch = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            git_rebase(path, branch).map(|r| r.output)
        }
        "git_branch_delete" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let branch = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let force = call.args.get(2).map(|s| s == "force" || s == "true").unwrap_or(false);
            git_branch_delete(path, branch, force).map(|r| r.output)
        }
        "git_remote_add" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let name = call.args.get(1).map(|s| s.as_str()).unwrap_or("origin");
            let url = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            git_remote_add(path, name, url).map(|r| r.output)
        }
        "git_remote" | "git_remote_list" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_remote_list(path).map(|r| r.output)
        }
        "git_push" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let remote = call.args.get(1).map(|s| s.as_str()).unwrap_or("origin");
            let branch = call.args.get(2).map(|s| s.as_str()).unwrap_or("HEAD");
            let upstream = call.args.get(3).map(|s| s == "upstream" || s == "true").unwrap_or(false);
            git_push(path, remote, branch, upstream).map(|r| r.output)
        }
        "git_pull" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let remote = call.args.get(1).map(|s| s.as_str()).unwrap_or("origin");
            let branch = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            git_pull(path, remote, branch).map(|r| r.output)
        }
        "git_fetch" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let remote = call.args.get(1).map(|s| s.as_str()).unwrap_or("origin");
            git_fetch(path, remote).map(|r| r.output)
        }
        "git_tag" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let name = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let msg = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            git_tag(path, name, msg).map(|r| r.output)
        }
        "git_tag_list" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_tag_list(path).map(|r| r.output)
        }
        "git_config" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let key = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let val = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            git_config(path, key, val).map(|r| r.output)
        }
        "git_config_global" => {
            let key = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let val = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            git_config_global(key, val).map(|r| r.output)
        }
        "commit_types" => {
            Ok(commit_types_help().to_string())
        }
        "git_current_branch" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_current_branch(path)
        }
        "git_blame" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let file = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            git_blame(path, file).map(|r| r.output)
        }
        "git_root" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_root(path)
        }
        "git_changed_files" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_changed_files(path).map(|files| {
                if files.is_empty() { "No changed files".to_string() }
                else { files.join("\n") }
            })
        }
        "git_staged_files" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_staged_files(path).map(|files| {
                if files.is_empty() { "No staged files".to_string() }
                else { files.join("\n") }
            })
        }
        "git_remote_branches" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let remote = call.args.get(1).map(|s| s.as_str()).unwrap_or("origin");
            git_remote_branches(path, remote).map(|r| r.output)
        }

        // ── Project scaffolding ──────────────────────────────────────────────
        "project_init" => {
            let project_type = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let name = call.args.get(1).map(|s| s.as_str()).unwrap_or("my-project");
            let path = call.args.get(2).map(|s| s.as_str()).unwrap_or(".");
            project_init(project_type, name, path).map(|r| r.output)
        }
        "github_actions" => {
            let proj_type = call.args.first().map(|s| s.as_str()).unwrap_or("generic");
            let path = call.args.get(1).map(|s| s.as_str()).unwrap_or(".");
            generate_github_actions(proj_type, path)
        }
        "pr_template" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            generate_pr_template(path)
        }

        // ── Packages / system ──────────────────────────────────────────────
        "pkg_install" => {
            let mgr = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let pkg = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            pkg_install(mgr, pkg).map(|r| r.output)
        }
        "pkg_remove" => {
            let mgr = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let pkg = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            pkg_remove(mgr, pkg).map(|r| r.output)
        }
        "pkg_upgrade" => {
            let mgr = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let pkg = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            pkg_upgrade(mgr, pkg).map(|r| r.output)
        }
        "pkg_update" => {
            let mgr = call.args.first().map(|s| s.as_str()).unwrap_or("");
            pkg_update(mgr).map(|r| r.output)
        }
        "pkg_list" => {
            let mgr = call.args.first().map(|s| s.as_str()).unwrap_or("");
            pkg_list(mgr).map(|r| r.output)
        }
        "pkg_search" => {
            let mgr = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let query = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            pkg_search(mgr, query).map(|r| r.output)
        }
        "sysinfo" => sysinfo().map(|r| r.output),
        "process_list" => {
            let filter = call.args.first().map(|s| s.as_str()).unwrap_or("");
            process_list(filter).map(|r| r.output)
        }
        "env_list" => {
            let filter = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let vars = env_list(filter);
            if vars.is_empty() {
                Ok("No environment variables".to_string())
            } else {
                Ok(vars.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("\n"))
            }
        }
        "get_env" => {
            let key = call.args.first().map(|s| s.as_str()).unwrap_or("");
            match get_env(key) {
                Some(v) => Ok(format!("{}={}", key, v)),
                None => Ok(format!("Environment variable '{}' not found", key)),
            }
        }
        "set_env" => {
            let key = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let val = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            set_env(key, val).map(|_| format!("Set: {}={}", key, val))
        }

        // ── Web / research ───────────────────────────────────────────────
        "web_fetch" => {
            let url = call.args.join(" ");
            web_fetch(url.trim()).await
        }
        "web_search" => {
            let query = call.args.join(" ");
            web_search(query.trim()).await
        }
        "research" => {
            let query = if call.args.last().map(|s| s.parse::<usize>().is_ok()).unwrap_or(false) {
                call.args[..call.args.len()-1].join(" ")
            } else {
                call.args.join(" ")
            };
            let n = call.args.last()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(3);
            research(query.trim(), n).await
        }
        "docs_fetch" => {
            let url = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let max_chars = call.args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6000usize);
            docs_fetch(url, max_chars).await
        }
        "pkg_info" => {
            let eco = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let pkg = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            pkg_info(eco, pkg).await
        }
        "pkg_versions" => {
            let eco = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let pkgs: Vec<&str> = call.args[1..].iter().map(|s| s.as_str()).collect();
            pkg_versions_bulk(eco, &pkgs).await
        }

        // ── Code quality ──────────────────────────────────────────────────
        "lint" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let path = call.args.get(1).map(|s| s.as_str()).unwrap_or(".");
            lint(lang, path).map(|r| r.to_string())
        }
        "format" | "format_code" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let path = call.args.get(1).map(|s| s.as_str()).unwrap_or(".");
            format_code(lang, path).map(|r| r.to_string())
        }
        "test" | "run_tests" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let path = call.args.get(1).map(|s| s.as_str()).unwrap_or(".");
            let filter = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            run_tests(lang, path, filter).map(|r| r.to_string())
        }
        "build" | "build_project" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let path = call.args.get(1).map(|s| s.as_str()).unwrap_or(".");
            build_project(lang, path).map(|r| r.to_string())
        }
        "create_venv" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let name = call.args.get(1).map(|s| s.as_str()).unwrap_or(".venv");
            create_venv(path, name).map(|r| r.to_string())
        }
        "nvm_use" | "nvm" => {
            let version = call.args.first().map(|s| s.as_str()).unwrap_or("--lts");
            crate::tools::code_quality::nvm_use(version).map(|r| r.to_string())
        }

        // ── Docker ──────────────────────────────────────────────────────────
        "docker_ps" => {
            let all = call.args.first().map(|s| s == "all" || s == "-a").unwrap_or(false);
            docker_ps(all).map(|r| r.output)
        }
        "docker_images" => docker_images().map(|r| r.output),
        "docker_pull" => {
            let image = call.args.join(" ");
            docker_pull(image.trim()).map(|r| r.output)
        }
        "docker_build" => {
            let tag = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let ctx = call.args.get(1).map(|s| s.as_str()).unwrap_or(".");
            let dockerfile = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            docker_build(tag, ctx, dockerfile).map(|r| r.output)
        }
        "docker_run" => {
            let image = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let opts = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let cmd = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            docker_run(image, opts, cmd).map(|r| r.output)
        }
        "docker_control" | "docker_stop" | "docker_start" | "docker_restart" => {
            let action = if call.name == "docker_control" {
                call.args.first().map(|s| s.as_str()).unwrap_or("stop")
            } else {
                call.name.trim_start_matches("docker_")
            };
            let container = if call.name == "docker_control" {
                call.args.get(1).map(|s| s.as_str()).unwrap_or("")
            } else {
                call.args.first().map(|s| s.as_str()).unwrap_or("")
            };
            docker_control(action, container).map(|r| r.output)
        }
        "docker_logs" => {
            let container = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let tail = call.args.get(1).and_then(|s| s.parse().ok()).unwrap_or(50usize);
            docker_logs(container, tail).map(|r| r.output)
        }
        "docker_exec" => {
            let container = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let cmd = call.args[1..].join(" ");
            docker_exec(container, &cmd).map(|r| r.output)
        }
        "docker_compose" => {
            let action = call.args.first().map(|s| s.as_str()).unwrap_or("ps");
            let path = call.args.get(1).map(|s| s.as_str()).unwrap_or(".");
            let detach = call.args.get(2).map(|s| s == "true" || s == "-d" || s == "detach").unwrap_or(true);
            docker_compose(action, path, detach).map(|r| r.output)
        }
        "docker_inspect" => {
            let target = call.args.join(" ");
            docker_inspect(target.trim()).map(|r| r.output)
        }
        "docker_stats" => docker_stats().map(|r| r.output),
        "docker_network_ls" | "docker_network" => {
            docker_network_ls().map(|r| r.output)
        }
        "docker_network_inspect" => {
            let net = call.args.join(" ");
            docker_network_inspect(net.trim()).map(|r| r.output)
        }
        "docker_volume_ls" | "docker_volume" => {
            docker_volume_ls().map(|r| r.output)
        }
        "docker_volume_rm" => {
            let vol = call.args.join(" ");
            docker_volume_rm(vol.trim()).map(|r| r.output)
        }
        "docker_prune" => {
            let all = call.args.first().map(|s| s == "all" || s == "-a").unwrap_or(false);
            docker_prune(all).map(|r| r.output)
        }
        "dockerfile" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let name = call.args.get(1).map(|s| s.as_str()).unwrap_or("app");
            let path = call.args.get(2).map(|s| s.as_str()).unwrap_or(".");
            generate_dockerfile(lang, name, path)
        }

        // ── Sub-agents ───────────────────────────────────────────────────
        "sub_agent" => {
            let task = call.args.join(" ");
            let url = std::env::var("OLLAMA_API_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string());
            let model = std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "gemma4:e4b".to_string());
            crate::agent::sub_agent::run_sub_agent(&task, &url, &model)
                .await.map_err(|e| anyhow::anyhow!(e))
        }
        "parallel_agent" => {
            let joined = call.args.join(" ");
            let tasks: Vec<String> = joined.split('|')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            if tasks.is_empty() {
                Ok("No tasks".to_string())
            } else {
                let url = std::env::var("OLLAMA_API_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string());
                let model = std::env::var("OLLAMA_MODEL")
                    .unwrap_or_else(|_| "gemma4:e4b".to_string());
                crate::agent::sub_agent::run_multi_agent(tasks, &url, &model)
                    .await
                    .map(|r| r.join("\n\n---\n\n"))
                    .map_err(|e| anyhow::anyhow!(e))
            }
        }

        // ── TODO ─────────────────────────────────────────────────────────────
        "todo_write" => {
            let json = call.args.first().map(|s| s.as_str()).unwrap_or("[]");
            todo_write(json)
        }
        "todo_read" => {
            todo_read().map(|items| {
                if items.is_empty() { "TODO list is empty.".into() }
                else {
                    items.iter().map(|t| {
                        let s = match t.status.as_str() {
                            "completed" => "✅", "in_progress" => "🔄", _ => "⏳",
                        };
                        format!("{} [{}] {}", s, t.id, t.content)
                    }).collect::<Vec<_>>().join("\n")
                }
            })
        }

        unknown => {
            return ToolResult::err(
                unknown,
                format!("Unknown tool: '{}'. See /help or tool_descriptions", unknown)
            );
        }
    };

    match result {
        Ok(output) => ToolResult::ok(&call.name, output),
        Err(e) => ToolResult::err(&call.name, format!("Error: {}", e)),
    }
}
