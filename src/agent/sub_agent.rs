/// 서브에이전트 / 멀티에이전트 구현
///
/// 서브에이전트는 독립된 컨텍스트에서 주어진 태스크를 처리하고 결과를 반환합니다.
/// (재귀 호출을 피하기 위해 내부 툴 디스패치는 직접 구현)

use anyhow::Result;
use crate::models::{AgentResponse, Message, ToolCall};
use super::ollama::OllamaClient;

const SUB_AGENT_MAX_TURNS: usize = 15;

// ─── 내부 툴 디스패치 (재귀 방지용 독립 구현) ─────────────────────────────────

async fn dispatch_tool_inner(call: &ToolCall) -> String {
    use crate::tools::{
        append_file, copy_file, delete_file, edit_file, glob_files, grep_files,
        list_dir, make_dir, move_file, read_file, run_code, sysinfo,
        todo_write, web_fetch, web_search, write_file,
        git_status, git_diff, git_add, git_commit, git_commit_all,
        git_log, git_init, git_clone, git_branch_list, git_checkout,
        git_changed_files, git_staged_files,
    };

    fn unescape(s: &str) -> String {
        s.replace("\\n", "\n").replace("\\t", "\t").replace("\\r", "\r").replace("\\\\", "\\")
    }

    let result: Result<String> = match call.name.as_str() {
        "read_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            read_file(path).map(|c| format!("=== {} ===\n{}", path, c))
        }
        "write_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let content = unescape(&call.args[1..].join(" "));
            write_file(path, &content).map(|_| format!("저장: {}", path))
        }
        "append_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let content = unescape(&call.args[1..].join(" "));
            append_file(path, &content).map(|_| format!("추가: {}", path))
        }
        "edit_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let old = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let new = call.args.get(2).map(|s| s.as_str()).unwrap_or("");
            edit_file(path, old, new)
        }
        "delete_file" | "remove_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            delete_file(path).map(|_| format!("삭제: {}", path))
        }
        "move_file" | "mv" => {
            let src = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dst = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            move_file(src, dst).map(|_| format!("이동: {} → {}", src, dst))
        }
        "copy_file" | "cp" => {
            let src = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dst = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            copy_file(src, dst).map(|b| format!("복사: {} → {} ({} bytes)", src, dst, b))
        }
        "mkdir" | "make_dir" => {
            let path = call.args.join(" ");
            make_dir(path.trim()).map(|_| format!("디렉토리 생성: {}", path.trim()))
        }
        "list_dir" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            list_dir(path).map(|items| items.join("\n"))
        }
        "glob" => {
            let pattern = call.args.join(" ");
            glob_files(pattern.trim()).map(|f| {
                if f.is_empty() { "없음".into() } else { format!("{} 파일\n{}", f.len(), f.join("\n")) }
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
            grep_files(&pat, &path).map(|l| {
                if l.is_empty() { "없음".into() } else { format!("{} 결과\n{}", l.len(), l.join("\n")) }
            })
        }
        "run_code" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let code = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            run_code(lang, code).map(|r| r.to_string())
        }
        "shell" => {
            let cmd = call.args.join(" ");
            crate::tools::system::run_shell(&cmd).map(|r| r.to_string())
        }
        "sysinfo" => sysinfo().map(|r| r.output),
        "web_fetch" => {
            let url = call.args.join(" ");
            web_fetch(url.trim()).await
        }
        "web_search" => {
            let query = call.args.join(" ");
            web_search(query.trim()).await
        }
        "todo_write" => {
            let json = call.args.first().map(|s| s.as_str()).unwrap_or("[]");
            todo_write(json)
        }
        "pkg_install" => {
            let mgr = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let pkg = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            crate::tools::pkg_install(mgr, pkg).map(|r| r.output)
        }
        "git_status" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_status(path).map(|r| r.output)
        }
        "git_diff" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let staged = call.args.get(1).map(|s| s == "staged").unwrap_or(false);
            git_diff(path, staged).map(|r| r.output)
        }
        "git_log" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let n = call.args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10usize);
            git_log(path, n).map(|r| r.output)
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
        "git_init" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_init(path).map(|r| r.output)
        }
        "git_clone" => {
            let url = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let dest = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            git_clone(url, dest).map(|r| r.output)
        }
        "git_branch" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_branch_list(path).map(|r| r.output)
        }
        "git_checkout" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            let branch = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            let create = call.args.get(2).map(|s| s == "create").unwrap_or(false);
            git_checkout(path, branch, create).map(|r| r.output)
        }
        "git_changed_files" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_changed_files(path).map(|f| f.join("\n"))
        }
        "git_staged_files" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            git_staged_files(path).map(|f| f.join("\n"))
        }
        unknown => Err(anyhow::anyhow!("알 수 없는 툴: '{}'", unknown)),
    };

    match result {
        Ok(out) => out,
        Err(e) => format!("오류: {}", e),
    }
}

