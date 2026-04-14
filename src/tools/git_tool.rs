/// Git repository management tool
///
/// init, clone, commit, branch, push/pull, status, etc.
/// Includes Conventional Commits message validation

use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;

const GIT_TIMEOUT_SECS: u64 = 120;
const MAX_OUTPUT: usize = 16_000;

// ─── Git result ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct GitResult {
    pub output: String,
    pub success: bool,
}

impl std::fmt::Display for GitResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output)
    }
}

// ─── Conventional Commits validation ────────────────────────────────────────────────

/// Validate and auto-format a Conventional Commits message
/// Format: <type>(<scope>): <description>
/// Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
pub fn validate_commit_message(msg: &str) -> Result<String, String> {
    let valid_types = [
        "feat", "fix", "docs", "style", "refactor", "perf",
        "test", "build", "ci", "chore", "revert", "hotfix", "release",
    ];

    let msg = msg.trim();
    if msg.is_empty() {
        return Err("Commit message is empty.".to_string());
    }

    // already in Conventional Commits format, return as-is
    let first_line = msg.lines().next().unwrap_or("").trim();
    for vt in &valid_types {
        if first_line.starts_with(&format!("{}(", vt))
            || first_line.starts_with(&format!("{}:", vt))
        {
            return Ok(msg.to_string());
        }
    }

    // not in format — warn only, do not enforce
    Ok(msg.to_string())
}

/// Return list of Conventional Commits types
pub fn commit_types_help() -> &'static str {
    r#"Conventional Commits types:
  feat:     add a new feature
  fix:      fix a bug
  docs:     documentation changes
  style:    code formatting, semicolons, etc. (no logic change)
  refactor: refactoring (no feature/bug change)
  perf:     performance improvements
  test:     add or update tests
  build:    build system or external dependency changes
  ci:       CI configuration changes
  chore:    other maintenance
  revert:   revert a previous commit
  release:  release version tag

Format: <type>(<scope>): <description>
Examples:
  feat(auth): add OAuth2 login support
  fix(api): resolve null pointer in user endpoint
  docs(readme): update installation guide
  refactor(db): extract repository pattern"#
}

// ─── Git command execution ─────────────────────────────────────────────────────────

fn run_git(args: &[&str], cwd: Option<&str>) -> Result<GitResult> {
    run_git_with_timeout(args, cwd, GIT_TIMEOUT_SECS)
}

fn run_git_with_timeout(args: &[&str], cwd: Option<&str>, timeout_secs: u64) -> Result<GitResult> {
    use std::time::Duration;

    let mut cmd = Command::new("git");
    cmd.args(args);

    if let Some(dir) = cwd {
        if !dir.is_empty() && dir != "." {
            cmd.current_dir(dir);
        }
    }

    // env: non-interactive, no color
    cmd.env("GIT_TERMINAL_PROMPT", "0")
       .env("GIT_ASKPASS", "echo")
       .env("TERM", "dumb");

    let timeout = Duration::from_secs(timeout_secs);
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd_owned = cwd.map(|s| s.to_string());

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut c = Command::new("git");
        c.args(&args_owned);
        if let Some(ref dir) = cwd_owned {
            if !dir.is_empty() && dir != "." {
                c.current_dir(dir);
            }
        }
        c.env("GIT_TERMINAL_PROMPT", "0")
         .env("GIT_ASKPASS", "echo")
         .env("TERM", "dumb");
        let _ = tx.send(c.output());
    });

    let output = rx.recv_timeout(timeout)
        .with_context(|| format!("git {:?} timeout", args))?
        .with_context(|| "git execution failed")?;

    let stdout = truncate(String::from_utf8_lossy(&output.stdout).to_string());
    let stderr = truncate(String::from_utf8_lossy(&output.stderr).to_string());
    let success = output.status.success();

    let combined = if success {
        if stdout.is_empty() { stderr.clone() } else { stdout }
    } else if stdout.is_empty() {
        stderr.clone()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    Ok(GitResult {
        output: combined.trim().to_string(),
        success,
    })
}

