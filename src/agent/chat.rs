use anyhow::Result;
use tracing::debug;

use crate::agent::{
    ollama::OllamaClient,
    tools::{dispatch_tool, tool_descriptions},
};
use crate::models::{AgentResponse, Message, ToolCall};

// ─── 코드 펜스 제거 ──────────────────────────────────────────────────────────

fn strip_code_fence(code: &str) -> String {
    let trimmed = code.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let after_open = match trimmed.find('\n') {
        Some(pos) => &trimmed[pos + 1..],
        None => return trimmed.to_string(),
    };
    if let Some(close_pos) = after_open.rfind("```") {
        after_open[..close_pos].trim().to_string()
    } else {
        after_open.trim().to_string()
    }
}

// ─── 단일 TOOL 블록 Parsing ─────────────────────────────────────────────────────

fn parse_single_tool(tool_text: &str) -> Option<ToolCall> {
    let rest = tool_text.trim();
    if rest.is_empty() {
        return None;
    }

    let (tool_name, after_name) = match rest.split_once(|c: char| c.is_whitespace()) {
        Some((name, r)) => (name.trim().to_string(), r.trim_start()),
        None => (rest.trim().to_string(), ""),
    };

    // 코드 실행 툴: 첫 줄이 언어, 나머지가 코드
    if tool_name == "run_code" || tool_name == "debug_code" {
        let (lang, raw_code) = match after_name.split_once('\n') {
            Some((l, c)) => (l.trim().to_string(), c.to_string()),
            None => match after_name.split_once(|c: char| c.is_whitespace()) {
                Some((l, c)) => (l.to_string(), c.to_string()),
                None => (after_name.to_string(), String::new()),
            },
        };
        let code = strip_code_fence(&raw_code);
        return Some(ToolCall { name: tool_name, args: vec![lang, code] });
    }

    // edit_file: 경로 + <<<OLD>>>...<<<NEW>>>...<<<END>>>
    if tool_name == "edit_file" {
        let path = after_name.lines().next().unwrap_or("").trim().to_string();
        let body = after_name.splitn(2, '\n').nth(1).unwrap_or("");
        let (old_str, new_str) = parse_edit_delimiters(body);
        return Some(ToolCall { name: tool_name, args: vec![path, old_str, new_str] });
    }

    // todo_write: JSON 멀티라인
    if tool_name == "todo_write" {
        let json = after_name.trim_start_matches('\n').trim().to_string();
        return Some(ToolCall { name: tool_name, args: vec![json] });
    }

    // write_file: 경로 + 내용
    if tool_name == "write_file" {
        let mut lines = after_name.splitn(2, '\n');
        let first = lines.next().unwrap_or("").trim();
        let rest_content = lines.next().unwrap_or("");
        // 경로가 따옴표 없이 첫 번째 토큰인 경우
        let (path, content) = if !rest_content.is_empty() {
            (first.to_string(), rest_content.to_string())
        } else {
            // 한 줄 형식: write_file path "content"
            let parts = shlex::split(after_name).unwrap_or_else(|| {
                after_name.split_whitespace().map(|s| s.to_string()).collect()
            });
            if parts.len() >= 2 {
                (parts[0].clone(), parts[1..].join(" "))
            } else {
                (after_name.to_string(), String::new())
            }
        };
        return Some(ToolCall { name: tool_name, args: vec![path, content] });
    }

    // 나머지는 shlex Parsing
    let parts = shlex::split(rest).unwrap_or_else(|| {
        rest.split_whitespace().map(|s| s.to_string()).collect()
    });
    if let Some((name, args)) = parts.split_first() {
        return Some(ToolCall { name: name.clone(), args: args.to_vec() });
    }

    None
}

// ─── 멀티-TOOL 응답 Parsing ─────────────────────────────────────────────────────
//
// AI가 한 응답에 여러 TOOL: 블록을 출력할 수 있음.
// 각 "TOOL:" 접두사를 기준으로 분리하여 모두 실행.

pub fn parse_response_pub(text: &str) -> AgentResponse { parse_response(text) }

fn parse_response(text: &str) -> AgentResponse {
    let trimmed = text.trim();

    if trimmed == "EXIT" {
        return AgentResponse::Exit;
    }

    // TOOL: 접두사가 하나라도 있으면 툴 호출로 처리
    if !trimmed.contains("TOOL:") {
        return AgentResponse::Text(text.to_string());
    }

    // 첫 번째 TOOL: 이전 텍스트는 무시하거나 접두 설명으로 처리
    // 여러 TOOL: 블록을 분리
    let tool_blocks = split_tool_blocks(trimmed);
    if tool_blocks.is_empty() {
        return AgentResponse::Text(text.to_string());
    }

    // 첫 번째 툴 반환 (나머지는 MultiTool에서 처리)
    if tool_blocks.len() == 1 {
        if let Some(tc) = parse_single_tool(tool_blocks[0].trim()) {
            return AgentResponse::ToolCall(tc);
        }
    }

    // 여러 툴: MultiTool 변형 (ToolCall의 name에 "__multi__" 마커 사용)
    let calls: Vec<ToolCall> = tool_blocks
        .iter()
        .filter_map(|block| parse_single_tool(block.trim()))
        .collect();

    if calls.is_empty() {
        return AgentResponse::Text(text.to_string());
    }
    if calls.len() == 1 {
        return AgentResponse::ToolCall(calls.into_iter().next().unwrap());
    }

    // 여러 툴을 직렬화하여 단일 ToolCall로 포장
    // args[0] = "__multi__", args[1..] = JSON 직렬화된 각 툴
    let serialized: Vec<String> = calls
        .iter()
        .map(|tc| {
            serde_json::json!({
                "name": tc.name,
                "args": tc.args
            })
            .to_string()
        })
        .collect();

    AgentResponse::ToolCall(ToolCall {
        name: "__multi__".to_string(),
        args: serialized,
    })
}

/// "TOOL:" 접두사를 기준으로 텍스트를 분리, 각 블록의 "TOOL:" 이후 내용만 추출
fn split_tool_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut rest = text;

    loop {
        match rest.find("TOOL:") {
            None => break,
            Some(pos) => {
                let after = &rest[pos + 5..]; // "TOOL:".len() == 5
                // 다음 TOOL: 위치 찾기
                match after.find("TOOL:") {
                    Some(next) => {
                        blocks.push(after[..next].trim());
                        rest = &after[next..];
                    }
                    None => {
                        blocks.push(after.trim());
                        break;
                    }
                }
            }
        }
    }

    blocks
}

fn parse_edit_delimiters(body: &str) -> (String, String) {
    let lower = body.to_lowercase();
    let old_tag = "<<<old>>>";
    let new_tag = "<<<new>>>";
    let end_tag = "<<<end>>>";

    if let (Some(old_pos), Some(new_pos), Some(end_pos)) =
        (lower.find(old_tag), lower.find(new_tag), lower.find(end_tag))
    {
        let old_start = old_pos + old_tag.len();
        let new_start = new_pos + new_tag.len();
        if old_start <= new_pos && new_start <= end_pos {
            let old = body[old_start..new_pos].trim().to_string();
            let new = body[new_start..end_pos].trim().to_string();
            return (old, new);
        }
    }
    (String::new(), String::new())
}

// ─── 히스토리 자동 압축 ───────────────────────────────────────────────────────

const MAX_HISTORY: usize = 60;
const KEEP_RECENT: usize = 40;

fn compact_history(history: &mut Vec<Message>) {
    if history.len() <= MAX_HISTORY {
        return;
    }
    let system_msgs: Vec<Message> = history
        .iter()
        .filter(|m| matches!(m.role, crate::models::Role::System))
        .cloned()
        .collect();

    let non_system: Vec<Message> = history
        .iter()
        .filter(|m| !matches!(m.role, crate::models::Role::System))
        .cloned()
        .collect();

    let keep_from = non_system.len().saturating_sub(KEEP_RECENT);
    let kept = non_system[keep_from..].to_vec();

    *history = system_msgs;
    history.push(Message::tool(
        "[이전 대화 내용이 컨텍스트 한계로 생략되었습니다.]".to_string(),
    ));
    history.extend(kept);
}

// ─── CLAUDE.md 자동 로딩 ─────────────────────────────────────────────────────

/// 전역(~/.claude/CLAUDE.md)과 프로젝트 CLAUDE.md 파일을 자동으로 읽어 반환
pub fn load_claude_md() -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1) 전역 설정: ~/.claude/CLAUDE.md
    if let Ok(home) = std::env::var("HOME") {
        let global = std::path::PathBuf::from(&home).join(".claude").join("CLAUDE.md");
        if let Ok(content) = std::fs::read_to_string(&global) {
            if !content.trim().is_empty() {
                parts.push(format!("## 전역 설정 (~/.claude/CLAUDE.md)\n{}", content.trim()));
            }
        }
    }

    // 2) 프로젝트 설정: cwd에서 git root까지 CLAUDE.md 탐색
    let mut dir = std::env::current_dir().ok();
    let mut visited = std::collections::HashSet::new();
    while let Some(d) = dir {
        let key = d.to_string_lossy().to_string();
        if visited.contains(&key) { break; }
        visited.insert(key);

        let claude_md = d.join("CLAUDE.md");
        if let Ok(content) = std::fs::read_to_string(&claude_md) {
            if !content.trim().is_empty() {
                parts.push(format!("## 프로젝트 설정 ({}/CLAUDE.md)\n{}", d.display(), content.trim()));
            }
        }

        // git root에서 멈춤
        if d.join(".git").exists() { break; }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("\n\n=== CLAUDE.md ===\n{}", parts.join("\n\n"))
    }
}

// ─── 컨텍스트 윈도우 추정 ────────────────────────────────────────────────────