// ─── 응답 Parsing ────────────────────────────────────────────────────────────────

fn parse_sub_response(text: &str) -> AgentResponse {
    let trimmed = text.trim();

    if trimmed == "EXIT" || trimmed.starts_with("DONE:") {
        return AgentResponse::Exit;
    }

    if let Some(rest) = trimmed.strip_prefix("TOOL:") {
        let rest = rest.trim();
        let first_word = rest.split_whitespace().next().unwrap_or("");

        if matches!(first_word, "run_code" | "debug_code") {
            let after = rest.trim_start_matches(first_word).trim_start();
            let (lang, code) = after.split_once('\n')
                .map(|(l, c)| (l.trim().to_string(), c.to_string()))
                .unwrap_or_else(|| (after.to_string(), String::new()));
            return AgentResponse::ToolCall(ToolCall {
                name: first_word.to_string(),
                args: vec![lang, code],
            });
        }

        if first_word == "edit_file" {
            let after = rest.trim_start_matches(first_word).trim_start();
            let path = after.lines().next().unwrap_or("").trim().to_string();
            let body = after.splitn(2, '\n').nth(1).unwrap_or("");
            let (old_s, new_s) = parse_edit_delimiters(body);
            return AgentResponse::ToolCall(ToolCall {
                name: "edit_file".to_string(),
                args: vec![path, old_s, new_s],
            });
        }

        let parts = shlex::split(rest).unwrap_or_else(|| {
            rest.split_whitespace().map(|s| s.to_string()).collect()
        });
        if let Some((name, args)) = parts.split_first() {
            return AgentResponse::ToolCall(ToolCall {
                name: name.clone(),
                args: args.to_vec(),
            });
        }
    }

    AgentResponse::Text(text.to_string())
}

fn parse_edit_delimiters(body: &str) -> (String, String) {
    let lower = body.to_lowercase();
    if let (Some(op), Some(np), Some(ep)) =
        (lower.find("<<<old>>>"), lower.find("<<<new>>>"), lower.find("<<<end>>>"))
    {
        let os = op + "<<<old>>>".len();
        let ns = np + "<<<new>>>".len();
        if os <= np && ns <= ep {
            return (body[os..np].trim().to_string(), body[ns..ep].trim().to_string());
        }
    }
    (String::new(), String::new())
}

// ─── 서브에이전트 ─────────────────────────────────────────────────────────────

fn sub_system_prompt(model: &str) -> String {
    format!(
        "모델: {}\n당신은 메인 에이전트의 서브에이전트입니다.\
태스크를 완료하면 'DONE: <결과>'로 응답하거나 일반 텍스트로 최종 답변을 제공하세요.\n\n{}",
        model,
        crate::agent::tools::tool_descriptions()
    )
}

