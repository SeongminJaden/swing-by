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

/// 툴 실행 결과
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

// ─── 시스템 프롬프트 ─────────────────────────────────────────────────────────

pub fn tool_descriptions() -> &'static str {
    r#"당신은 Rust로 구현된 풀스택 AI 에이전트입니다. 소프트웨어 개발 전 단계를 지원하며,
라이브러리/프레임워크를 사용할 때는 항상 research나 pkg_info로 최신 정보를 먼저 확인하고
공식 docs와 블로그를 동시에 분석하여 최신 베스트 프랙티스를 적용합니다.

=== 툴 사용 형식 ===
TOOL: <툴이름> <인자>

=== 파일 시스템 ===

TOOL: read_file <경로>
TOOL: write_file <경로> <내용>          (\n으로 줄바꿈)
TOOL: append_file <경로> <내용>
TOOL: edit_file <경로>
<<<OLD>>>기존 내용<<<NEW>>>새 내용<<<END>>>
TOOL: delete_file <경로>               (파일/디렉토리 삭제)
TOOL: move_file <원본> <대상>           (이동/이름변경)
TOOL: copy_file <원본> <대상>           (복사)
TOOL: mkdir <경로>                      (디렉토리 생성, 중간경로 포함)
TOOL: list_dir [경로]
TOOL: glob <패턴>                       (예: src/**/*.rs)
TOOL: grep <패턴> [경로]               (-i 플래그: 대소문자 무시)
TOOL: current_dir                       (현재 작업 디렉토리 확인)
TOOL: change_dir <경로>                (작업 디렉토리 변경)

=== 코드 실행 ===

TOOL: run_code <언어>
<코드>
지원: python/python3, javascript/js, typescript/ts, rust, go, bash/sh,
      ruby, php, perl, lua, r, java, c, c++/cpp, kotlin, swift, scala

TOOL: debug_code <언어>
<코드>

TOOL: shell <명령어>
  파이프, 리다이렉트 지원. 예: TOOL: shell "ls -la | head -20"

=== Git 레포지토리 관리 ===

TOOL: git_init [경로]                   (레포 초기화 + .gitignore 생성)
TOOL: git_clone <url> [경로]
TOOL: git_status [경로]
TOOL: git_diff [경로] [staged]         (staged=true: 스테이징 영역 diff)
TOOL: git_log [경로] [n=10]            (최근 n개 커밋 로그)
TOOL: git_show [경로] <ref>

TOOL: git_add [경로] [파일...]          (파일 없으면 전체 스테이징)
TOOL: git_commit [경로] <메시지>       (Conventional Commits 형식 권장)
TOOL: git_commit_all [경로] <메시지>   (add -A + commit 한번에)
TOOL: git_stash [경로] [pop|list|drop]

TOOL: git_branch [경로]                (브랜치 목록)
TOOL: git_branch [경로] <이름> create  (새 브랜치 생성)
TOOL: git_checkout [경로] <브랜치>
TOOL: git_checkout [경로] <브랜치> create  (새 브랜치 생성 후 전환)
TOOL: git_merge [경로] <브랜치> [no-ff=true]
TOOL: git_rebase [경로] <브랜치>
TOOL: git_branch_delete [경로] <브랜치> [force=true]

TOOL: git_remote_add [경로] <이름> <url>
TOOL: git_remote_list [경로]
TOOL: git_push [경로] [remote=origin] [branch=HEAD] [upstream=true]
TOOL: git_pull [경로] [remote=origin] [브랜치]
TOOL: git_fetch [경로] [remote=origin]

TOOL: git_tag [경로] <태그명> [메시지]
TOOL: git_tag_list [경로]
TOOL: git_config [경로] <key> <value>
TOOL: git_config_global <key> <value>
TOOL: commit_types                     (Conventional Commits 타입 목록)
TOOL: git_blame [경로] <파일>          (라인별 최종 수정자 확인)
TOOL: git_root [경로]                  (레포 루트 경로)
TOOL: git_changed_files [경로]         (변경된 파일 목록)
TOOL: git_staged_files [경로]          (스테이지된 파일 목록)
TOOL: git_remote_branches [경로] [remote=origin]  (원격 브랜치 목록)

=== 프로젝트 생성 ===

TOOL: project_init <타입> <이름> [경로]
  타입: rust, rust-lib, python, node, typescript, react, react-ts,
        vue, next, django, flask, fastapi, go, express, cpp, deno

TOOL: github_actions <프로젝트타입> <경로>  (CI/CD 워크플로우 생성)
TOOL: pr_template <경로>                     (PR/이슈 템플릿 생성)

=== 패키지 / 시스템 ===