fn truncate(s: String) -> String {
    if s.len() > MAX_OUTPUT {
        format!("{}...[truncated {} bytes]", &s[..MAX_OUTPUT], s.len())
    } else {
        s
    }
}

// ─── Git init / clone ────────────────────────────────────────────────────────

/// git init [path]
pub fn git_init(path: &str) -> Result<GitResult> {
    let target = if path.is_empty() { "." } else { path };
    if target != "." {
        std::fs::create_dir_all(target)
            .with_context(|| format!("Failed to create directory: {}", target))?;
    }
    let result = run_git(&["init"], Some(target))?;

    // create default .gitignore if not present
    let gi_path = format!("{}/.gitignore", target.trim_end_matches('/'));
    if !Path::new(&gi_path).exists() {
        let _ = std::fs::write(&gi_path, DEFAULT_GITIGNORE);
    }

    // set default branch to main
    let _ = run_git(&["checkout", "-b", "main"], Some(target));

    Ok(GitResult {
        output: format!("{}\n.gitignore created", result.output),
        success: result.success,
    })
}

/// git clone <url> [dest_dir]
pub fn git_clone(url: &str, dest: &str) -> Result<GitResult> {
    if dest.is_empty() {
        run_git(&["clone", url], None)
    } else {
        run_git(&["clone", url, dest], None)
    }
}

// ─── Status / log ─────────────────────────────────────────────────────────────

/// git status
pub fn git_status(path: &str) -> Result<GitResult> {
    run_git(&["status", "--short", "--branch"], Some(path))
}

/// git diff [--staged]
pub fn git_diff(path: &str, staged: bool) -> Result<GitResult> {
    if staged {
        run_git(&["diff", "--staged", "--stat"], Some(path))
    } else {
        run_git(&["diff", "--stat"], Some(path))
    }
}

/// git log --oneline -N
pub fn git_log(path: &str, n: usize) -> Result<GitResult> {
    let n_str = n.to_string();
    run_git(
        &["log", "--oneline", "--graph", "--decorate", &format!("-{}", n_str)],
        Some(path),
    )
}

/// git show <ref>
pub fn git_show(path: &str, git_ref: &str) -> Result<GitResult> {
    run_git(&["show", "--stat", git_ref], Some(path))
}

// ─── Staging / commit ─────────────────────────────────────────────────────────

/// git add <files...>
pub fn git_add(path: &str, files: &[&str]) -> Result<GitResult> {
    if files.is_empty() || files == ["."] {
        run_git(&["add", "."], Some(path))
    } else {
        let mut args = vec!["add"];
        args.extend_from_slice(files);
        run_git(&args, Some(path))
    }
}

/// git commit -m <message> (with Conventional Commits validation)
pub fn git_commit(path: &str, message: &str, allow_empty: bool) -> Result<GitResult> {
    let msg = validate_commit_message(message)
        .unwrap_or_else(|_| message.to_string());

    if allow_empty {
        run_git(&["commit", "--allow-empty", "-m", &msg], Some(path))
    } else {
        run_git(&["commit", "-m", &msg], Some(path))
    }
}

/// git add -A && git commit -m <message> (all-in-one)
pub fn git_commit_all(path: &str, message: &str) -> Result<GitResult> {
    let add_result = run_git(&["add", "-A"], Some(path))?;
    if !add_result.success {
        return Ok(add_result);
    }
    git_commit(path, message, false)
}

/// git stash [pop|list]
pub fn git_stash(path: &str, subcmd: &str) -> Result<GitResult> {
    match subcmd {
        "pop" => run_git(&["stash", "pop"], Some(path)),
        "list" => run_git(&["stash", "list"], Some(path)),
        "drop" => run_git(&["stash", "drop"], Some(path)),
        _ => run_git(&["stash"], Some(path)),
    }
}

// ─── Branch ──────────────────────────────────────────────────────────────────