/// 단일 서브에이전트 실행
pub async fn run_sub_agent(task: &str, ollama_url: &str, model: &str) -> Result<String> {
    let client = OllamaClient::new(ollama_url, model);
    let mut history = vec![
        Message::system(&sub_system_prompt(model)),
        Message::user(&format!("태스크: {}", task)),
    ];
    let mut last = String::from("완료");

    for turn in 0..SUB_AGENT_MAX_TURNS {
        let response = match client.chat(history.clone()).await {
            Ok(r) => r.message.content,
            Err(e) => return Ok(format!("[서브에이전트 오류 (turn={})]: {}", turn, e)),
        };
        last = response.clone();

        if response.trim().starts_with("DONE:") {
            let result = response.trim().trim_start_matches("DONE:").trim();
            return Ok(format!("[서브에이전트 {}턴 완료]\n{}", turn + 1, result));
        }

        match parse_sub_response(&response) {
            AgentResponse::ToolCall(tc) => {
                let out = dispatch_tool_inner(&tc).await;
                history.push(Message::assistant(&response));
                history.push(Message::tool(format!("툴 '{}' 결과:\n{}", tc.name, out)));
            }
            AgentResponse::Text(_) => {
                history.push(Message::assistant(&response));
                if turn > 0 { break; }
            }
            AgentResponse::Exit => break,
        }

        if history.len() > 30 {
            let sys = history.first().cloned();
            let recent: Vec<_> = history.iter().rev().take(20).rev().cloned().collect();
            history = sys.into_iter().collect();
            history.extend(recent);
        }
    }

    Ok(format!("[서브에이전트 최대{}턴 도달]\n{}", SUB_AGENT_MAX_TURNS, last))
}

// ─── 멀티에이전트 (순차 실행) ────────────────────────────────────────────────

/// 여러 태스크를 순차적으로 처리하고 결과 목록을 반환
/// (병렬 실행은 Send 제약으로 인해 순차로 처리)
pub async fn run_multi_agent(
    tasks: Vec<String>,
    ollama_url: &str,
    model: &str,
) -> Result<Vec<String>> {
    let mut results = vec![];
    for (i, task) in tasks.iter().enumerate() {
        println!("\n[에이전트-{}] 시작: {}", i + 1, &task[..task.len().min(60)]);
        let result = run_sub_agent(task, ollama_url, model).await
            .unwrap_or_else(|e| format!("오류: {}", e));
        println!("[에이전트-{}] 완료", i + 1);
        results.push(format!("[에이전트-{}]\n{}", i + 1, result));
    }
    Ok(results)
}

/// 태스크를 서브태스크로 분해하여 병렬 처리
#[allow(dead_code)]
pub async fn run_parallel_task(
    task: &str,
    ollama_url: &str,
    model: &str,
) -> Result<String> {
    let client = OllamaClient::new(ollama_url, model);
    let decompose_prompt = format!(
        "다음 태스크를 독립적인 서브태스크로 분해하세요.\n\
각 서브태스크는 한 줄씩, '- ' 접두사로 작성. 최대 4개.\n\
분해 불필요하면 '- {}' 하나만 출력.\n\n태스크: {}",
        task, task
    );

    let decomp_history = vec![
        Message::system("태스크를 독립 서브태스크로 분해하는 도우미"),
        Message::user(&decompose_prompt),
    ];

    let subtasks: Vec<String> = client.chat(decomp_history).await
        .map(|r| r.message.content)
        .unwrap_or_else(|_| format!("- {}", task))
        .lines()
        .filter(|l| l.trim_start().starts_with('-'))
        .map(|l| l.trim_start_matches('-').trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let subtasks = if subtasks.is_empty() { vec![task.to_string()] } else { subtasks };

    println!("\n[병렬에이전트] {}개 서브태스크:", subtasks.len());
    for (i, st) in subtasks.iter().enumerate() {
        println!("  [{}] {}", i + 1, &st[..st.len().min(70)]);
    }

    let results = run_multi_agent(subtasks, ollama_url, model).await?;
    Ok(results.join("\n\n---\n\n"))
}