TOOL: pkg_install <매니저> <패키지>    (apt, pip, npm, cargo, gem, go, snap...)
TOOL: pkg_remove <매니저> <패키지>
TOOL: pkg_upgrade <매니저> <패키지>    (특정 패키지 업그레이드)
TOOL: pkg_update <매니저>              (패키지 인덱스 갱신)
TOOL: pkg_list <매니저>
TOOL: pkg_search <매니저> <검색어>
TOOL: sysinfo
TOOL: process_list [필터]
TOOL: env_list [필터]                  (환경변수 목록, 필터 옵션)
TOOL: get_env <키>                     (환경변수 조회)
TOOL: set_env <키> <값>               (환경변수 설정)

=== 웹 ===

TOOL: web_fetch <URL>
TOOL: web_search <검색어>

=== 리서치 (최신 정보 수집) ===

TOOL: research <검색어> [페이지수=3]
  → 검색 + 상위 N개 페이지를 동시에 패치하여 통합 분석
  예: TOOL: research "fastapi best practices 2024" 3
  예: TOOL: research "react 19 new features" 2

TOOL: docs_fetch <URL> [최대글자수=6000]
  → 공식 문서 URL 직접 패치 + HTML 정리
  예: TOOL: docs_fetch https://docs.astro.build/en/getting-started/
  예: TOOL: docs_fetch https://react.dev/reference/react/hooks 8000

TOOL: pkg_info <생태계> <패키지명>
  → 최신 버전, 의존성, 다운로드 등 메타데이터 조회
  지원: npm, pip/pypi, cargo/crates, go, gem/ruby
  예: TOOL: pkg_info npm react
  예: TOOL: pkg_info pip fastapi
  예: TOOL: pkg_info cargo tokio

TOOL: pkg_versions <생태계> <패키지1> <패키지2> ...
  → 여러 패키지 최신 버전 동시 조회

=== 코드 품질 ===

TOOL: lint <언어> <경로>
  → 코드 품질 검사 (rustfmt/clippy, ruff/flake8, eslint, golangci-lint 등)

TOOL: format <언어> <경로>
  → 코드 자동 포맷팅 (cargo fmt, black/ruff, prettier, gofmt 등)

TOOL: test <언어> <경로> [필터]
  → 테스트 실행 (cargo test, pytest, jest, go test 등)

TOOL: build <언어> <경로>
  → 프로젝트 빌드

TOOL: create_venv <경로> [이름=.venv]
  → Python 가상환경 생성

TOOL: nvm_use <버전>                   (Node 버전 전환, 예: nvm_use 20, nvm_use --lts)

=== Docker / 컨테이너 ===

TOOL: docker_ps [all]                  (컨테이너 목록)
TOOL: docker_images                    (이미지 목록)
TOOL: docker_pull <이미지>
TOOL: docker_build <태그> <컨텍스트> [도커파일]
TOOL: docker_run <이미지> [옵션] [명령]
TOOL: docker_control <stop|start|restart|rm> <컨테이너>
TOOL: docker_logs <컨테이너> [tail=50]
TOOL: docker_exec <컨테이너> <명령>
TOOL: docker_compose <up|down|build|ps|logs> <경로> [detach=true]
TOOL: docker_inspect <컨테이너|이미지>
TOOL: docker_stats                     (컨테이너 리소스 현황)
TOOL: docker_network_ls                (네트워크 목록)
TOOL: docker_network_inspect <네트워크>
TOOL: docker_volume_ls                 (볼륨 목록)
TOOL: docker_volume_rm <볼륨>
TOOL: docker_prune [all]               (미사용 리소스 정리)
TOOL: dockerfile <언어> <프로젝트명> <경로>
  → Dockerfile + docker-compose.yml 자동 생성

=== 서브에이전트 ===

TOOL: sub_agent <태스크>
  → 독립 컨텍스트에서 서브에이전트 실행 (max 15턴)
  예: TOOL: sub_agent "FastAPI 프로젝트에 JWT 인증 구현"

TOOL: parallel_agent <t1> | <t2> | <t3>
  → 여러 태스크 병렬 처리
  예: TOOL: parallel_agent "백엔드 API 구현" | "프론트엔드 UI 구현" | "테스트 작성"

=== TODO ===

TOOL: todo_read
TOOL: todo_write
[{"id":"1","content":"할 일","status":"pending","priority":"high"}]

=== Git 컨벤션 가이드 ===

Conventional Commits 형식:
  <type>(<scope>): <description>

  feat(auth): add OAuth2 login
  fix(api): resolve NPE in user endpoint
  refactor(db): extract repository pattern
  docs(readme): update setup guide

브랜치 네이밍 컨벤션:
  feature/<이름>   새 기능 개발
  fix/<이름>       버그 수정
  hotfix/<이름>    긴급 수정
  release/<버전>   릴리스 준비
  docs/<이름>      문서 작업

=== 규칙 ===
- 파일 수정: read_file → edit_file (정확한 문자열 교체)
- 레포 생성: project_init → git_init → git_commit_all "chore: initial setup"
- 커밋은 반드시 Conventional Commits 형식 사용
- 외부 라이브러리 사용 전: research + pkg_info로 최신 버전/베스트프랙티스 확인
- 복잡한 작업: todo_write로 계획 수립 → 단계별 실행
- 코드 작성 후: lint → test → commit 순서 권장
- Docker 배포 포함 시: dockerfile 툴로 Dockerfile 자동 생성
- EXIT 단독 입력 시 종료"#
}