/// 히스토리 전체의 추정 토큰 수 (chars / 4)
fn estimate_tokens(history: &[Message]) -> usize {
    history.iter().map(|m| m.content.len() / 4).sum()
}

/// 컨텍스트 사용률 막대 (가정: 모델 컨텍스트 128k tokens)
fn context_bar(used: usize, total: usize) -> String {
    let pct = (used * 100) / total.max(1);
    let filled = pct / 5;  // 20칸 막대
    let bar: String = (0..20).map(|i| if i < filled { '█' } else { '░' }).collect();
    format!("[{}] {}% ({}/{}k)", bar, pct, used / 1000, total / 1000)
}

// ─── 세션 저장/불러오기 ───────────────────────────────────────────────────────

const SESSION_FILE: &str = ".ai_session.json";
const MEMORY_FILE: &str = ".ai_memory.json";
const CONFIG_FILE: &str = ".ai_config.json";

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mins = (secs % 3600) / 60;
    let hours = (secs % 86400) / 3600;
    let days = secs / 86400;
    // 간단한 날짜: epoch + 일수 (정확하지 않지만 식별에 충분)
    format!("day{} {:02}:{:02}", days, hours, mins)
}

fn session_file() -> String {
    match std::env::var("AI_SESSION_NAME") {
        Ok(name) if !name.is_empty() => format!(".ai_session_{}.json", name),
        _ => SESSION_FILE.to_string(),
    }
}

fn save_session(history: &[Message]) {
    let path = session_file();
    if let Ok(json) = serde_json::to_string(history) {
        if std::fs::write(&path, json).is_err() {
            eprintln!("[경고] 세션 저장 실패: {}", path);
        }
    }
}