/// git branch [--all]
pub fn git_branch_list(path: &str) -> Result<GitResult> {
    run_git(&["branch", "-a", "-v"], Some(path))
}

/// git checkout [-b] <branch>
pub fn git_checkout(path: &str, branch: &str, create: bool) -> Result<GitResult> {
    if create {
        run_git(&["checkout", "-b", branch], Some(path))
    } else {
        run_git(&["checkout", branch], Some(path))
    }
}

/// git merge <branch> [--no-ff]
pub fn git_merge(path: &str, branch: &str, no_ff: bool) -> Result<GitResult> {
    if no_ff {
        run_git(&["merge", "--no-ff", branch], Some(path))
    } else {
        run_git(&["merge", branch], Some(path))
    }
}

/// git rebase <branch>
pub fn git_rebase(path: &str, branch: &str) -> Result<GitResult> {
    run_git(&["rebase", branch], Some(path))
}

/// git branch -d <branch>
pub fn git_branch_delete(path: &str, branch: &str, force: bool) -> Result<GitResult> {
    if force {
        run_git(&["branch", "-D", branch], Some(path))
    } else {
        run_git(&["branch", "-d", branch], Some(path))
    }
}

// ─── Remote / push / pull ──────────────────────────────────────────────────────

/// git remote add <name> <url>
pub fn git_remote_add(path: &str, name: &str, url: &str) -> Result<GitResult> {
    run_git(&["remote", "add", name, url], Some(path))
}

/// git remote -v
pub fn git_remote_list(path: &str) -> Result<GitResult> {
    run_git(&["remote", "-v"], Some(path))
}

/// git push [remote] [branch]
pub fn git_push(path: &str, remote: &str, branch: &str, set_upstream: bool) -> Result<GitResult> {
    let remote = if remote.is_empty() { "origin" } else { remote };
    let branch = if branch.is_empty() { "HEAD" } else { branch };

    if set_upstream {
        run_git_with_timeout(
            &["push", "-u", remote, branch],
            Some(path), 60,
        )
    } else {
        run_git_with_timeout(
            &["push", remote, branch],
            Some(path), 60,
        )
    }
}

/// git pull [remote] [branch]
pub fn git_pull(path: &str, remote: &str, branch: &str) -> Result<GitResult> {
    let remote = if remote.is_empty() { "origin" } else { remote };
    if branch.is_empty() {
        run_git_with_timeout(&["pull", remote], Some(path), 60)
    } else {
        run_git_with_timeout(&["pull", remote, branch], Some(path), 60)
    }
}

/// git fetch [remote]
pub fn git_fetch(path: &str, remote: &str) -> Result<GitResult> {
    let remote = if remote.is_empty() { "origin" } else { remote };
    run_git_with_timeout(&["fetch", remote], Some(path), 60)
}

// ─── Tags ────────────────────────────────────────────────────────────────────

/// git tag [name] [message]
pub fn git_tag(path: &str, name: &str, message: &str) -> Result<GitResult> {
    if message.is_empty() {
        run_git(&["tag", name], Some(path))
    } else {
        run_git(&["tag", "-a", name, "-m", message], Some(path))
    }
}

/// git tag --list
pub fn git_tag_list(path: &str) -> Result<GitResult> {
    run_git(&["tag", "--list", "--sort=-version:refname"], Some(path))
}

// ─── Config ────────────────────────────────────────────────────────────────────

/// git config (local)
pub fn git_config(path: &str, key: &str, value: &str) -> Result<GitResult> {
    run_git(&["config", "--local", key, value], Some(path))
}

/// git config --global
pub fn git_config_global(key: &str, value: &str) -> Result<GitResult> {
    run_git(&["config", "--global", key, value], None)
}

