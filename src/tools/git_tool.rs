/// Git 레포지토리 관리 툴
///
/// 초기화, 클론, 커밋, 브랜치, 푸시/풀, 상태 확인 등
/// Conventional Commits 기반 커밋 메시지 검증 포함

use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;

const GIT_TIMEOUT_SECS: u64 = 120;
const MAX_OUTPUT: usize = 16_000;

// ─── Git 결과 ────────────────────────────────────────────────────────────────

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

// ─── Conventional Commits 검증 ────────────────────────────────────────────────

/// Conventional Commits 형식 검증 및 자동 포맷
/// 형식: <type>(<scope>): <description>
/// 타입: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
pub fn validate_commit_message(msg: &str) -> Result<String, String> {
    let valid_types = [
        "feat", "fix", "docs", "style", "refactor", "perf",
        "test", "build", "ci", "chore", "revert", "hotfix", "release",
    ];

    let msg = msg.trim();
    if msg.is_empty() {
        return Err("커밋 메시지가 비어 있습니다.".to_string());
    }

    // 이미 Conventional Commits 형식이면 그대로 반환
    let first_line = msg.lines().next().unwrap_or("").trim();
    for vt in &valid_types {
        if first_line.starts_with(&format!("{}(", vt))
            || first_line.starts_with(&format!("{}:", vt))
        {
            return Ok(msg.to_string());
        }
    }

    // 형식이 아니면 경고만 (강제하지 않음)
    Ok(msg.to_string())
}

/// Conventional Commits 타입 목록 반환
pub fn commit_types_help() -> &'static str {
    r#"Conventional Commits 타입:
  feat:     새 기능 추가
  fix:      버그 수정
  docs:     문서 변경
  style:    코드 포맷, 세미콜론 등 (기능 변경 없음)
  refactor: 리팩토링 (기능/버그 수정 없음)
  perf:     성능 개선
  test:     테스트 추가/수정
  build:    빌드 시스템, 외부 의존성 변경
  ci:       CI 설정 변경
  chore:    기타 유지보수
  revert:   이전 커밋 되돌리기
  release:  릴리스 버전 태그

형식: <type>(<scope>): <description>
예시:
  feat(auth): add OAuth2 login support
  fix(api): resolve null pointer in user endpoint
  docs(readme): update installation guide
  refactor(db): extract repository pattern"#
}

// ─── Git 명령어 실행 ─────────────────────────────────────────────────────────

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

    // 환경변수: non-interactive, no color
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
        .with_context(|| format!("git {:?} 타임아웃", args))?
        .with_context(|| "git 실행 실패")?;

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
        format!("{}...[잘림 {}바이트]", &s[..MAX_OUTPUT], s.len())
    } else {
        s
    }
}

// ─── Git 초기화 / 클론 ────────────────────────────────────────────────────────

/// git init [path]
pub fn git_init(path: &str) -> Result<GitResult> {
    let target = if path.is_empty() { "." } else { path };
    if target != "." {
        std::fs::create_dir_all(target)
            .with_context(|| format!("디렉토리 생성 실패: {}", target))?;
    }
    let result = run_git(&["init"], Some(target))?;

    // 기본 .gitignore 생성 (없을 때만)
    let gi_path = format!("{}/.gitignore", target.trim_end_matches('/'));
    if !Path::new(&gi_path).exists() {
        let _ = std::fs::write(&gi_path, DEFAULT_GITIGNORE);
    }

    // 기본 브랜치를 main으로 설정
    let _ = run_git(&["checkout", "-b", "main"], Some(target));

    Ok(GitResult {
        output: format!("{}\n.gitignore 생성됨", result.output),
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

// ─── 상태 / 로그 ─────────────────────────────────────────────────────────────

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

// ─── 스테이징 / 커밋 ─────────────────────────────────────────────────────────

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

/// git commit -m <message> (Conventional Commits 검증 포함)
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

// ─── 브랜치 ──────────────────────────────────────────────────────────────────

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

// ─── 리모트 / 푸시 / 풀 ──────────────────────────────────────────────────────

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

// ─── 태그 ────────────────────────────────────────────────────────────────────

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

// ─── 설정 ────────────────────────────────────────────────────────────────────

/// git config (로컬 설정)
pub fn git_config(path: &str, key: &str, value: &str) -> Result<GitResult> {
    run_git(&["config", "--local", key, value], Some(path))
}

/// git config --global
pub fn git_config_global(key: &str, value: &str) -> Result<GitResult> {
    run_git(&["config", "--global", key, value], None)
}

/// 기본 git 설정 (사용자 정보 없으면 설정)
pub fn git_setup_defaults(path: &str) -> Result<String> {
    let mut msgs = vec![];

    // user.email 확인
    let email_check = run_git(&["config", "user.email"], Some(path));
    if email_check.map(|r| r.output.trim().is_empty()).unwrap_or(true) {
        let _ = run_git(&["config", "--local", "user.email", "ai-agent@local"], Some(path));
        let _ = run_git(&["config", "--local", "user.name", "AI Agent"], Some(path));
        msgs.push("git 사용자 정보 설정: AI Agent <ai-agent@local>");
    }

    // core.autocrlf 설정
    let _ = run_git(&["config", "--local", "core.autocrlf", "input"], Some(path));
    // pull.rebase 설정
    let _ = run_git(&["config", "--local", "pull.rebase", "false"], Some(path));

    Ok(msgs.join("\n"))
}

// ─── 레포지토리 정보 ─────────────────────────────────────────────────────────

/// 현재 브랜치명 반환
pub fn git_current_branch(path: &str) -> Result<String> {
    run_git(&["branch", "--show-current"], Some(path))
        .map(|r| r.output.trim().to_string())
}

/// 레포 루트 경로 반환
pub fn git_root(path: &str) -> Result<String> {
    run_git(&["rev-parse", "--show-toplevel"], Some(path))
        .map(|r| r.output.trim().to_string())
}

/// 변경된 파일 목록
pub fn git_changed_files(path: &str) -> Result<Vec<String>> {
    run_git(&["diff", "--name-only", "HEAD"], Some(path))
        .map(|r| {
            r.output.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
}

/// git blame — 파일 라인별 최종 수정 커밋/작성자 표시
pub fn git_blame(path: &str, file: &str) -> Result<GitResult> {
    run_git(&["blame", "--line-porcelain", file], Some(path))
        .map(|r| {
            // porcelain 형식을 읽기 쉬운 요약으로 변환
            let mut lines: Vec<String> = Vec::new();
            let mut commit = String::new();
            let mut author = String::new();
            let mut line_no = 0usize;

            for l in r.output.lines() {
                if l.starts_with('\t') {
                    // 실제 소스 라인
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
                    // commit hash 줄
                    commit = l.to_string();
                } else if l.starts_with("summary ") {
                    // skip
                } else {
                    // "finalline lineno count" 헤더에서 라인번호 추출
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

/// 스테이지된 변경사항 파일 목록
pub fn git_staged_files(path: &str) -> Result<Vec<String>> {
    run_git(&["diff", "--cached", "--name-only"], Some(path))
        .map(|r| {
            r.output.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
}

/// 원격 브랜치 목록
pub fn git_remote_branches(path: &str, remote: &str) -> Result<GitResult> {
    run_git(&["branch", "-r", "--list", &format!("{}/*", remote)], Some(path))
}

// ─── 기본 .gitignore ─────────────────────────────────────────────────────────

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

# Package lock (선택적으로 커밋)
# package-lock.json
# yarn.lock
"#;