// ─── 툴 디스패치 ─────────────────────────────────────────────────────────────

pub async fn dispatch_tool(call: &ToolCall) -> ToolResult {
    let result: Result<String> = match call.name.as_str() {

        // ── 파일 시스템 ──────────────────────────────
        "read_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            read_file(path).map(|c| format!("=== {} ===\n{}", path, c))
        }
        "write_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let content = unescape(&call.args[1..].join(" "));
            write_file(path, &content).map(|_| format!("저장 완료: {}", path))
        }
        "append_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let content = unescape(&call.args[1..].join(" "));
            append_file(path, &content).map(|_| format!("추가 완료: {}", path))
        }
        "edit_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let old = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let new = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            edit_file(path, old, new)
        }
        "delete_file" | "remove_file" | "rm_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            delete_file(path).map(|_| format!("삭제 완료: {}", path))
        }
        "move_file" | "rename_file" | "mv" => {
            let src = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dst = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            move_file(src, dst).map(|_| format!("이동 완료: {} → {}", src, dst))
        }
        "copy_file" | "cp" => {
            let src = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dst = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            copy_file(src, dst).map(|bytes| format!("복사 완료: {} → {} ({} bytes)", src, dst, bytes))
        }
        "mkdir" | "make_dir" => {
            let path = call.args.join(" ");
            make_dir(path.trim()).map(|_| format!("디렉토리 생성: {}", path.trim()))
        }
        "current_dir" | "pwd" => {
            current_dir().map(|d| format!("현재 디렉토리: {}", d))
        }
        "change_dir" | "cd" => {
            let path = call.args.join(" ");
            change_dir(path.trim()).map(|_| format!("디렉토리 변경: {}", path.trim()))
        }
        "list_dir" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            list_dir(path).map(|items| items.join("\n"))
        }
        "glob" => {
            let pattern = call.args.join(" ");
            glob_files(pattern.trim()).map(|files| {
                if files.is_empty() { "일치하는 파일 없음".into() }
                else { format!("{} 개 파일\n{}", files.len(), files.join("\n")) }
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
                if lines.is_empty() { "일치하는 내용 없음".into() }
                else { format!("{} 개 결과\n{}", lines.len(), lines.join("\n")) }
            })
        }

        // ── 코드 실행 ────────────────────────────────
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

        // ── Git ──────────────────────────────────────
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
            // git_branch <path> <name> [create] → 이름이 있으면 생성
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
                // -a 같은 플래그: 목록
                git_branch_list(path).map(|r| r.output)
            } else {
                git_branch_list(path).map(|r| r.output)
            }
        }
        "git_checkout" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            // AI가 -b 플래그를 쓸 수 있음: git_checkout <path> -b <branch>
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
                if files.is_empty() { "변경된 파일 없음".to_string() }
                else { files.join("\n") }
            })
        }
        "git_staged_files" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_staged_files(path).map(|files| {
                if files.is_empty() { "스테이지된 파일 없음".to_string() }
                else { files.join("\n") }
            })
        }
        "git_remote_branches" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let remote = call.args.get(1).map(|s| s.as_str()).unwrap_or("origin");
            git_remote_branches(path, remote).map(|r| r.output)
        }

        // ── 프로젝트 생성 ─────────────────────────────
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

        // ── 패키지 / 시스템 ───────────────────────────
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
                Ok("환경변수 없음".to_string())
            } else {
                Ok(vars.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("\n"))
            }
        }
        "get_env" => {
            let key = call.args.first().map(|s| s.as_str()).unwrap_or("");
            match get_env(key) {
                Some(v) => Ok(format!("{}={}", key, v)),
                None => Ok(format!("'{}' 환경변수 없음", key)),
            }
        }
        "set_env" => {
            let key = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let val = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            set_env(key, val).map(|_| format!("설정 완료: {}={}", key, val))
        }

        // ── 웹 / 리서치 ──────────────────────────────
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

        // ── 코드 품질 ─────────────────────────────────
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

        // ── Docker ─────────────────────────────────────
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

        // ── 서브에이전트 ─────────────────────────────
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
                Ok("태스크 없음".to_string())
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

        // ── TODO ─────────────────────────────────────
        "todo_write" => {
            let json = call.args.first().map(|s| s.as_str()).unwrap_or("[]");
            todo_write(json)
        }
        "todo_read" => {
            todo_read().map(|items| {
                if items.is_empty() { "TODO 목록이 비어있습니다.".into() }
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
                format!("알 수 없는 툴: '{}'. /help 또는 tool_descriptions 참고", unknown)
            );
        }
    };

    match result {
        Ok(output) => ToolResult::ok(&call.name, output),
        Err(e) => ToolResult::err(&call.name, format!("오류: {}", e)),
    }
}