/// Apply default git config (sets user info if not set)
pub fn git_setup_defaults(path: &str) -> Result<String> {
    let mut msgs = vec![];

    // check user.email
    let email_check = run_git(&["config", "user.email"], Some(path));
    if email_check.map(|r| r.output.trim().is_empty()).unwrap_or(true) {
        let _ = run_git(&["config", "--local", "user.email", "ai-agent@local"], Some(path));
        let _ = run_git(&["config", "--local", "user.name", "AI Agent"], Some(path));
        msgs.push("git user set: AI Agent <ai-agent@local>");
    }

    // set core.autocrlf
    let _ = run_git(&["config", "--local", "core.autocrlf", "input"], Some(path));
    // set pull.rebase
    let _ = run_git(&["config", "--local", "pull.rebase", "false"], Some(path));

    Ok(msgs.join("\n"))
}

// ─── Repository info ─────────────────────────────────────────────────────────

/// Return current branch name
pub fn git_current_branch(path: &str) -> Result<String> {
    run_git(&["branch", "--show-current"], Some(path))
        .map(|r| r.output.trim().to_string())
}

/// Return repository root path
pub fn git_root(path: &str) -> Result<String> {
    run_git(&["rev-parse", "--show-toplevel"], Some(path))
        .map(|r| r.output.trim().to_string())
}

/// List changed files
pub fn git_changed_files(path: &str) -> Result<Vec<String>> {
    run_git(&["diff", "--name-only", "HEAD"], Some(path))
        .map(|r| {
            r.output.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
}

/// git blame — show last commit/author per line
pub fn git_blame(path: &str, file: &str) -> Result<GitResult> {
    run_git(&["blame", "--line-porcelain", file], Some(path))
        .map(|r| {
            // convert porcelain format to readable summary
            let mut lines: Vec<String> = Vec::new();
            let mut commit = String::new();
            let mut author = String::new();
            let mut line_no = 0usize;

            for l in r.output.lines() {
                if l.starts_with('\t') {
                    // actual source line
                    let src_line = &l[1..];
                    lines.push(format!(
                        "{:4} {:7} {:20} | {}",
                        line_no,
                        &commit[..commit.len().min(7)],
                        &author[..author.len().min(20)],
                        src_line
                    ));
                } else if l.starts_with("author ") {
                    author = l[7..].trim().to_string();
                } else if l.len() >= 40 && !l.contains(' ') {
                    // commit hash line
                    commit = l.to_string();
                } else if l.starts_with("summary ") {
                    // skip
                } else {
                    // extract line number from header
                    let parts: Vec<&str> = l.split_whitespace().collect();
                    if parts.len() >= 3 {
                        line_no = parts[2].parse().unwrap_or(line_no);
                    }
                }
            }

            GitResult {
                output: if lines.is_empty() {
                    r.output
                } else {
                    format!("LINE  COMMIT  AUTHOR               | CODE\n{}", lines.join("\n"))
                },
                success: r.success,
            }
        })
}

/// List staged files
pub fn git_staged_files(path: &str) -> Result<Vec<String>> {
    run_git(&["diff", "--cached", "--name-only"], Some(path))
        .map(|r| {
            r.output.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
}

/// List remote branches
pub fn git_remote_branches(path: &str, remote: &str) -> Result<GitResult> {
    run_git(&["branch", "-r", "--list", &format!("{}/*", remote)], Some(path))
}

// ─── Default .gitignore ─────────────────────────────────────────────────────────

const DEFAULT_GITIGNORE: &str = r#"# OS
.DS_Store
Thumbs.db
*.swp
*.swo
*~

# IDE
.vscode/
.idea/
*.iml
*.suo
*.user

# Logs
*.log
logs/

# Temp
*.tmp
*.bak
*.orig

# Secrets
.env
.env.local
.env.*.local
*.pem
*.key
secrets/
credentials/

# Build outputs
dist/
build/
target/
out/
*.o
*.a
*.so
*.dylib
*.exe

# Dependencies
node_modules/
vendor/
.cargo/
__pycache__/
*.pyc
*.pyo
.pytest_cache/
venv/
.venv/

# Package lock (commit selectively)
# package-lock.json
# yarn.lock
"#;