fn load_session() -> Vec<Message> {
    std::fs::read_to_string(session_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ─── 메모리 관리 ─────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MemoryEntry {
    id: usize,
    note: String,
    created: String,
}

fn memory_load() -> Vec<MemoryEntry> {
    std::fs::read_to_string(MEMORY_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn memory_save(entries: &[MemoryEntry]) {
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        if std::fs::write(MEMORY_FILE, json).is_err() {
            eprintln!("[경고] 메모리 저장 실패: {}", MEMORY_FILE);
        }
    }
}

// ─── 설정 관리 ───────────────────────────────────────────────────────────────

fn config_load() -> serde_json::Value {
    std::fs::read_to_string(CONFIG_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn config_save(cfg: &serde_json::Value) {
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        if std::fs::write(CONFIG_FILE, json).is_err() {
            eprintln!("[경고] 설정 저장 실패: {}", CONFIG_FILE);
        }
    }
}

// ─── 세션 통계 ───────────────────────────────────────────────────────────────

struct SessionStats {
    turns: usize,
    tool_calls: usize,
    est_prompt_tokens: usize,    // 추정 (chars / 4)
    est_response_tokens: usize,
    start: std::time::Instant,
}

impl SessionStats {
    fn new() -> Self {
        Self {
            turns: 0,
            tool_calls: 0,
            est_prompt_tokens: 0,
            est_response_tokens: 0,
            start: std::time::Instant::now(),
        }
    }

    fn add_prompt(&mut self, text: &str) {
        self.est_prompt_tokens += text.len() / 4;
    }

    fn add_response(&mut self, text: &str) {
        self.est_response_tokens += text.len() / 4;
        self.turns += 1;
    }

    fn add_tool(&mut self) {
        self.tool_calls += 1;
    }

    fn total(&self) -> usize {
        self.est_prompt_tokens + self.est_response_tokens
    }

    fn elapsed(&self) -> String {
        let s = self.start.elapsed().as_secs();
        if s < 60 { format!("{}초", s) }
        else if s < 3600 { format!("{}분 {}초", s/60, s%60) }
        else { format!("{}시간 {}분", s/3600, (s%3600)/60) }
    }
}

// ─── AI 기반 히스토리 요약 압축 ──────────────────────────────────────────────

/// 오래된 메시지를 AI로 요약하여 압축 (단순 삭제 대신 요약 보존)
async fn compact_with_summary(
    history: &mut Vec<Message>,
    client: &OllamaClient,
) -> bool {
    if history.len() <= MAX_HISTORY {
        return false;
    }

    let system_msgs: Vec<Message> = history
        .iter()
        .filter(|m| matches!(m.role, crate::models::Role::System))
        .cloned()
        .collect();

    let non_system: Vec<Message> = history
        .iter()
        .filter(|m| !matches!(m.role, crate::models::Role::System))
        .cloned()
        .collect();

    let cut = non_system.len().saturating_sub(KEEP_RECENT);
    let to_summarize = &non_system[..cut];
    let keep = non_system[cut..].to_vec();

    // If nothing to summarize, just truncate
    if to_summarize.is_empty() {
        *history = system_msgs;
        history.extend(keep);
        return true;
    }

    // Request summary from AI
    let conversation_text: String = to_summarize.iter()
        .map(|m| format!("[{:?}] {}", m.role, crate::utils::trunc(&m.content, 500)))
        .collect::<Vec<_>>()
        .join("\n");

    let summary_prompt = format!(
        "다음 대화 기록을 핵심 정보만 남겨 한국어로 간결하게 요약하세요.\n\
         코드 변경사항, 결정사항, 주요 컨텍스트를 포함하세요.\n\
         요약은 300자 이내로 작성하세요.\n\n{}",
        conversation_text
    );

    let summary_msgs = vec![
        Message::system("당신은 대화 요약 전문가입니다."),
        Message::user(&summary_prompt),
    ];

    let summary = client.chat(summary_msgs).await
        .map(|r| r.message.content)
        .unwrap_or_else(|_| format!("[이전 {} 메시지 요약 실패 — 내용 생략]", to_summarize.len()));

    *history = system_msgs;
    history.push(Message::tool(format!("[대화 요약] {}", summary)));
    history.extend(keep);
    true
}

// ─── 헤드리스(비대화형) 단일 실행 ────────────────────────────────────────────

/// --print 모드: 단일 프롬프트를 실행하고 결과를 stdout으로 출력 후 종료
pub async fn run_print_mode(client: &OllamaClient, prompt: &str) -> Result<()> {
    use std::io::Write;

    let claude_md = load_claude_md();
    let system_prompt = format!("모델: {}\n\n{}{}", client.model(), tool_descriptions(), claude_md);

    let mut history = vec![
        Message::system(&system_prompt),
        Message::user(prompt),
    ];

    // 최대 10번 툴 호출 루프
    for _ in 0..10 {
        let ai_text = client.chat_stream(history.clone(), |token| {
            print!("{}", token);
            let _ = std::io::stdout().flush();
        }).await?;
        println!();

        match crate::agent::chat::parse_response_pub(&ai_text) {
            crate::models::AgentResponse::Exit => break,
            crate::models::AgentResponse::Text(_) => break,
            crate::models::AgentResponse::ToolCall(tc) if tc.name == "__multi__" => {
                let mut results = Vec::new();
                for raw in &tc.args {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
                    let name = val["name"].as_str().unwrap_or("").to_string();
                    let args: Vec<String> = val["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    let result = crate::agent::tools::dispatch_tool(&crate::models::ToolCall { name: name.clone(), args }).await;
                    results.push(format!("툴 '{}' 결과:\n{}", name, result.output));
                }
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(results.join("\n\n")));
            }
            crate::models::AgentResponse::ToolCall(tc) => {
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("툴 '{}' 결과:\n{}", tc.name, result.output)));
            }
        }
    }

    Ok(())
}

// ─── 멀티툴 실행 Helpers ────────────────────────────────────────────────────────

async fn execute_multi_tool(
    serialized_calls: &[String],
    history: &mut Vec<Message>,
    ai_text: &str,
) -> bool {
    // __multi__ 마커: 직렬화된 JSON ToolCall 목록 실행
    let mut any_success = false;
    let mut tool_results = Vec::new();

    history.push(Message::assistant(ai_text));

    for raw in serialized_calls {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
        let name = val["name"].as_str().unwrap_or("").to_string();
        let args: Vec<String> = val["args"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let tc = ToolCall { name: name.clone(), args };

        let args_preview: Vec<String> = tc.args.iter()
            .map(|a| { let s = a.replace('\n', "↵"); if s.len() > 60 { format!("{}...", crate::utils::trunc(&s, 60)) } else { s } })
            .collect();
        println!("\n┌─[툴] {} {}", tc.name, args_preview.join(" "));

        let result = dispatch_tool(&tc).await;
        let icon = if result.success { "✓" } else { "✗" };
        println!("└─[{}] {}", icon, crate::utils::trunc(&result.output, 200));

        if result.success { any_success = true; }
        tool_results.push(format!("툴 '{}' 결과:\n{}", tc.name, result.output));
    }

    // 모든 툴 결과를 하나의 tool 메시지로 합산
    if !tool_results.is_empty() {
        history.push(Message::tool(tool_results.join("\n\n")));
    }

    any_success
}

// ─── 채팅 루프 ───────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub async fn run_chat_loop(client: &OllamaClient) -> Result<()> {
    run_chat_loop_opts(client, false).await
}

pub async fn run_chat_loop_opts(client: &OllamaClient, resume: bool) -> Result<()> {
    use std::io::{self, BufRead, Write};

    // CLAUDE.md 자동 로딩
    let claude_md = load_claude_md();
    if !claude_md.is_empty() {
        println!("CLAUDE.md 로드됨");
    }

    let system_prompt = format!("모델: {}\n\n{}{}", client.model(), tool_descriptions(), claude_md);

    let mut history: Vec<Message> = if resume {
        let prev = load_session();
        if prev.is_empty() {
            vec![Message::system(&system_prompt)]
        } else {
            println!("이전 세션 복원 완료 (메시지 {}개)\n", prev.len());
            prev
        }
    } else {
        vec![Message::system(&system_prompt)]
    };

    let mut current_model = client.model().to_string();
    let mut stats = SessionStats::new();
    let mut plan_mode = false;
    let mut think_mode = false;  // 확장 추론 모드
    let ctx_limit_tokens = 128_000usize;  // 대부분 모델의 컨텍스트 한도
    let mut monitor_enabled = false;  // 상태 표시줄 활성화 여부

    let session_label = match std::env::var("AI_SESSION_NAME") {
        Ok(n) if !n.is_empty() => format!(" [{}]", n),
        _ => String::new(),
    };
    println!("╔══════════════════════════════════════════════╗");
    println!("║   AI Agent  ──  Ollama + {:<16}  ║", current_model);
    println!("╚══════════════════════════════════════════════╝");
    if !session_label.is_empty() {
        println!("세션: {}", session_label.trim());
    }
    println!("슬래시 명령어: /help  |  exit 종료\n");

    // 백그라운드 시스템 모니터 시작
    let (sys_stats, _monitor_handle) = crate::monitor::start_background_monitor(2000);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // 상태 표시줄 출력
        if monitor_enabled {
            let used = estimate_tokens(&history);
            let sys = sys_stats.lock().map(|g| g.clone()).unwrap_or_default();
            // Ollama 모델 상태는 비동기이므로 캐시된 정보 사용
            let model_status = crate::monitor::ModelStatus {
                model: current_model.clone(),
                running: true,
                vram_mb: None,
                context_tokens: Some(used),
            };
            crate::monitor::print_status_bar(used, ctx_limit_tokens, &sys, &model_status);
        }

        let prompt_prefix = match (plan_mode, think_mode) {
            (true, true)   => "[PLAN+THINK] You> ",
            (true, false)  => "[PLAN] You> ",
            (false, true)  => "[THINK] You> ",
            (false, false) => "You> ",
        };
        print!("{}", prompt_prefix);
        stdout.flush()?;

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) | Err(_) => { save_session(&history); break; }
            Ok(_) => {}
        }

        let input = input.trim().to_string();
        if input.is_empty() { continue; }

        // ── 슬래시 명령어 ──────────────────────────────
        if input == "exit" || input == "quit" || input == "종료" {
            save_session(&history);
            println!("세션 저장 완료. 종료합니다.");
            break;
        }

        // 접두어 기반 커맨드
        let cmd_parts: Vec<&str> = input.splitn(3, ' ').collect();
        let cmd = cmd_parts[0];
        let arg1 = cmd_parts.get(1).copied().unwrap_or("");
        let arg2 = cmd_parts.get(2).copied().unwrap_or("");

        match cmd {
            "/help" => {
                println!("\n╔════════════════════════════════════════════════════╗");
                println!("║               슬래시 명령어 목록                 ║");
                println!("╠════════════════════════════════════════════════════╣");
                println!("║ /help               이 도움말                    ║");
                println!("║ /clear              히스토리 초기화              ║");
                println!("║ /resume             저장된 세션 불러오기         ║");
                println!("║ /history [n]        히스토리 보기 (최근 n개)     ║");
                println!("║ /save               세션 저장                    ║");
                println!("║ /compact            히스토리 AI 요약 압축        ║");
                println!("║ /model <이름>       모델 변경                    ║");
                println!("║ /models             사용 가능한 모델 목록        ║");
                println!("║ /cost               세션 토큰 사용량             ║");
                println!("║ /context            컨텍스트 윈도우 사용률       ║");
                println!("║ /status             세션 상태 요약               ║");
                println!("║ /doctor             환경 진단                    ║");
                println!("║ /init               CLAUDE.md 자동 생성         ║");
                println!("║ /add <파일>         파일을 컨텍스트에 추가       ║");
                println!("║ /think              확장 추론 모드 토글          ║");
                println!("║ /plan               플랜 모드 토글               ║");
                println!("║ /export [파일명]    대화 마크다운 내보내기       ║");
                println!("║ /commit [경로]      AI 커밋 메시지 자동 생성    ║");
                println!("║ /review [경로]      AI 코드 리뷰                ║");
                println!("║ /memory save <메모> 메모 저장                   ║");
                println!("║ /memory list        저장된 메모 목록            ║");
                println!("║ /memory clear       메모 전체 삭제              ║");
                println!("║ /config [키] [값]   설정 조회/변경              ║");
                println!("║ /agile <작업>       애자일 스프린트 (PO→Dev→QA) ║");
                println!("║ /agile --fast <작업> 빠른 스프린트 (BA/UX 스킵)  ║");
                println!("║ /board [project]    애자일 보드 상태 출력         ║");
                println!("║ /retro [sprint_id]  스프린트 회고 (KPT)           ║");
                println!("║ /postmortem <설명>  장애 포스트모템 분석          ║");
                println!("║ /techdebt [path]    기술 부채 분석 보고서         ║");
                println!("║ /ba <task>          비즈니스 분석 단독 실행       ║");
                println!("║ /ux <task>          UX 설계 단독 실행             ║");
                println!("║ /devops [path]      DevOps CI/CD 설정 생성        ║");
                println!("║ /docs [path]        기술 문서 자동 생성           ║");
                println!("║ /sre [path]         SRE 모니터링 + 런북 생성      ║");
                println!("║ /security [path]    보안 감사 (HackerAgent)        ║");
                println!("║ /coordinator <작업> 병렬 멀티에이전트 Coordinator  ║");
                println!("║ /rag index [path]   코드베이스 RAG 인덱싱         ║");
                println!("║ /rag query <질문>   RAG 기반 코드 질의응답         ║");
                println!("║ /rag status         RAG 인덱스 상태                ║");
                println!("║ /pr [create|list]   GitHub PR 관리                 ║");
                println!("║ /pipeline <작업>    멀티에이전트 파이프라인       ║");
                println!("║ /nodes <작업>       노드 파이프라인               ║");
                println!("║ /ipc [port]         AI-to-AI HTTP 서버 시작       ║");
                println!("║ /skills             툴 상세 목록                 ║");
                println!("║ /skill <name> [args]  사용자 스킬 실행           ║");
                println!("║ /skill-new <name> <desc>  스킬 파일 생성         ║");
                println!("║ /mcp                MCP 서버/툴 목록              ║");
                println!("║ /mcp-call <srv> <tool> <json>  MCP 툴 호출       ║");
                println!("║ /monitor            시스템 상태 표시 토글         ║");
                println!("║ /sysinfo            현재 시스템/GPU 상태 출력     ║");
                println!("║ exit / quit         종료                         ║");
                println!("╚════════════════════════════════════════════════════╝\n");
                continue;
            }

            "/clear" => {
                history = vec![Message::system(&system_prompt)];
                stats = SessionStats::new();
                println!("히스토리 초기화 완료.\n");
                continue;
            }

            "/resume" => {
                let prev = load_session();
                if prev.is_empty() {
                    println!("저장된 세션이 없습니다.\n");
                } else {
                    history = prev;
                    println!("세션 복원 완료 (메시지 {}개)\n", history.len());
                }
                continue;
            }

            "/save" => {
                save_session(&history);
                println!("세션 저장 완료 ({})\n", SESSION_FILE);
                continue;
            }

            "/history" => {
                if arg1 == "sessions" || arg1 == "list" {
                    // 저장된 세션 목록 출력
                    let hist_mgr = crate::history::HistoryManager::new();
                    hist_mgr.print_history();
                } else {
                    // 현재 세션 히스토리 출력
                    let n: usize = arg1.parse().unwrap_or(0);
                    let skip = if n > 0 { history.len().saturating_sub(n) } else { 0 };
                    println!("── 현재 세션 히스토리 ({} 메시지) ──", history.len());
                    for (i, msg) in history.iter().skip(skip).enumerate() {
                        let preview = crate::utils::trunc_owned(&msg.content, 120, "...");
                        println!("[{}] [{:?}] {}", i, msg.role, preview);
                    }
                }
                println!("---\n");
                continue;
            }

            "/compact" => {
                let before = history.len();
                if history.len() <= MAX_HISTORY {
                    println!("압축 불필요 ({} 메시지 / 최대 {})\n", before, MAX_HISTORY);
                } else {
                    print!("AI 요약 압축 중... ");
                    stdout.flush()?;
                    let compacted = compact_with_summary(&mut history, client).await;
                    if compacted {
                        println!("완료 ({} → {} 메시지)\n", before, history.len());
                    } else {
                        println!("실패 — 단순 압축 적용\n");
                        compact_history(&mut history);
                    }
                }
                continue;
            }

            "/models" => {
                match client.list_models().await {
                    Ok(models) => {
                        println!("사용 가능한 모델 ({} 개):", models.len());
                        for m in &models {
                            let marker = if m == &current_model { " ◀ 현재" } else { "" };
                            println!("  {}{}", m, marker);
                        }
                        println!();
                    }
                    Err(e) => println!("모델 목록 조회 실패: {}\n", e),
                }
                continue;
            }

            "/model" => {
                if arg1.is_empty() {
                    println!("현재 모델: {}\n사용법: /model <모델명>\n", current_model);
                } else {
                    current_model = arg1.to_string();
                    std::env::set_var("OLLAMA_MODEL", &current_model);
                    let new_prompt = format!("모델: {}\n\n{}", current_model, tool_descriptions());
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            first.content = new_prompt;
                        }
                    }
                    println!("모델 변경됨: {}\n", current_model);
                }
                continue;
            }

            // ─── 새 커맨드들 ─────────────────────────────
            "/cost" | "/usage" => {
                let hist_tokens = estimate_tokens(&history);
                println!("\n=== 세션 토큰 사용량 (추정) ===");
                println!("  대화 턴       : {}", stats.turns);
                println!("  툴 호출       : {}", stats.tool_calls);
                println!("  프롬프트 토큰 : ~{}", stats.est_prompt_tokens);
                println!("  응답 토큰     : ~{}", stats.est_response_tokens);
                println!("  총 토큰       : ~{}", stats.total());
                println!("  컨텍스트      : {}", context_bar(hist_tokens, ctx_limit_tokens));
                println!("  경과 시간     : {}", stats.elapsed());
                println!("  (※ Ollama 스트리밍은 정확한 토큰 수를 제공하지 않아 추정값입니다)\n");
                continue;
            }

            "/context" => {
                let used = estimate_tokens(&history);
                let pct = (used * 100) / ctx_limit_tokens.max(1);
                println!("\n=== 컨텍스트 윈도우 ===");
                println!("  {}", context_bar(used, ctx_limit_tokens));
                println!("  메시지 수     : {}", history.len());
                println!("  추정 토큰     : ~{}k / {}k", used / 1000, ctx_limit_tokens / 1000);
                if pct > 80 {
                    println!("  ⚠️  컨텍스트 {}% 사용 — /compact 권장", pct);
                }
                println!();
                continue;
            }

            "/init" => {
                println!("프로젝트 분석 중...");
                // 디렉토리 구조 파악
                let tree = crate::tools::list_dir(".")
                    .map(|v| v.join("\n"))
                    .unwrap_or_else(|_| "목록 없음".to_string());
                let git_status = crate::tools::git_status(".")
                    .map(|r| r.output).unwrap_or_default();

                let init_prompt = format!(
                    "다음 프로젝트를 분석하고 CLAUDE.md 파일을 작성해주세요.\n\
                     CLAUDE.md는 AI 에이전트가 프로젝트를 이해하기 위한 문서입니다.\n\n\
                     포함할 내용:\n\
                     1. 프로젝트 개요 및 목적\n\
                     2. 주요 기술 스택\n\
                     3. 디렉토리 구조 설명\n\
                     4. 개발 규칙/컨벤션 (있는 경우)\n\
                     5. 빌드/테스트/실행 방법\n\
                     6. 주의사항\n\n\
                     ## 파일 목록\n{}\n\n## Git 상태\n{}\n\n\
                     CLAUDE.md 내용만 출력하세요 (마크다운 형식).",
                    crate::utils::trunc(&tree, 2000),
                    crate::utils::trunc(&git_status, 500),
                );

                let tmp_history = vec![
                    Message::system("당신은 프로젝트 문서화 전문가입니다."),
                    Message::user(&init_prompt),
                ];

                print!("\nCLAUDE.md 생성 중> ");
                stdout.flush()?;
                let content = client.chat_stream(tmp_history, |tok| {
                    print!("{}", tok); let _ = std::io::stdout().flush();
                }).await.unwrap_or_else(|e| format!("오류: {}", e));
                println!("\n");

                match std::fs::write("CLAUDE.md", &content) {
                    Ok(_) => {
                        println!("CLAUDE.md 생성 완료 ({} 바이트)", content.len());
                        // 방금 생성한 CLAUDE.md를 시스템 프롬프트에 반영
                        let new_claude_md = load_claude_md();
                        let new_prompt = format!("모델: {}\n\n{}{}", current_model, tool_descriptions(), new_claude_md);
                        if let Some(first) = history.first_mut() {
                            if matches!(first.role, crate::models::Role::System) {
                                first.content = new_prompt;
                            }
                        }
                        println!("시스템 프롬프트에 CLAUDE.md 반영됨\n");
                    }
                    Err(e) => println!("저장 실패: {}\n", e),
                }
                continue;
            }

            "/add" => {
                if arg1.is_empty() {
                    println!("사용법: /add <파일경로>\n");
                } else {
                    match std::fs::read_to_string(arg1) {
                        Ok(content) => {
                            let msg = format!("## 파일 컨텍스트: {}\n```\n{}\n```", arg1, content);
                            history.push(Message::tool(msg));
                            println!("파일 추가됨: {} ({} 바이트)\n", arg1, content.len());
                        }
                        Err(e) => println!("파일 읽기 실패: {}\n", e),
                    }
                }
                continue;
            }

            "/think" => {
                think_mode = !think_mode;
                if think_mode {
                    println!("확장 추론 모드 ON — 응답 전 단계별 사고 과정을 거칩니다.\n");
                    let think_addendum = "\n\n=== 확장 추론 모드 ===\n\
                        모든 요청에 대해 먼저 <think> 태그 안에 단계별 사고 과정을 작성하고,\n\
                        </think> 이후에 최종 답변/행동을 출력하세요.\n\
                        복잡한 문제는 여러 각도로 검토하고 최선의 접근법을 선택하세요.";
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) && !first.content.contains("확장 추론 모드") {
                            first.content.push_str(think_addendum);
                        }
                    }
                } else {
                    println!("확장 추론 모드 OFF\n");
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            if let Some(pos) = first.content.find("\n\n=== 확장 추론 모드 ===") {
                                first.content.truncate(pos);
                            }
                        }
                    }
                }
                continue;
            }

            "/status" => {
                let msg_count = history.iter().filter(|m| !matches!(m.role, crate::models::Role::System)).count();
                let hist_tokens = estimate_tokens(&history);
                println!("\n=== 세션 상태 ===");
                println!("  모델          : {}", current_model);
                println!("  히스토리      : {} 메시지", msg_count);
                println!("  플랜 모드     : {}", if plan_mode { "ON" } else { "OFF" });
                println!("  추론 모드     : {}", if think_mode { "ON" } else { "OFF" });
                println!("  컨텍스트      : {}", context_bar(hist_tokens, ctx_limit_tokens));
                println!("  대화 턴       : {}", stats.turns);
                println!("  툴 호출       : {}", stats.tool_calls);
                println!("  경과 시간     : {}", stats.elapsed());
                let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                println!("  작업 디렉토리 : {}", cwd);
                println!("  세션 파일     : {}", session_file());
                println!();
                continue;
            }

            "/doctor" => {
                println!("\n=== 환경 진단 ===");
                // Ollama 연결
                match client.health_check().await {
                    Ok(true) => println!("  ✅ Ollama 서버: 연결 정상"),
                    _ => println!("  ❌ Ollama 서버: 연결 실패 ({})", std::env::var("OLLAMA_API_URL").unwrap_or_default()),
                }
                // 모델
                match client.list_models().await {
                    Ok(models) => {
                        let has_model = models.iter().any(|m| m == &current_model);
                        if has_model {
                            println!("  ✅ 모델 '{}': 사용 가능", current_model);
                        } else {
                            println!("  ⚠️  모델 '{}': 없음 (ollama pull {} 필요)", current_model, current_model);
                        }
                        println!("  ℹ️  설치된 모델: {} 개", models.len());
                    }
                    Err(e) => println!("  ❌ 모델 목록: {}", e),
                }
                // 주요 툴
                for tool in &["git", "docker", "cargo", "python3", "node", "ruff", "pytest"] {
                    let ok = std::process::Command::new("which").arg(tool)
                        .output().map(|o| o.status.success()).unwrap_or(false);
                    println!("  {} {}", if ok { "✅" } else { "  " }, tool);
                }
                // 디스크
                if let Ok(r) = crate::tools::system::run_shell("df -h . | tail -1") {
                    println!("  ℹ️  디스크: {}", r.stdout.trim());
                }
                // 세션 파일
                let has_session = std::path::Path::new(SESSION_FILE).exists();
                println!("  {} 세션 파일 ({}): {}", if has_session { "✅" } else { "  " }, SESSION_FILE, if has_session { "있음" } else { "없음" });
                println!();
                continue;
            }

            "/export" => {
                let filename = if arg1.is_empty() {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs()).unwrap_or(0);
                    format!("conversation_{}.md", ts)
                } else {
                    arg1.to_string()
                };

                let mut md = format!("# AI Agent 대화 내보내기\n\n모델: `{}`  \n경과: {}\n\n---\n\n", current_model, stats.elapsed());
                for msg in &history {
                    match msg.role {
                        crate::models::Role::System => {}
                        crate::models::Role::User => md.push_str(&format!("**You:** {}\n\n", msg.content)),
                        crate::models::Role::Assistant => md.push_str(&format!("**Agent:** {}\n\n", msg.content)),
                        crate::models::Role::Tool => md.push_str(&format!("```\n{}\n```\n\n", msg.content)),
                    }
                }
                match std::fs::write(&filename, &md) {
                    Ok(_) => println!("대화 내보내기 완료: {} ({} 바이트)\n", filename, md.len()),
                    Err(e) => println!("내보내기 실패: {}\n", e),
                }
                continue;
            }

            "/commit" => {
                let repo_path = if arg1.is_empty() { "." } else { arg1 };
                println!("Git diff 분석 중...");
                let diff = crate::tools::git_diff(repo_path, false)
                    .map(|r| r.output)
                    .unwrap_or_else(|e| format!("diff 오류: {}", e));
                let status = crate::tools::git_status(repo_path)
                    .map(|r| r.output)
                    .unwrap_or_default();

                if diff.trim().is_empty() && status.trim().is_empty() {
                    println!("변경사항이 없습니다.\n");
                    continue;
                }

                let commit_prompt = format!(
                    "다음 git diff를 분석하고 Conventional Commits 형식의 커밋 메시지를 작성해주세요.\n\
                     형식: <type>(<scope>): <description>\n\
                     타입: feat/fix/docs/style/refactor/perf/test/build/ci/chore\n\
                     본문 없이 제목만 작성하세요 (1-2줄).\n\n\
                     ## git status\n{}\n\n## git diff\n{}",
                    crate::utils::trunc(&status, 500),
                    crate::utils::trunc(&diff, 3000)
                );
                let tmp_history = vec![
                    Message::system("당신은 git 커밋 메시지 작성 전문가입니다."),
                    Message::user(&commit_prompt),
                ];
                print!("\nAgent> ");
                stdout.flush()?;
                let response = client.chat_stream(tmp_history, |tok| {
                    print!("{}", tok); let _ = std::io::stdout().flush();
                }).await.unwrap_or_else(|e| format!("오류: {}", e));
                println!("\n");
                stats.add_prompt(&commit_prompt);
                stats.add_response(&response);
                continue;
            }

            "/review" => {
                let repo_path = if arg1.is_empty() { "." } else { arg1 };
                println!("코드 변경사항 분석 중...");
                let diff = crate::tools::git_diff(repo_path, false)
                    .map(|r| r.output)
                    .unwrap_or_else(|e| format!("diff 오류: {}", e));

                if diff.trim().is_empty() {
                    println!("리뷰할 변경사항이 없습니다.\n");
                    continue;
                }

                let review_prompt = format!(
                    "다음 코드 변경사항을 리뷰해주세요.\n\
                     확인할 항목:\n\
                     1. 버그 가능성\n\
                     2. 보안 취약점\n\
                     3. 성능 문제\n\
                     4. 코드 품질 및 가독성\n\
                     5. 개선 제안\n\n\
                     ## git diff\n{}",
                    crate::utils::trunc(&diff, 4000)
                );
                let tmp_history = vec![
                    Message::system("당신은 시니어 소프트웨어 엔지니어로 코드 리뷰 전문가입니다."),
                    Message::user(&review_prompt),
                ];
                print!("\nReview> ");
                stdout.flush()?;
                let response = client.chat_stream(tmp_history, |tok| {
                    print!("{}", tok); let _ = std::io::stdout().flush();
                }).await.unwrap_or_else(|e| format!("오류: {}", e));
                println!("\n");
                stats.add_prompt(&review_prompt);
                stats.add_response(&response);
                continue;
            }

            "/plan" => {
                plan_mode = !plan_mode;
                if plan_mode {
                    println!("플랜 모드 ON — 대화를 실행 전 계획 수립에 집중합니다.\n");
                    // 시스템 프롬프트에 플랜 지침 추가
                    let plan_addendum = "\n\n=== 플랜 모드 ===\n\
                        사용자 요청을 받으면 먼저 단계별 실행 계획을 작성하고,\n\
                        사용자 확인 후 실행하세요. 툴 호출 전 반드시 계획을 공유하세요.";
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            if !first.content.contains("플랜 모드") {
                                first.content.push_str(plan_addendum);
                            }
                        }
                    }
                } else {
                    println!("플랜 모드 OFF — 일반 모드로 돌아갑니다.\n");
                    // 플랜 지침 제거
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            if let Some(pos) = first.content.find("\n\n=== 플랜 모드 ===") {
                                first.content.truncate(pos);
                            }
                        }
                    }
                }
                continue;
            }

            "/memory" => {
                let subcmd = arg1;
                let mut entries = memory_load();
                match subcmd {
                    "save" | "add" | "저장" => {
                        let note = if arg2.is_empty() { arg1 } else { arg2 };
                        if note.is_empty() || note == "save" || note == "add" {
                            println!("사용법: /memory save <메모 내용>\n");
                        } else {
                            let id = entries.len() + 1;
                            let ts = chrono_now();
                            entries.push(MemoryEntry { id, note: note.to_string(), created: ts });
                            memory_save(&entries);
                            println!("메모 저장 완료 (ID: {})\n", id);
                        }
                    }
                    "list" | "ls" | "목록" | "" => {
                        if entries.is_empty() {
                            println!("저장된 메모 없음.\n");
                        } else {
                            println!("=== 메모 목록 ({} 개) ===", entries.len());
                            for e in &entries {
                                println!("[{}] ({}) {}", e.id, e.created, e.note);
                            }
                            println!();
                        }
                    }
                    "clear" | "전체삭제" => {
                        entries.clear();
                        memory_save(&entries);
                        println!("메모 전체 삭제 완료.\n");
                    }
                    "del" | "rm" | "삭제" => {
                        if let Ok(id) = arg2.parse::<usize>() {
                            let before = entries.len();
                            entries.retain(|e| e.id != id);
                            memory_save(&entries);
                            if entries.len() < before { println!("메모 #{} 삭제됨.\n", id); }
                            else { println!("메모 #{} 를 찾을 수 없습니다.\n", id); }
                        } else {
                            println!("사용법: /memory del <ID>\n");
                        }
                    }
                    _ => println!("사용법: /memory [save <메모>|list|clear|del <ID>]\n"),
                }
                continue;
            }

            "/config" | "/settings" => {
                let app_cfg = crate::config::AppConfig::load();
                match arg1 {
                    "" => {
                        println!("=== 현재 설정 ===");
                        println!("모델:        {}", app_cfg.ollama.model);
                        println!("API URL:     {}", app_cfg.ollama.api_url);
                        println!("타임아웃:    {}초", app_cfg.ollama.timeout_secs);
                        println!("최대 턴:     {}", app_cfg.agent.max_turns);
                        println!("히스토리:    {}", if app_cfg.agent.history_enabled { "활성화" } else { "비활성화" });
                        println!("컨텍스트:    최대 {}개 메시지", app_cfg.agent.history_max_context);
                        println!("프로젝트:    {}", app_cfg.agile.project);
                        println!("QA 재시도:   최대 {}회", app_cfg.agile.max_qa_retries);
                        println!("보안 라운드: 최대 {}회", app_cfg.agile.max_security_rounds);
                        println!("\n설정 파일 위치: ai-agent.toml (로컬) | ~/.config/ai-agent/config.toml (전역)");
                        println!("초기화: /config-init\n");
                    }
                    "init" | "-init" => {
                        let path = std::path::PathBuf::from("ai-agent.toml");
                        match crate::config::AppConfig::save_default(&path) {
                            Ok(()) => println!("설정 파일 생성: {:?}\n", path),
                            Err(e) => println!("설정 파일 생성 실패: {}\n", e),
                        }
                    }
                    _ => {
                        // 레거시 JSON 설정 (하위 호환)
                        let mut cfg = config_load();
                        if arg2.is_empty() {
                            match cfg.get(arg1) {
                                Some(v) => println!("{} = {}\n", arg1, v),
                                None => println!("'{}' 설정 없음.\n", arg1),
                            }
                        } else {
                            let json_val: serde_json::Value = arg2.parse()
                                .unwrap_or_else(|_| serde_json::Value::String(arg2.to_string()));
                            cfg[arg1] = json_val;
                            config_save(&cfg);
                            println!("설정 저장: {} = {}\n", arg1, arg2);
                        }
                    }
                }
                continue;
            }

            // ── 애자일 Sprint ────────────────────────────────
            "/agile" | "/sprint" => {
                let rest = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                let (fast, task) = if rest.starts_with("--fast") {
                    (true, rest.trim_start_matches("--fast").trim().to_string())
                } else {
                    (false, rest.to_string())
                };
                if task.is_empty() {
                    println!("사용법: /agile [--fast] <작업 설명>\n예: /agile 사용자 인증 시스템 구현\n      /agile --fast 로그인 기능\n");
                } else {
                    let project = std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string());
                    println!("\n🏃 애자일 스프린트 시작{}: {}", if fast { " (fast)" } else { "" }, crate::utils::trunc(&task, 60));
                    match crate::agile::run_agile_sprint_opts(client, &project, &task, fast, |msg| {
                        println!("{}", msg);
                    }).await {
                        Ok(result) => {
                            let summary = format!(
                                "스프린트 완료 — 완료: {}개, 릴리즈: {}개, 실패: {}개, 버그: {}개, 문서: {}개, 벨로시티: {}pts",
                                result.completed_stories.len(),
                                result.released_stories.len(),
                                result.failed_stories.len(),
                                result.total_bugs,
                                result.docs_generated,
                                result.velocity,
                            );
                            println!("\n{}\n", summary);
                            history.push(Message::tool(summary));
                        }
                        Err(e) => println!("스프린트 오류: {}\n", e),
                    }
                }
                continue;
            }

            // ── Sprint 회고 ──────────────────────────────────
            "/retro" | "/retrospective" => {
                let sprint_id = if arg1.is_empty() { None } else { Some(arg1) };
                let project = std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string());
                let board = crate::agile::AgileBoard::load_or_new(&project);
                println!("\n🔄 스프린트 회고 시작...");
                match crate::agile::run_retrospective(client, &board, sprint_id, |msg| {
                    println!("{}", msg);
                }).await {
                    Ok(result) => {
                        let summary = format!(
                            "회고 완료 — 팀 건강도: {}/10, 액션: {}개, 속도: {}",
                            result.team_health_score, result.action_items.len(), result.velocity_trend
                        );
                        println!("\n{}\n", summary);
                        history.push(Message::tool(summary));
                    }
                    Err(e) => println!("회고 오류: {}\n", e),
                }
                continue;
            }

            // ── 포스트모템 ─────────────────────────────────────
            "/postmortem" | "/pm" => {
                let desc = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if desc.is_empty() {
                    println!("사용법: /postmortem <장애 설명>\n예: /postmortem API 서버 다운 — 메모리 누수로 OOM 발생\n");
                } else {
                    let path = std::env::current_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| ".".to_string());
                    println!("\n🚨 포스트모템 분석 시작...");
                    match crate::agile::run_postmortem(client, desc, &path, |msg| {
                        println!("{}", msg);
                    }).await {
                        Ok(result) => {
                            let summary = format!(
                                "포스트모템 완료 — {} | 심각도: {} | 액션: {}개",
                                result.incident_id, result.severity, result.action_items.len()
                            );
                            println!("\n{}\n", summary);
                            history.push(Message::tool(summary));
                        }
                        Err(e) => println!("포스트모템 오류: {}\n", e),
                    }
                }
                continue;
            }

            // ── 기술 부채 분석 ─────────────────────────────────
            "/techdebt" | "/debt" => {
                let path = if arg1.is_empty() { "." } else { arg1 };
                println!("\n📊 기술 부채 분석 시작: {}", path);
                match crate::agile::run_techdebt_analysis(client, path, |msg| {
                    println!("{}", msg);
                }).await {
                    Ok(report) => {
                        let summary = format!(
                            "기술 부채 분석 완료 — {}개 항목, 총 {}일 추정, 부채비율: {}",
                            report.debt_items.len(), report.total_estimated_days, report.debt_ratio
                        );
                        println!("\n{}\n", summary);
                        history.push(Message::tool(summary));
                    }
                    Err(e) => println!("기술 부채 분석 오류: {}\n", e),
                }
                continue;
            }

            // ── 단독 역할 실행 ─────────────────────────────────
            "/ba" | "/biz-analyst" => {
                let task = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if task.is_empty() {
                    println!("사용법: /ba <분석할 기능 설명>\n");
                } else {
                    let hub = crate::agent::node::NodeHub::new();
                    println!("\n📊 비즈니스 분석 시작...");
                    let out = crate::agile::run_role_standalone(
                        client, crate::agile::AgileRole::BusinessAnalyst, task, "", &hub, &|msg| println!("{}", msg)
                    ).await;
                    println!("\n{}\n", out);
                    history.push(Message::tool(crate::utils::trunc(&out, 500).to_string()));
                }
                continue;
            }

            "/ux" | "/ux-design" => {
                let task = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if task.is_empty() {
                    println!("사용법: /ux <UX 설계할 기능 설명>\n");
                } else {
                    let hub = crate::agent::node::NodeHub::new();
                    println!("\n🎨 UX 설계 시작...");
                    let out = crate::agile::run_role_standalone(
                        client, crate::agile::AgileRole::UXDesigner, task, "", &hub, &|msg| println!("{}", msg)
                    ).await;
                    println!("\n{}\n", out);
                    history.push(Message::tool(crate::utils::trunc(&out, 500).to_string()));
                }
                continue;
            }

            "/devops" => {
                let path = if arg1.is_empty() { "." } else { arg1 };
                let task = format!("프로젝트 경로 {} 에 대한 CI/CD 파이프라인, Dockerfile, K8s 매니페스트를 생성하세요.", path);
                let hub = crate::agent::node::NodeHub::new();
                println!("\n🚀 DevOps 설정 생성 시작: {}", path);
                let out = crate::agile::run_role_standalone(
                    client, crate::agile::AgileRole::DevOpsEngineer, &task, "", &hub, &|msg| println!("{}", msg)
                ).await;
                println!("\n{}\n", crate::utils::trunc(&out, 600));
                history.push(Message::tool(crate::utils::trunc(&out, 400).to_string()));
                continue;
            }

            "/docs" | "/document" => {
                let path = if arg1.is_empty() { "." } else { arg1 };
                let task = format!("프로젝트 경로 {} 의 README, API 문서, 아키텍처 문서를 작성하세요.", path);
                let hub = crate::agent::node::NodeHub::new();
                println!("\n📝 기술 문서 생성 시작: {}", path);
                let out = crate::agile::run_role_standalone(
                    client, crate::agile::AgileRole::TechnicalWriter, &task, "", &hub, &|msg| println!("{}", msg)
                ).await;
                println!("\n{}\n", crate::utils::trunc(&out, 600));
                history.push(Message::tool(crate::utils::trunc(&out, 400).to_string()));
                continue;
            }

            "/sre" => {
                let path = if arg1.is_empty() { "." } else { arg1 };
                let task = format!("프로젝트 경로 {} 에 대한 SLO, Prometheus 알람, Grafana 대시보드, 런북을 작성하세요.", path);
                let hub = crate::agent::node::NodeHub::new();
                println!("\n📡 SRE 설정 생성 시작: {}", path);
                let out = crate::agile::run_role_standalone(
                    client, crate::agile::AgileRole::SRE, &task, "", &hub, &|msg| println!("{}", msg)
                ).await;
                println!("\n{}\n", crate::utils::trunc(&out, 600));
                history.push(Message::tool(crate::utils::trunc(&out, 400).to_string()));
                continue;
            }

            // ── Coordinator 병렬 멀티에이전트 ──────────────────────────
            "/coordinator" | "/coord" => {
                let task = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if task.is_empty() {
                    println!("사용법: /coordinator <복합 태스크 설명>\n예: /coordinator 백엔드 API + 프론트엔드 UI + 테스트 동시 구현\n");
                } else {
                    println!("\n🤝 Coordinator 시작: {}", crate::utils::trunc(task, 60));
                    match crate::agile::run_coordinator(client, task, |msg| println!("{}", msg)).await {
                        Ok(result) => {
                            let summary = format!(
                                "Coordinator 완료 — {}개 워커 병렬, 서브태스크 {}개",
                                result.total_workers, result.subtasks.len()
                            );
                            println!("\n{}\n", summary);
                            history.push(Message::tool(crate::utils::trunc(&result.synthesis, 600).to_string()));
                        }
                        Err(e) => println!("Coordinator 오류: {}\n", e),
                    }
                }
                continue;
            }

            // ── RAG 코드베이스 인덱싱/검색 ───────────────────────────
            "/rag" => {
                match arg1 {
                    "index" => {
                        let path = if arg2.is_empty() { "." } else { arg2 };
                        println!("📚 코드베이스 인덱싱 중: {} ...", path);
                        match crate::agent::rag::index_codebase(path) {
                            Ok(index) => {
                                let status = index.status();
                                crate::agent::rag::save_index(&index).ok();
                                println!("✅ 인덱싱 완료\n{}\n", status);
                            }
                            Err(e) => println!("인덱싱 오류: {}\n", e),
                        }
                    }
                    "query" | "q" => {
                        let query = input.splitn(4, ' ').nth(2).unwrap_or("").trim().to_string();
                        if query.is_empty() {
                            println!("사용법: /rag query <질문>\n");
                        } else {
                            match crate::agent::rag::load_index() {
                                Some(index) => {
                                    let chunks = crate::agent::rag::search(&index, &query);
                                    if chunks.is_empty() {
                                        println!("관련 코드를 찾지 못했습니다. /rag index 를 먼저 실행하세요.\n");
                                    } else {
                                        let ctx = crate::agent::rag::build_context(&chunks);
                                        println!("🔍 {}개 청크 검색됨 — AI에 질의 중...", chunks.len());
                                        let mut rag_history = history.clone();
                                        rag_history.push(Message::tool(ctx));
                                        rag_history.push(Message::user(&query));
                                        print!("Agent> ");
                                        use std::io::Write;
                                        std::io::stdout().flush().ok();
                                        match client.chat_stream(rag_history, |tok| {
                                            print!("{}", tok);
                                            std::io::stdout().flush().ok();
                                        }).await {
                                            Ok(resp) => {
                                                println!();
                                                history.push(Message::user(&query));
                                                history.push(Message::assistant(&resp));
                                            }
                                            Err(e) => println!("\n오류: {}\n", e),
                                        }
                                    }
                                }
                                None => println!("RAG 인덱스 없음. /rag index 를 먼저 실행하세요.\n"),
                            }
                        }
                    }
                    "status" | "st" | "" => {
                        match crate::agent::rag::load_index() {
                            Some(index) => println!("\n📊 RAG 인덱스 상태\n{}\n", index.status()),
                            None => println!("RAG 인덱스 없음. /rag index [경로] 로 인덱싱하세요.\n"),
                        }
                    }
                    _ => println!("사용법: /rag [index|query|status]\n"),
                }
                continue;
            }

            // ── GitHub PR 관리 ────────────────────────────────────────
            "/pr" | "/pull-request" => {
                match arg1 {
                    "list" | "ls" | "" => {
                        let state = if arg2.is_empty() { "open" } else { arg2 };
                        match crate::agent::github::list_prs(state) {
                            Ok(output) => println!("\n📋 PR 목록 ({})\n{}\n", state, output),
                            Err(e) => println!("PR 목록 오류: {}\n", e),
                        }
                    }
                    "create" => {
                        let branch = crate::agent::github::current_branch();
                        let title = if arg2.is_empty() {
                            format!("feat: {} 브랜치 변경사항", branch)
                        } else { arg2.to_string() };

                        // AI로 PR 본문 생성
                        println!("📝 AI PR 본문 생성 중...");
                        let git_log = std::process::Command::new("git")
                            .args(["log", "--oneline", "-10"])
                            .output()
                            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                            .unwrap_or_default();

                        let body_prompt = format!(
                            "다음 git 로그를 기반으로 GitHub PR 본문을 마크다운으로 작성하세요.\n\n{}", git_log
                        );
                        let body_msgs = vec![
                            Message::system("당신은 GitHub PR 작성 전문가입니다."),
                            Message::user(&body_prompt),
                        ];
                        let pr_body = match client.chat_stream(body_msgs, |_| {}).await {
                            Ok(t) => t,
                            Err(_) => "AI 생성 PR 본문".to_string(),
                        };

                        let opts = crate::agent::github::PrOptions::new(&title, &pr_body);
                        match crate::agent::github::create_pr(&opts) {
                            Ok(result) => println!("✅ PR 생성: {}\n", result.url),
                            Err(e) => println!("PR 생성 오류: {}\n", e),
                        }
                    }
                    _ => println!("사용법: /pr [list|create]\n"),
                }
                continue;
            }

            // 보드 상태 조회
            "/board" => {
                let project = if arg1.is_empty() {
                    std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string())
                } else { arg1.to_string() };
                let board = crate::agile::AgileBoard::load_or_new(&project);
                println!("{}", board.render());
                println!("{}", board.render_burndown());
                continue;
            }

            // 보안 감사 (HackerAgent 단독 실행)
            "/security" | "/hack" | "/audit" => {
                let project_path = if arg1.is_empty() { "." } else { arg1 };
                let project = std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string());
                let board = crate::agile::AgileBoard::load_or_new(&project);
                let hub = crate::agent::node::NodeHub::new();

                // 현재 보드에서 진행 중인 스토리를 찾거나 Create temporary story
                let story_id = {
                    let state = board.shared_state();
                    let s = state.lock().unwrap();
                    s.stories.values()
                        .find(|st| matches!(st.status,
                            crate::agile::story::StoryStatus::Done |
                            crate::agile::story::StoryStatus::SecurityReview))
                        .map(|st| st.id.clone())
                        .or_else(|| s.stories.values().next().map(|st| st.id.clone()))
                };

                let sid = if let Some(id) = story_id {
                    id
                } else {
                    // 스토리가 없으면 수동 감사용 Create temporary story
                    let new_sid = board.next_story_id();
                    let mut tmp = crate::agile::story::UserStory::new(
                        &new_sid, "수동 보안 감사", "사용자 요청 수동 보안 감사",
                        crate::agile::story::Priority::High, 3,
                    );
                    tmp.implementation = Some(format!("프로젝트 경로: {}", project_path));
                    let _ = board.add_story(tmp);
                    new_sid
                };

                println!("\n🔒 보안 감사 시작 — 경로: {}", project_path);
                let sec = crate::agile::hacker::run_security_fix_loop(
                    &client, &board, &hub, &sid, project_path,
                    |msg| println!("{}", msg),
                ).await;
                println!("{}", sec.final_report.render());
                println!(
                    "\n결과: {} | 라운드 {} | 취약점 총 {}개 | 미수정 {}개",
                    if sec.approved { "✅ 통과" } else { "⚠️ 미수정 존재" },
                    sec.rounds,
                    sec.final_report.vulnerabilities.len(),
                    sec.final_report.unfixed_count(),
                );
                continue;
            }

            // IPC 서버 시작 (대화 중에도 가능)
            "/ipc" | "/ipc-server" => {
                let port: u16 = arg1.parse().unwrap_or(8765);
                println!("IPC HTTP 서버 시작 (포트 {})...", port);
                println!("다른 AI가 POST http://localhost:{} 로 JSON-RPC 요청 가능", port);
                println!("예: curl -X POST http://localhost:{} -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"params\":{{}}}}'", port);
                let new_client = crate::agent::OllamaClient::from_env();
                let server = crate::ipc::AgentServer::new(new_client);
                tokio::spawn(async move {
                    if let Err(e) = server.run_http_server(port).await {
                        eprintln!("[IPC] 서버 오류: {}", e);
                    }
                });
                println!("IPC 서버가 백그라운드에서 실행 중입니다.\n");
                continue;
            }

            // 멀티에이전트 Pipeline
            "/pipeline" | "/pipe" => {
                let task = if arg2.is_empty() {
                    arg1.to_string()
                } else {
                    format!("{} {}", arg1, arg2)
                };
                if task.is_empty() {
                    println!("사용법: /pipeline <작업 설명>\n");
                } else {
                    match crate::agent::orchestrator::run_pipeline(client, &task).await {
                        Ok(result) => {
                            println!("\n📋 계획:\n{}", crate::utils::trunc(&result.plan, 500));
                            println!("\n💻 구현:\n{}", crate::utils::trunc(&result.implementation, 500));
                            println!("\n🔍 검증:\n{}", crate::utils::trunc(&result.verification, 400));
                            if let Some(ref r) = result.review {
                                println!("\n👁️ 리뷰:\n{}", crate::utils::trunc(r, 400));
                            }
                            println!();
                            history.push(Message::tool(format!(
                                "파이프라인 완료:\n계획: {}\n구현: {}\n검증: {}",
                                crate::utils::trunc(&result.plan, 300),
                                crate::utils::trunc(&result.implementation, 300),
                                crate::utils::trunc(&result.verification, 200),
                            )));
                        }
                        Err(e) => println!("파이프라인 오류: {}\n", e),
                    }
                }
                continue;
            }

            // 영향 분석
            "/impact" => {
                if arg1.is_empty() {
                    println!("사용법: /impact <파일경로> [변경내용]\n");
                } else {
                    print!("영향 분석 중... ");
                    stdout.flush()?;
                    match crate::agent::react::analyze_impact(client, arg1, arg2).await {
                        Ok(analysis) => println!("\n{}\n", analysis),
                        Err(e) => println!("분석 실패: {}\n", e),
                    }
                }
                continue;
            }

            // /monitor — 실시간 상태 표시줄 토글
            "/monitor" | "/mon" => {
                monitor_enabled = !monitor_enabled;
                if monitor_enabled {
                    println!("시스템 모니터 ON — 각 프롬프트 위에 상태 표시\n");
                } else {
                    println!("시스템 모니터 OFF\n");
                }
                continue;
            }

            // /sysinfo — 현재 시스템 상태 일회성 출력
            "/sysinfo" | "/sys" => {
                print!("시스템 정보 수집 중... ");
                stdout.flush()?;
                let sys = crate::monitor::SystemStats::collect();
                println!("\n");
                println!("=== 시스템 상태 ===");
                println!("  CPU 사용률  : {:.1}%", sys.cpu_pct);
                println!("  메모리      : {} / {} MB ({:.0}%)",
                    sys.mem_used_mb, sys.mem_total_mb,
                    sys.mem_used_mb as f32 * 100.0 / sys.mem_total_mb.max(1) as f32);
                if let Some(pct) = sys.gpu_pct {
                    println!("  GPU 이름    : {}", sys.gpu_name.as_deref().unwrap_or("Unknown"));
                    println!("  GPU 사용률  : {:.1}%", pct);
                    if let (Some(used), Some(total)) = (sys.gpu_mem_used_mb, sys.gpu_mem_total_mb) {
                        println!("  VRAM        : {} / {} MB ({:.0}%)", used, total,
                            used as f32 * 100.0 / total.max(1) as f32);
                    }
                } else {
                    println!("  GPU         : nvidia-smi / rocm-smi 없음");
                }

                // Ollama 모델 상태
                print!("\n  Ollama 상태 : ");
                stdout.flush()?;
                let model_status = crate::monitor::get_model_status(&current_model).await;
                if model_status.running {
                    println!("실행 중 ({}{})", current_model,
                        model_status.vram_mb.map(|m| format!(" {:.1}GB VRAM", m as f64 / 1024.0)).unwrap_or_default());
                } else {
                    println!("유휴 (모델 미로딩)");
                }

                // 컨텍스트
                let used = estimate_tokens(&history);
                let pct = used * 100 / ctx_limit_tokens.max(1);
                println!("  컨텍스트    : ~{}k / {}k tokens ({:.0}%)",
                    used / 1000, ctx_limit_tokens / 1000, pct);
                println!("  세션 통계   : {}턴, {}툴호출, {}초경과",
                    stats.turns, stats.tool_calls, stats.start.elapsed().as_secs());
                println!();
                continue;
            }

            "/skills" | "/skill-list" => {
                println!("\n{}\n", tool_descriptions());
                // 로드된 사용자 스킬 목록
                let mut skill_reg = crate::skills::SkillRegistry::new();
                skill_reg.load_all();
                if !skill_reg.is_empty() {
                    println!("=== 사용자 스킬 ({} 개) ===", skill_reg.len());
                    for s in skill_reg.all() {
                        println!("  /{} — {}", s.name, s.description);
                    }
                    println!();
                }
                continue;
            }

            // /skill <name> [args...] — 스킬 실행
            "/skill" => {
                if arg1.is_empty() {
                    println!("사용법: /skill <이름> [인자...]\n");
                } else {
                    let mut skill_reg = crate::skills::SkillRegistry::new();
                    skill_reg.load_all();
                    // 나머지 인자들 수집
                    let extra_args: Vec<&str> = input.splitn(3, ' ').skip(2).collect();
                    let args_refs: Vec<&str> = std::iter::once(arg2)
                        .chain(extra_args.iter().map(|s| *s))
                        .filter(|s| !s.is_empty())
                        .collect();
                    print!("스킬 '{}' 실행 중> ", arg1);
                    stdout.flush()?;
                    match crate::skills::execute_skill(&skill_reg, client, arg1, &args_refs, |tok| {
                        print!("{}", tok);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }).await {
                        Ok(result) => {
                            println!("\n");
                            history.push(Message::tool(format!("스킬 '{}' 결과:\n{}", arg1, result)));
                        }
                        Err(e) => println!("\n스킬 오류: {}\n", e),
                    }
                }
                continue;
            }

            // /skill-new <name> <description> — 새 스킬 생성
            "/skill-new" => {
                if arg1.is_empty() || arg2.is_empty() {
                    println!("사용법: /skill-new <이름> <설명>\n");
                } else {
                    let template = format!(
                        "다음 요청을 처리해주세요:\n{{{{args}}}}\n\n작업: {}",
                        arg1
                    );
                    match crate::skills::loader::SkillRegistry::create_skill_file(arg1, arg2, &[], &template) {
                        Ok(path) => println!("스킬 생성: {}\n편집 후 /skill {} 로 실행하세요.\n", path, arg1),
                        Err(e) => println!("스킬 생성 실패: {}\n", e),
                    }
                }
                continue;
            }

            // /mcp — MCP 서버 및 툴 목록
            "/mcp" => {
                let mut reg = crate::mcp::McpRegistry::from_config();
                if reg.server_count() == 0 {
                    println!("MCP 서버 없음. ~/.claude/mcp_servers.json 또는 ./.mcp_servers.json 에 설정 필요.\n");
                    println!("예시 형식:");
                    println!(r#"[{{"name":"filesystem","type":"stdio","command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","/"]}}]"#);
                    println!();
                } else {
                    println!("MCP 서버 ({} 개): {}", reg.server_count(), reg.server_names().join(", "));
                    print!("툴 목록 로드 중... ");
                    stdout.flush()?;
                    let count = reg.discover_tools().await;
                    println!("완료 ({} 개 툴)\n", count);
                    for tool in reg.tools() {
                        println!("  [{}] {} — {}", tool.server, tool.name, tool.description);
                    }
                    println!();
                }
                continue;
            }

            // /mcp-call <server> <tool> <json_args> — MCP 툴 직접 호출
            "/mcp-call" => {
                // arg1 = server, arg2 = tool, rest = json args
                let parts: Vec<&str> = input.splitn(5, ' ').collect();
                let server = parts.get(2).copied().unwrap_or("");
                let tool = parts.get(3).copied().unwrap_or("");
                let json_str = parts.get(4).copied().unwrap_or("{}");
                if server.is_empty() || tool.is_empty() {
                    println!("사용법: /mcp-call <server> <tool> <json_args>\n");
                } else {
                    let mut reg = crate::mcp::McpRegistry::from_config();
                    reg.discover_tools().await;
                    let args: serde_json::Value = serde_json::from_str(json_str)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    print!("MCP 호출 [{}/{}]... ", server, tool);
                    stdout.flush()?;
                    match reg.call_tool(tool, args).await {
                        Ok(result) => {
                            println!("\n{}\n", result.output);
                            history.push(Message::tool(format!("MCP [{}/{}] 결과:\n{}", server, tool, result.output)));
                        }
                        Err(e) => println!("\nMCP 오류: {}\n", e),
                    }
                }
                continue;
            }

            // /nodes — 노드 Pipeline 실행
            "/nodes" | "/node-pipeline" => {
                let task = if arg2.is_empty() { arg1.to_string() } else { format!("{} {}", arg1, arg2) };
                if task.is_empty() {
                    println!("사용법: /nodes <작업 설명>\n노드 파이프라인으로 Planner→Developer→Debugger 순으로 실행합니다.\n");
                } else {
                    println!("노드 파이프라인 시작: {}", crate::utils::trunc(&task, 80));
                    let hub = crate::agent::node::NodeHub::new();
                    match crate::agent::node::run_node_pipeline(&hub, client, &task, |msg| {
                        println!("  {}", msg);
                    }).await {
                        Ok(result) => {
                            println!("\n완료:\n{}\n", crate::utils::trunc(&result, 600));
                            history.push(Message::tool(format!("노드 파이프라인 결과:\n{}", result)));
                        }
                        Err(e) => println!("파이프라인 오류: {}\n", e),
                    }
                }
                continue;
            }

            _ if cmd.starts_with('/') => {
                // 사용자 정의 스킬인지 확인
                let skill_name = &cmd[1..];
                let mut skill_reg = crate::skills::SkillRegistry::new();
                skill_reg.load_all();
                if skill_reg.get(skill_name).is_some() {
                    let extra: Vec<&str> = input.splitn(3, ' ').skip(1).collect();
                    let args_refs: Vec<&str> = extra.iter().map(|s| *s).collect();
                    print!("스킬 '{}' 실행 중> ", skill_name);
                    stdout.flush()?;
                    match crate::skills::execute_skill(&skill_reg, client, skill_name, &args_refs, |tok| {
                        print!("{}", tok);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }).await {
                        Ok(result) => {
                            println!("\n");
                            history.push(Message::tool(format!("스킬 '{}' 결과:\n{}", skill_name, result)));
                        }
                        Err(e) => println!("\n스킬 오류: {}\n", e),
                    }
                } else {
                    println!("알 수 없는 명령어: '{}'. /help 참고\n", cmd);
                }
                continue;
            }

            _ => {}
        }

        stats.add_prompt(&input);
        history.push(Message::user(&input));

        // 컨텍스트 80% 초과 시 자동 AI 요약 압축
        let used_tokens = estimate_tokens(&history);
        if used_tokens > ctx_limit_tokens * 80 / 100 {
            print!("[컨텍스트 {}% — AI 자동 압축 중...]", used_tokens * 100 / ctx_limit_tokens);
            stdout.flush()?;
            compact_with_summary(&mut history, client).await;
            println!(" 완료\n");
        } else {
            compact_history(&mut history);
        }

        // ── 최대 20턴 툴 호출 루프 ───────────────────
        for turn in 0..20 {
            debug!("AI 응답 요청 (turn={})", turn);

            print!("\nAgent> ");
            stdout.flush()?;

            let ai_text = if current_model != client.model() {
                let alt = OllamaClient::new(
                    std::env::var("OLLAMA_API_URL")
                        .unwrap_or_else(|_| "http://localhost:11434".to_string()),
                    &current_model,
                );
                alt.chat_stream(history.clone(), |token| {
                    print!("{}", token);
                    let _ = std::io::stdout().flush();
                }).await?
            } else {
                client.chat_stream(history.clone(), |token| {
                    print!("{}", token);
                    let _ = std::io::stdout().flush();
                }).await?
            };

            println!();

            match parse_response(&ai_text) {
                AgentResponse::Exit => {
                    save_session(&history);
                    println!("\n에이전트 종료 요청.");
                    return Ok(());
                }

                AgentResponse::Text(_) => {
                    println!();
                    stats.add_response(&ai_text);
                    history.push(Message::assistant(&ai_text));
                    break;
                }

                AgentResponse::ToolCall(tool_call) if tool_call.name == "__multi__" => {
                    // 멀티툴 실행
                    println!("\n[멀티툴 {} 개 실행]", tool_call.args.len());
                    stats.add_response(&ai_text);
                    stats.tool_calls += tool_call.args.len();
                    let _any_ok = execute_multi_tool(&tool_call.args, &mut history, &ai_text).await;
                    println!();

                    if turn >= 4 {
                        history.push(Message::tool(
                            "[경고] 반복 실패. 다른 접근 방법을 시도하세요.".to_string()
                        ));
                        break;
                    }
                }

                AgentResponse::ToolCall(tool_call) => {
                    stats.add_response(&ai_text);
                    stats.add_tool();
                    let args_preview: Vec<String> = tool_call.args.iter()
                        .map(|a| {
                            let s = a.replace('\n', "↵");
                            if s.len() > 80 { format!("{}...", crate::utils::trunc(&s, 80)) } else { s }
                        })
                        .collect();

                    println!("\n┌─[툴] {} {}", tool_call.name, args_preview.join(" "));

                    let result = dispatch_tool(&tool_call).await;
                    let icon = if result.success { "✓" } else { "✗" };
                    println!("└─[{}] {}\n", icon, result.output);

                    history.push(Message::assistant(&ai_text));
                    history.push(Message::tool(format!(
                        "툴 '{}' 결과:\n{}", tool_call.name, result.output
                    )));

                    if !result.success && turn >= 4 {
                        history.push(Message::tool(
                            "[경고] 같은 툴이 반복 실패 중입니다. 다른 접근 방법을 시도하세요.".to_string()
                        ));
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AgentResponse;

    // ── parse_response_pub tests ──────────────────────────────────────────────

    #[test]
    fn parse_exit() {
        let r = parse_response_pub("EXIT");
        assert!(matches!(r, AgentResponse::Exit));
    }

    #[test]
    fn parse_plain_text() {
        let r = parse_response_pub("안녕하세요");
        assert!(matches!(r, AgentResponse::Text(_)));
    }

    #[test]
    fn parse_single_tool_read_file() {
        let r = parse_response_pub("TOOL: read_file /tmp/test.txt");
        match r {
            AgentResponse::ToolCall(tc) => {
                assert_eq!(tc.name, "read_file");
                assert_eq!(tc.args[0], "/tmp/test.txt");
            }
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn parse_single_tool_run_shell() {
        let r = parse_response_pub("TOOL: run_shell \"ls -la\"");
        match r {
            AgentResponse::ToolCall(tc) => {
                assert_eq!(tc.name, "run_shell");
            }
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn parse_multi_tool_becomes_multi_marker() {
        let input = "TOOL: read_file /tmp/a.txt\nTOOL: run_shell ls";
        let r = parse_response_pub(input);
        match r {
            AgentResponse::ToolCall(tc) => {
                assert_eq!(tc.name, "__multi__");
                assert_eq!(tc.args.len(), 2);
            }
            _ => panic!("expected __multi__ ToolCall"),
        }
    }

    #[test]
    fn parse_text_without_tool_colon() {
        let r = parse_response_pub("Here is some text about TOOL usage in general.");
        // Contains "TOOL" but not "TOOL:" so should be Text
        assert!(matches!(r, AgentResponse::Text(_)));
    }

    #[test]
    fn parse_write_file_tool() {
        let input = "TOOL: write_file /tmp/out.txt\nhello world";
        let r = parse_response_pub(input);
        match r {
            AgentResponse::ToolCall(tc) => {
                assert_eq!(tc.name, "write_file");
                assert_eq!(tc.args[0], "/tmp/out.txt");
                assert!(tc.args[1].contains("hello"));
            }
            _ => panic!("expected ToolCall"),
        }
    }

    // ── load_claude_md tests ──────────────────────────────────────────────────

    #[test]
    fn load_claude_md_returns_string() {
        // Just test it doesn't panic and returns a valid String.
        let result = load_claude_md();
        let _ = result.len();
    }
}
