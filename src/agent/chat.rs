use anyhow::Result;
use colored::*;
use tracing::debug;

use crate::agent::{
    ollama::OllamaClient,
    tools::{dispatch_tool, tool_descriptions},
};
use crate::models::{AgentResponse, Message, ToolCall};

// ─── Strip code fence ──────────────────────────────────────────────────────────

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

// ─── Single TOOL block parsing ─────────────────────────────────────────────────────

fn parse_single_tool(tool_text: &str) -> Option<ToolCall> {
    let rest = tool_text.trim();
    if rest.is_empty() {
        return None;
    }

    let (tool_name, after_name) = match rest.split_once(|c: char| c.is_whitespace()) {
        Some((name, r)) => (name.trim().to_string(), r.trim_start()),
        None => (rest.trim().to_string(), ""),
    };

    // Code execution tool: first line is language, rest is code
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

    // edit_file: path + <<<OLD>>>...<<<NEW>>>...<<<END>>>
    if tool_name == "edit_file" {
        let path = after_name.lines().next().unwrap_or("").trim().to_string();
        let body = after_name.splitn(2, '\n').nth(1).unwrap_or("");
        let (old_str, new_str) = parse_edit_delimiters(body);
        return Some(ToolCall { name: tool_name, args: vec![path, old_str, new_str] });
    }

    // todo_write: JSON multiline
    if tool_name == "todo_write" {
        let json = after_name.trim_start_matches('\n').trim().to_string();
        return Some(ToolCall { name: tool_name, args: vec![json] });
    }

    // write_file: path + content
    if tool_name == "write_file" {
        let mut lines = after_name.splitn(2, '\n');
        let first = lines.next().unwrap_or("").trim();
        let rest_content = lines.next().unwrap_or("");
        // Path is the first token without quotes
        let (path, content) = if !rest_content.is_empty() {
            (first.to_string(), rest_content.to_string())
        } else {
            // Single-line format: write_file path "content"
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

    // Rest uses shlex parsing
    let parts = shlex::split(rest).unwrap_or_else(|| {
        rest.split_whitespace().map(|s| s.to_string()).collect()
    });
    if let Some((name, args)) = parts.split_first() {
        return Some(ToolCall { name: name.clone(), args: args.to_vec() });
    }

    None
}

// ─── Multi-TOOL response parsing ─────────────────────────────────────────────────────
//
// AI may output multiple TOOL: blocks in one response.
// Split by each "TOOL:" prefix and execute all.

pub fn parse_response_pub(text: &str) -> AgentResponse { parse_response(text) }

fn parse_response(text: &str) -> AgentResponse {
    let trimmed = text.trim();

    if trimmed == "EXIT" {
        return AgentResponse::Exit;
    }

    // If at least one TOOL: prefix found, treat as tool call
    if !trimmed.contains("TOOL:") {
        return AgentResponse::Text(text.to_string());
    }

    // Text before the first TOOL: is ignored or treated as prefix description
    // Split multiple TOOL: blocks
    let tool_blocks = split_tool_blocks(trimmed);
    if tool_blocks.is_empty() {
        return AgentResponse::Text(text.to_string());
    }

    // Return first tool (rest handled in MultiTool)
    if tool_blocks.len() == 1 {
        if let Some(tc) = parse_single_tool(tool_blocks[0].trim()) {
            return AgentResponse::ToolCall(tc);
        }
    }

    // Multiple tools: MultiTool variant (uses "__multi__" marker in ToolCall name)
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

    // Serialize multiple tools into a single ToolCall
    // args[0] = "__multi__", args[1..] = JSON-serialized tools
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

/// Split text by "TOOL:" prefix, extract content after each "TOOL:"
fn split_tool_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut rest = text;

    loop {
        match rest.find("TOOL:") {
            None => break,
            Some(pos) => {
                let after = &rest[pos + 5..]; // "TOOL:".len() == 5
                // Find next TOOL: position
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

// ─── History auto-compaction ───────────────────────────────────────────────────────

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
        "[Prior conversation omitted due to context limit.]".to_string(),
    ));
    history.extend(kept);
}

// ─── CLAUDE.md auto-loading ─────────────────────────────────────────────────────

/// Automatically reads global (~/.claude/CLAUDE.md) and project CLAUDE.md files and returns them
pub fn load_claude_md() -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1) Global config: ~/.claude/CLAUDE.md
    if let Ok(home) = std::env::var("HOME") {
        let global = std::path::PathBuf::from(&home).join(".claude").join("CLAUDE.md");
        if let Ok(content) = std::fs::read_to_string(&global) {
            if !content.trim().is_empty() {
                parts.push(format!("## Global config (~/.claude/CLAUDE.md)\n{}", content.trim()));
            }
        }
    }

    // 2) Project config: search CLAUDE.md from cwd up to git root
    let mut dir = std::env::current_dir().ok();
    let mut visited = std::collections::HashSet::new();
    while let Some(d) = dir {
        let key = d.to_string_lossy().to_string();
        if visited.contains(&key) { break; }
        visited.insert(key);

        let claude_md = d.join("CLAUDE.md");
        if let Ok(content) = std::fs::read_to_string(&claude_md) {
            if !content.trim().is_empty() {
                parts.push(format!("## Project config ({}/CLAUDE.md)\n{}", d.display(), content.trim()));
            }
        }

        // Stop at git root
        if d.join(".git").exists() { break; }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("\n\n=== CLAUDE.md ===\n{}", parts.join("\n\n"))
    }
}

// ─── Context window estimation ────────────────────────────────────────────────────

/// Estimated token count for entire history (chars / 4)
fn estimate_tokens(history: &[Message]) -> usize {
    history.iter().map(|m| m.content.len() / 4).sum()
}

/// Context usage bar (assumption: model context 128k tokens)
fn context_bar(used: usize, total: usize) -> String {
    let pct = (used * 100) / total.max(1);
    let filled = pct / 5;  // 20-cell bar
    let bar: String = (0..20).map(|i| if i < filled { '█' } else { '░' }).collect();
    format!("[{}] {}% ({}/{}k)", bar, pct, used / 1000, total / 1000)
}

// ─── Session save/load ───────────────────────────────────────────────────────

const SESSION_FILE: &str = ".ai_session.json";
const MEMORY_FILE: &str = ".ai_memory.json";
const CONFIG_FILE: &str = ".ai_config.json";

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mins = (secs % 3600) / 60;
    let hours = (secs % 86400) / 3600;
    let days = secs / 86400;
    // Simple date: epoch + days (not precise but sufficient for identification)
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
            eprintln!("[WARNING] Failed to save session: {}", path);
        }
    }
}

fn load_session() -> Vec<Message> {
    std::fs::read_to_string(session_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ─── Memory management ─────────────────────────────────────────────────────────────

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
            eprintln!("[WARNING] Failed to save memory: {}", MEMORY_FILE);
        }
    }
}

// ─── Configuration management ───────────────────────────────────────────────────────────────

fn config_load() -> serde_json::Value {
    std::fs::read_to_string(CONFIG_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn config_save(cfg: &serde_json::Value) {
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        if std::fs::write(CONFIG_FILE, json).is_err() {
            eprintln!("[WARNING] Failed to save config: {}", CONFIG_FILE);
        }
    }
}

// ─── Session statistics ───────────────────────────────────────────────────────────────

struct SessionStats {
    turns: usize,
    tool_calls: usize,
    est_prompt_tokens: usize,    // estimated (chars / 4)
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
        if s < 60 { format!("{}s", s) }
        else if s < 3600 { format!("{}m {}s", s/60, s%60) }
        else { format!("{}h {}m", s/3600, (s%3600)/60) }
    }
}

// ─── AI-based history summarization compaction ──────────────────────────────────────────────

/// Compress old messages by AI summarization (preserves summary instead of deletion)
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
        "Summarize the following conversation keeping only key information.\n\
         Include code changes, decisions, and important context.\n\
         Keep the summary under 300 characters.\n\n{}",
        conversation_text
    );

    let summary_msgs = vec![
        Message::system("You are an expert conversation summarizer."),
        Message::user(&summary_prompt),
    ];

    let summary = client.chat(summary_msgs).await
        .map(|r| r.message.content)
        .unwrap_or_else(|_| format!("[Failed to summarize {} previous messages — content omitted]", to_summarize.len()));

    *history = system_msgs;
    history.push(Message::tool(format!("[Conversation summary] {}", summary)));
    history.extend(keep);
    true
}

// ─── Headless (non-interactive) single execution ────────────────────────────────────────────

/// --print mode: run a single prompt, output result to stdout, then exit
pub async fn run_print_mode(client: &OllamaClient, prompt: &str) -> Result<()> {
    use std::io::Write;

    let claude_md = load_claude_md();
    let system_prompt = format!("Model: {}\n\n{}{}", client.model(), tool_descriptions(), claude_md);

    let mut history = vec![
        Message::system(&system_prompt),
        Message::user(prompt),
    ];

    // Max 10 tool call loop
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
                    results.push(format!("Tool '{}' result:\n{}", name, result.output));
                }
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(results.join("\n\n")));
            }
            crate::models::AgentResponse::ToolCall(tc) => {
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("Tool '{}' result:\n{}", tc.name, result.output)));
            }
        }
    }

    Ok(())
}

// ─── Multi-tool execution helpers ────────────────────────────────────────────────────────

async fn execute_multi_tool(
    serialized_calls: &[String],
    history: &mut Vec<Message>,
    ai_text: &str,
) -> bool {
    // __multi__ marker: execute serialized JSON ToolCall list
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
        crate::ui::print_tool_start(&tc.name, &args_preview.join(" "));

        let result = dispatch_tool(&tc).await;
        crate::ui::print_tool_result(result.success, &crate::utils::trunc(&result.output, 200).to_string());

        if result.success { any_success = true; }
        tool_results.push(format!("Tool '{}' result:\n{}", tc.name, result.output));
    }

    // Combine all tool results into one tool message
    if !tool_results.is_empty() {
        history.push(Message::tool(tool_results.join("\n\n")));
    }

    any_success
}

// ─── Chat loop ───────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub async fn run_chat_loop(client: &OllamaClient) -> Result<()> {
    run_chat_loop_opts(client, false).await
}

pub async fn run_chat_loop_opts(client: &OllamaClient, resume: bool) -> Result<()> {
    use std::io::{self, BufRead, Write};

    // Auto-load CLAUDE.md
    let claude_md = load_claude_md();
    if !claude_md.is_empty() {
        crate::ui::print_ok("CLAUDE.md loaded");
    }

    let system_prompt = format!("Model: {}\n\n{}{}", client.model(), tool_descriptions(), claude_md);

    let mut history: Vec<Message> = if resume {
        let prev = load_session();
        if prev.is_empty() {
            vec![Message::system(&system_prompt)]
        } else {
            crate::ui::print_ok(&format!("Previous session restored ({} messages)", prev.len()));
            println!();
            prev
        }
    } else {
        vec![Message::system(&system_prompt)]
    };

    let mut current_model = client.model().to_string();
    let mut stats = SessionStats::new();
    let mut plan_mode = false;
    let mut think_mode = false;  // extended reasoning mode
    let ctx_limit_tokens = 128_000usize;  // context limit for most models
    let mut monitor_enabled = false;  // status bar enabled

    let session_label = match std::env::var("AI_SESSION_NAME") {
        Ok(n) if !n.is_empty() => format!(" [{}]", n),
        _ => String::new(),
    };
    crate::ui::print_banner(&current_model, &session_label);

    // Start background system monitor
    let (sys_stats, _monitor_handle) = crate::monitor::start_background_monitor(2000);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Print status bar
        if monitor_enabled {
            let used = estimate_tokens(&history);
            let sys = sys_stats.lock().map(|g| g.clone()).unwrap_or_default();
            // Ollama model status is async, use cached info
            let model_status = crate::monitor::ModelStatus {
                model: current_model.clone(),
                running: true,
                vram_mb: None,
                context_tokens: Some(used),
            };
            crate::monitor::print_status_bar(used, ctx_limit_tokens, &sys, &model_status);
        }

        print!("{}", crate::ui::prompt_prefix(plan_mode, think_mode));
        stdout.flush()?;

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) | Err(_) => { save_session(&history); break; }
            Ok(_) => {}
        }

        let input = input.trim().to_string();
        if input.is_empty() { continue; }

        // ── Slash commands ──────────────────────────────
        if input == "exit" || input == "quit" {
            save_session(&history);
            println!("Session saved. Exiting.");
            break;
        }

        // Prefix-based commands
        let cmd_parts: Vec<&str> = input.splitn(3, ' ').collect();
        let cmd = cmd_parts[0];
        let arg1 = cmd_parts.get(1).copied().unwrap_or("");
        let arg2 = cmd_parts.get(2).copied().unwrap_or("");

        match cmd {
            "/help" => {
                crate::ui::print_help_table();
                continue;
            }

            "/clear" => {
                history = vec![Message::system(&system_prompt)];
                stats = SessionStats::new();
                crate::ui::print_ok("History cleared.");
                println!();
                continue;
            }

            "/resume" => {
                let prev = load_session();
                if prev.is_empty() {
                    crate::ui::print_warn("No saved session found.");
                    println!();
                } else {
                    history = prev;
                    crate::ui::print_ok(&format!("Session restored ({} messages)", history.len()));
                    println!();
                }
                continue;
            }

            "/save" => {
                save_session(&history);
                crate::ui::print_ok(&format!("Session saved ({})", SESSION_FILE));
                println!();
                continue;
            }

            "/history" => {
                if arg1 == "sessions" || arg1 == "list" {
                    // Print stored session list
                    let hist_mgr = crate::history::HistoryManager::new();
                    hist_mgr.print_history();
                } else {
                    // Print current session history
                    let n: usize = arg1.parse().unwrap_or(0);
                    let skip = if n > 0 { history.len().saturating_sub(n) } else { 0 };
                    println!("── Current session history ({} messages) ──", history.len());
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
                    println!("Compression not needed ({} messages / max {})\n", before, MAX_HISTORY);
                } else {
                    print!("Compressing with AI summary... ");
                    stdout.flush()?;
                    let compacted = compact_with_summary(&mut history, client).await;
                    if compacted {
                        println!("Done ({} → {} messages)\n", before, history.len());
                    } else {
                        println!("Failed — applying simple compaction\n");
                        compact_history(&mut history);
                    }
                }
                continue;
            }

            "/models" => {
                match client.list_models().await {
                    Ok(models) => {
                        println!("\n{} ({}):", "Available models".bright_cyan().bold(), models.len());
                        for m in &models {
                            if m == &current_model {
                                println!("  {} {} {}", "▶".bright_green(), m.bright_white().bold(), "◀ current".bright_green());
                            } else {
                                println!("  {} {}", " ".normal(), m.dimmed());
                            }
                        }
                        println!();
                    }
                    Err(e) => crate::ui::print_err(&format!("Failed to list models: {}", e)),
                }
                continue;
            }

            "/model" => {
                if arg1.is_empty() {
                    println!("Current model: {}  |  Usage: {}", current_model.bright_cyan().bold(), "/model <name>".dimmed());
                    println!();
                } else {
                    current_model = arg1.to_string();
                    std::env::set_var("OLLAMA_MODEL", &current_model);
                    let new_prompt = format!("Model: {}\n\n{}", current_model, tool_descriptions());
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            first.content = new_prompt;
                        }
                    }
                    crate::ui::print_ok(&format!("Model changed: {}", current_model));
                    println!();
                }
                continue;
            }

            // ─── New commands ─────────────────────────────
            "/cost" | "/usage" => {
                let hist_tokens = estimate_tokens(&history);
                println!("\n=== Session token usage (estimated) ===");
                println!("  Conversation turns  : {}", stats.turns);
                println!("  Tool calls          : {}", stats.tool_calls);
                println!("  Prompt tokens       : ~{}", stats.est_prompt_tokens);
                println!("  Response tokens     : ~{}", stats.est_response_tokens);
                println!("  Total tokens        : ~{}", stats.total());
                println!("  Context             : {}", context_bar(hist_tokens, ctx_limit_tokens));
                println!("  Elapsed time        : {}", stats.elapsed());
                println!("  (Note: Ollama streaming does not provide exact token counts — values are estimated)\n");
                continue;
            }

            "/context" => {
                let used = estimate_tokens(&history);
                let pct = (used * 100) / ctx_limit_tokens.max(1);
                println!("\n=== Context window ===");
                println!("  {}", context_bar(used, ctx_limit_tokens));
                println!("  Message count       : {}", history.len());
                println!("  Estimated tokens   : ~{}k / {}k", used / 1000, ctx_limit_tokens / 1000);
                if pct > 80 {
                    println!("  ⚠️  Context {}% used — /compact recommended", pct);
                }
                println!();
                continue;
            }

            "/init" => {
                println!("Analyzing project...");
                // Analyze directory structure
                let tree = crate::tools::list_dir(".")
                    .map(|v| v.join("\n"))
                    .unwrap_or_else(|_| "No listing".to_string());
                let git_status = crate::tools::git_status(".")
                    .map(|r| r.output).unwrap_or_default();

                let init_prompt = format!(
                    "Analyze the following project and write a CLAUDE.md file.\n\
                     CLAUDE.md is a document to help AI agents understand the project.\n\n\
                     Include:\n\
                     1. Project overview and purpose\n\
                     2. Main tech stack\n\
                     3. Directory structure explanation\n\
                     4. Development rules/conventions (if any)\n\
                     5. Build/test/run instructions\n\
                     6. Important notes\n\n\
                     ## File list\n{}\n\n## Git status\n{}\n\n\
                     Output only the CLAUDE.md content (markdown format).",
                    crate::utils::trunc(&tree, 2000),
                    crate::utils::trunc(&git_status, 500),
                );

                let tmp_history = vec![
                    Message::system("You are an expert project documentation writer."),
                    Message::user(&init_prompt),
                ];

                print!("\n{}", crate::ui::agent_prefix());
                stdout.flush()?;
                let content = client.chat_stream(tmp_history, |tok| {
                    print!("{}", tok); let _ = std::io::stdout().flush();
                }).await.unwrap_or_else(|e| format!("Error: {}", e));
                println!("\n");

                match std::fs::write("CLAUDE.md", &content) {
                    Ok(_) => {
                        println!("CLAUDE.md created ({} bytes)", content.len());
                        // Apply newly created CLAUDE.md to system prompt
                        let new_claude_md = load_claude_md();
                        let new_prompt = format!("Model: {}\n\n{}{}", current_model, tool_descriptions(), new_claude_md);
                        if let Some(first) = history.first_mut() {
                            if matches!(first.role, crate::models::Role::System) {
                                first.content = new_prompt;
                            }
                        }
                        println!("CLAUDE.md applied to system prompt\n");
                    }
                    Err(e) => println!("Save failed: {}\n", e),
                }
                continue;
            }

            "/add" => {
                if arg1.is_empty() {
                    println!("Usage: /add <file_path>\n");
                } else {
                    match std::fs::read_to_string(arg1) {
                        Ok(content) => {
                            let msg = format!("## File context: {}\n```\n{}\n```", arg1, content);
                            history.push(Message::tool(msg));
                            println!("File added: {} ({} bytes)\n", arg1, content.len());
                        }
                        Err(e) => println!("Failed to read file: {}\n", e),
                    }
                }
                continue;
            }

            "/think" => {
                think_mode = !think_mode;
                if think_mode {
                    println!("Extended reasoning mode ON — think step-by-step before responding.\n");
                    let think_addendum = "\n\n=== Extended reasoning mode ===\n\
                        For every request, first write a step-by-step thought process inside <think> tags,\n\
                        then output the final answer/action after </think>.\n\
                        Examine complex problems from multiple angles and choose the best approach.";
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) && !first.content.contains("Extended reasoning mode") {
                            first.content.push_str(think_addendum);
                        }
                    }
                } else {
                    println!("Extended reasoning mode OFF\n");
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            if let Some(pos) = first.content.find("\n\n=== Extended reasoning mode ===") {
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
                println!("\n=== Session status ===");
                println!("  Model               : {}", current_model);
                println!("  History             : {} messages", msg_count);
                println!("  Plan mode           : {}", if plan_mode { "ON" } else { "OFF" });
                println!("  Think mode          : {}", if think_mode { "ON" } else { "OFF" });
                println!("  Context             : {}", context_bar(hist_tokens, ctx_limit_tokens));
                println!("  Conversation turns  : {}", stats.turns);
                println!("  Tool calls          : {}", stats.tool_calls);
                println!("  Elapsed time        : {}", stats.elapsed());
                let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                println!("  Working directory   : {}", cwd);
                println!("  Session file        : {}", session_file());
                println!();
                continue;
            }

            "/doctor" => {
                println!("\n=== Environment diagnostics ===");
                // Ollama connection
                match client.health_check().await {
                    Ok(true) => println!("  ✅ Ollama server: connected"),
                    _ => println!("  ❌ Ollama server: connection failed ({})", std::env::var("OLLAMA_API_URL").unwrap_or_default()),
                }
                // Model
                match client.list_models().await {
                    Ok(models) => {
                        let has_model = models.iter().any(|m| m == &current_model);
                        if has_model {
                            println!("  ✅ Model '{}': available", current_model);
                        } else {
                            println!("  ⚠️  Model '{}': not found (run ollama pull {})", current_model, current_model);
                        }
                        println!("  ℹ️  Installed models: {}", models.len());
                    }
                    Err(e) => println!("  ❌ Model list: {}", e),
                }
                // Key tools
                for tool in &["git", "docker", "cargo", "python3", "node", "ruff", "pytest"] {
                    let ok = std::process::Command::new("which").arg(tool)
                        .output().map(|o| o.status.success()).unwrap_or(false);
                    println!("  {} {}", if ok { "✅" } else { "  " }, tool);
                }
                // Disk
                if let Ok(r) = crate::tools::system::run_shell("df -h . | tail -1") {
                    println!("  ℹ️  Disk: {}", r.stdout.trim());
                }
                // Session file
                let has_session = std::path::Path::new(SESSION_FILE).exists();
                println!("  {} Session file ({}): {}", if has_session { "✅" } else { "  " }, SESSION_FILE, if has_session { "present" } else { "absent" });
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

                let mut md = format!("# AI Agent Conversation Export\n\nModel: `{}`  \nElapsed: {}\n\n---\n\n", current_model, stats.elapsed());
                for msg in &history {
                    match msg.role {
                        crate::models::Role::System => {}
                        crate::models::Role::User => md.push_str(&format!("**You:** {}\n\n", msg.content)),
                        crate::models::Role::Assistant => md.push_str(&format!("**Agent:** {}\n\n", msg.content)),
                        crate::models::Role::Tool => md.push_str(&format!("```\n{}\n```\n\n", msg.content)),
                    }
                }
                match std::fs::write(&filename, &md) {
                    Ok(_) => println!("Conversation exported: {} ({} bytes)\n", filename, md.len()),
                    Err(e) => println!("Export failed: {}\n", e),
                }
                continue;
            }

            "/commit" => {
                let repo_path = if arg1.is_empty() { "." } else { arg1 };
                println!("Analyzing git diff...");
                let diff = crate::tools::git_diff(repo_path, false)
                    .map(|r| r.output)
                    .unwrap_or_else(|e| format!("diff error: {}", e));
                let status = crate::tools::git_status(repo_path)
                    .map(|r| r.output)
                    .unwrap_or_default();

                if diff.trim().is_empty() && status.trim().is_empty() {
                    println!("No changes found.\n");
                    continue;
                }

                let commit_prompt = format!(
                    "Analyze the following git diff and write a commit message in Conventional Commits format.\n\
                     Format: <type>(<scope>): <description>\n\
                     Types: feat/fix/docs/style/refactor/perf/test/build/ci/chore\n\
                     Write subject only, no body (1-2 lines).\n\n\
                     ## git status\n{}\n\n## git diff\n{}",
                    crate::utils::trunc(&status, 500),
                    crate::utils::trunc(&diff, 3000)
                );
                let tmp_history = vec![
                    Message::system("You are an expert git commit message writer."),
                    Message::user(&commit_prompt),
                ];
                print!("\n{}", crate::ui::agent_prefix());
                stdout.flush()?;
                let response = client.chat_stream(tmp_history, |tok| {
                    print!("{}", tok); let _ = std::io::stdout().flush();
                }).await.unwrap_or_else(|e| format!("Error: {}", e));
                println!("\n");
                stats.add_prompt(&commit_prompt);
                stats.add_response(&response);
                continue;
            }

            "/review" => {
                let repo_path = if arg1.is_empty() { "." } else { arg1 };
                println!("Analyzing code changes...");
                let diff = crate::tools::git_diff(repo_path, false)
                    .map(|r| r.output)
                    .unwrap_or_else(|e| format!("diff error: {}", e));

                if diff.trim().is_empty() {
                    println!("No changes to review.\n");
                    continue;
                }

                let review_prompt = format!(
                    "Please review the following code changes.\n\
                     Check for:\n\
                     1. Potential bugs\n\
                     2. Security vulnerabilities\n\
                     3. Performance issues\n\
                     4. Code quality and readability\n\
                     5. Improvement suggestions\n\n\
                     ## git diff\n{}",
                    crate::utils::trunc(&diff, 4000)
                );
                let tmp_history = vec![
                    Message::system("You are a senior software engineer and expert code reviewer."),
                    Message::user(&review_prompt),
                ];
                print!("\nReview> ");
                stdout.flush()?;
                let response = client.chat_stream(tmp_history, |tok| {
                    print!("{}", tok); let _ = std::io::stdout().flush();
                }).await.unwrap_or_else(|e| format!("Error: {}", e));
                println!("\n");
                stats.add_prompt(&review_prompt);
                stats.add_response(&response);
                continue;
            }

            "/plan" => {
                plan_mode = !plan_mode;
                if plan_mode {
                    println!("Plan mode ON — focus on planning before execution.\n");
                    // Add plan directives to system prompt
                    let plan_addendum = "\n\n=== Plan mode ===\n\
                        Upon receiving a user request, first write a step-by-step execution plan,\n\
                        then execute after user confirmation. Always share the plan before calling tools.";
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            if !first.content.contains("Plan mode") {
                                first.content.push_str(plan_addendum);
                            }
                        }
                    }
                } else {
                    println!("Plan mode OFF — returning to normal mode.\n");
                    // Remove plan directives
                    if let Some(first) = history.first_mut() {
                        if matches!(first.role, crate::models::Role::System) {
                            if let Some(pos) = first.content.find("\n\n=== Plan mode ===") {
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
                    "save" | "add" => {
                        let note = if arg2.is_empty() { arg1 } else { arg2 };
                        if note.is_empty() || note == "save" || note == "add" {
                            println!("Usage: /memory save <note>\n");
                        } else {
                            let id = entries.len() + 1;
                            let ts = chrono_now();
                            entries.push(MemoryEntry { id, note: note.to_string(), created: ts });
                            memory_save(&entries);
                            println!("Note saved (ID: {})\n", id);
                        }
                    }
                    "list" | "ls" | "" => {
                        if entries.is_empty() {
                            println!("No saved notes.\n");
                        } else {
                            println!("=== Notes ({}) ===", entries.len());
                            for e in &entries {
                                println!("[{}] ({}) {}", e.id, e.created, e.note);
                            }
                            println!();
                        }
                    }
                    "clear" => {
                        entries.clear();
                        memory_save(&entries);
                        println!("All notes deleted.\n");
                    }
                    "del" | "rm" => {
                        if let Ok(id) = arg2.parse::<usize>() {
                            let before = entries.len();
                            entries.retain(|e| e.id != id);
                            memory_save(&entries);
                            if entries.len() < before { println!("Note #{} deleted.\n", id); }
                            else { println!("Note #{} not found.\n", id); }
                        } else {
                            println!("Usage: /memory del <ID>\n");
                        }
                    }
                    _ => println!("Usage: /memory [save <note>|list|clear|del <ID>]\n"),
                }
                continue;
            }

            "/config" | "/settings" => {
                let app_cfg = crate::config::AppConfig::load();
                match arg1 {
                    "" => {
                        println!("=== Current settings ===");
                        println!("Model:           {}", app_cfg.ollama.model);
                        println!("API URL:         {}", app_cfg.ollama.api_url);
                        println!("Timeout:         {}s", app_cfg.ollama.timeout_secs);
                        println!("Max turns:       {}", app_cfg.agent.max_turns);
                        println!("History:         {}", if app_cfg.agent.history_enabled { "enabled" } else { "disabled" });
                        println!("Context:         max {} messages", app_cfg.agent.history_max_context);
                        println!("Project:         {}", app_cfg.agile.project);
                        println!("QA retries:      max {}", app_cfg.agile.max_qa_retries);
                        println!("Security rounds: max {}", app_cfg.agile.max_security_rounds);
                        println!("\nConfig file: ai-agent.toml (local) | ~/.config/ai-agent/config.toml (global)");
                        println!("Initialize: /config-init\n");
                    }
                    "init" | "-init" => {
                        let path = std::path::PathBuf::from("ai-agent.toml");
                        match crate::config::AppConfig::save_default(&path) {
                            Ok(()) => println!("Config file created: {:?}\n", path),
                            Err(e) => println!("Failed to create config file: {}\n", e),
                        }
                    }
                    _ => {
                        // Legacy JSON config (backwards compatibility)
                        let mut cfg = config_load();
                        if arg2.is_empty() {
                            match cfg.get(arg1) {
                                Some(v) => println!("{} = {}\n", arg1, v),
                                None => println!("'{}' not configured.\n", arg1),
                            }
                        } else {
                            let json_val: serde_json::Value = arg2.parse()
                                .unwrap_or_else(|_| serde_json::Value::String(arg2.to_string()));
                            cfg[arg1] = json_val;
                            config_save(&cfg);
                            println!("Config saved: {} = {}\n", arg1, arg2);
                        }
                    }
                }
                continue;
            }

            // ── Agile Sprint ────────────────────────────────
            "/agile" | "/sprint" => {
                let rest = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                let (fast, task) = if rest.starts_with("--fast") {
                    (true, rest.trim_start_matches("--fast").trim().to_string())
                } else {
                    (false, rest.to_string())
                };
                if task.is_empty() {
                    println!("Usage: /agile [--fast] <task>\nExamples:\n  /agile implement user authentication\n  /agile --fast login feature\n");
                } else {
                    let project = std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string());
                    println!("\n🏃 Agile sprint started{}: {}", if fast { " (fast)" } else { "" }, crate::utils::trunc(&task, 60));
                    match crate::agile::run_agile_sprint_opts(client, &project, &task, fast, |msg| {
                        println!("{}", msg);
                    }).await {
                        Ok(result) => {
                            let summary = format!(
                                "Sprint complete — done: {}, released: {}, failed: {}, bugs: {}, docs: {}, velocity: {}pts",
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
                        Err(e) => println!("Sprint error: {}\n", e),
                    }
                }
                continue;
            }

            // ── Sprint retrospective ──────────────────────────────
            "/retro" | "/retrospective" => {
                let sprint_id = if arg1.is_empty() { None } else { Some(arg1) };
                let project = std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string());
                let board = crate::agile::AgileBoard::load_or_new(&project);
                println!("\n🔄 Starting sprint retrospective...");
                match crate::agile::run_retrospective(client, &board, sprint_id, |msg| {
                    println!("{}", msg);
                }).await {
                    Ok(result) => {
                        let summary = format!(
                            "Retrospective done — team health: {}/10, actions: {}, trend: {}",
                            result.team_health_score, result.action_items.len(), result.velocity_trend
                        );
                        println!("\n{}\n", summary);
                        history.push(Message::tool(summary));
                    }
                    Err(e) => println!("Retrospective error: {}\n", e),
                }
                continue;
            }

            // ── Post-mortem ─────────────────────────────────────
            "/postmortem" | "/pm" => {
                let desc = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if desc.is_empty() {
                    println!("Usage: /postmortem <incident description>\nExample: /postmortem API server down — OOM due to memory leak\n");
                } else {
                    let path = std::env::current_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| ".".to_string());
                    println!("\n🚨 Starting post-mortem analysis...");
                    match crate::agile::run_postmortem(client, desc, &path, |msg| {
                        println!("{}", msg);
                    }).await {
                        Ok(result) => {
                            let summary = format!(
                                "Post-mortem done — {} | severity: {} | actions: {}",
                                result.incident_id, result.severity, result.action_items.len()
                            );
                            println!("\n{}\n", summary);
                            history.push(Message::tool(summary));
                        }
                        Err(e) => println!("Post-mortem error: {}\n", e),
                    }
                }
                continue;
            }

            // ── Tech debt analysis ─────────────────────────────────
            "/techdebt" | "/debt" => {
                let path = if arg1.is_empty() { "." } else { arg1 };
                println!("\n📊 Tech debt analysis started: {}", path);
                match crate::agile::run_techdebt_analysis(client, path, |msg| {
                    println!("{}", msg);
                }).await {
                    Ok(report) => {
                        let summary = format!(
                            "Tech debt analysis done — {} items, ~{} days estimated, debt ratio: {}",
                            report.debt_items.len(), report.total_estimated_days, report.debt_ratio
                        );
                        println!("\n{}\n", summary);
                        history.push(Message::tool(summary));
                    }
                    Err(e) => println!("Tech debt analysis error: {}\n", e),
                }
                continue;
            }

            // ── Standalone role execution ─────────────────────────────
            "/ba" | "/biz-analyst" => {
                let task = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if task.is_empty() {
                    println!("Usage: /ba <feature description to analyze>\n");
                } else {
                    let hub = crate::agent::node::NodeHub::new();
                    println!("\n📊 Business analysis started...");
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
                    println!("Usage: /ux <feature description to design UX for>\n");
                } else {
                    let hub = crate::agent::node::NodeHub::new();
                    println!("\n🎨 UX design started...");
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
                let task = format!("Generate CI/CD pipeline, Dockerfile, and K8s manifests for project path {}.", path);
                let hub = crate::agent::node::NodeHub::new();
                println!("\n🚀 DevOps setup generation started: {}", path);
                let out = crate::agile::run_role_standalone(
                    client, crate::agile::AgileRole::DevOpsEngineer, &task, "", &hub, &|msg| println!("{}", msg)
                ).await;
                println!("\n{}\n", crate::utils::trunc(&out, 600));
                history.push(Message::tool(crate::utils::trunc(&out, 400).to_string()));
                continue;
            }

            "/docs" | "/document" => {
                let path = if arg1.is_empty() { "." } else { arg1 };
                let task = format!("Write README, API docs, and architecture docs for project path {}.", path);
                let hub = crate::agent::node::NodeHub::new();
                println!("\n📝 Technical documentation generation started: {}", path);
                let out = crate::agile::run_role_standalone(
                    client, crate::agile::AgileRole::TechnicalWriter, &task, "", &hub, &|msg| println!("{}", msg)
                ).await;
                println!("\n{}\n", crate::utils::trunc(&out, 600));
                history.push(Message::tool(crate::utils::trunc(&out, 400).to_string()));
                continue;
            }

            "/sre" => {
                let path = if arg1.is_empty() { "." } else { arg1 };
                let task = format!("Write SLO, Prometheus alerts, Grafana dashboards, and runbooks for project path {}.", path);
                let hub = crate::agent::node::NodeHub::new();
                println!("\n📡 SRE configuration generation started: {}", path);
                let out = crate::agile::run_role_standalone(
                    client, crate::agile::AgileRole::SRE, &task, "", &hub, &|msg| println!("{}", msg)
                ).await;
                println!("\n{}\n", crate::utils::trunc(&out, 600));
                history.push(Message::tool(crate::utils::trunc(&out, 400).to_string()));
                continue;
            }

            // ── Coordinator parallel multi-agent ──────────────────────────
            "/coordinator" | "/coord" => {
                let task = input.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if task.is_empty() {
                    println!("Usage: /coordinator <complex task>\nExample: /coordinator backend API + frontend UI + tests simultaneously\n");
                } else {
                    println!("\n🤝 Coordinator started: {}", crate::utils::trunc(task, 60));
                    match crate::agile::run_coordinator(client, task, |msg| println!("{}", msg)).await {
                        Ok(result) => {
                            let summary = format!(
                                "Coordinator done — {} parallel workers, {} subtasks",
                                result.total_workers, result.subtasks.len()
                            );
                            println!("\n{}\n", summary);
                            history.push(Message::tool(crate::utils::trunc(&result.synthesis, 600).to_string()));
                        }
                        Err(e) => println!("Coordinator error: {}\n", e),
                    }
                }
                continue;
            }

            // ── RAG codebase indexing/search ───────────────────────────
            "/rag" => {
                match arg1 {
                    "index" => {
                        let path = if arg2.is_empty() { "." } else { arg2 };
                        println!("📚 Indexing codebase: {} ...", path);
                        match crate::agent::rag::index_codebase(path) {
                            Ok(index) => {
                                let status = index.status();
                                crate::agent::rag::save_index(&index).ok();
                                println!("✅ Indexing complete\n{}\n", status);
                            }
                            Err(e) => println!("Indexing error: {}\n", e),
                        }
                    }
                    "query" | "q" => {
                        let query = input.splitn(4, ' ').nth(2).unwrap_or("").trim().to_string();
                        if query.is_empty() {
                            println!("Usage: /rag query <question>\n");
                        } else {
                            match crate::agent::rag::load_index() {
                                Some(index) => {
                                    let chunks = crate::agent::rag::search(&index, &query);
                                    if chunks.is_empty() {
                                        println!("No relevant code found. Run /rag index first.\n");
                                    } else {
                                        let ctx = crate::agent::rag::build_context(&chunks);
                                        println!("🔍 {} chunks found — querying AI...", chunks.len());
                                        let mut rag_history = history.clone();
                                        rag_history.push(Message::tool(ctx));
                                        rag_history.push(Message::user(&query));
                                        print!("{}", crate::ui::agent_prefix());
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
                                            Err(e) => println!("\nError: {}\n", e),
                                        }
                                    }
                                }
                                None => println!("No RAG index found. Run /rag index first.\n"),
                            }
                        }
                    }
                    "status" | "st" | "" => {
                        match crate::agent::rag::load_index() {
                            Some(index) => println!("\n📊 RAG index status\n{}\n", index.status()),
                            None => println!("No RAG index. Index with /rag index [path].\n"),
                        }
                    }
                    _ => println!("Usage: /rag [index|query|status]\n"),
                }
                continue;
            }

            // ── GitHub PR management ────────────────────────────────────────
            "/pr" | "/pull-request" => {
                match arg1 {
                    "list" | "ls" | "" => {
                        let state = if arg2.is_empty() { "open" } else { arg2 };
                        match crate::agent::github::list_prs(state) {
                            Ok(output) => println!("\n📋 PR list ({})\n{}\n", state, output),
                            Err(e) => println!("PR list error: {}\n", e),
                        }
                    }
                    "create" => {
                        let branch = crate::agent::github::current_branch();
                        let title = if arg2.is_empty() {
                            format!("feat: {} branch changes", branch)
                        } else { arg2.to_string() };

                        // Generate PR body with AI
                        println!("📝 Generating AI PR body...");
                        let git_log = std::process::Command::new("git")
                            .args(["log", "--oneline", "-10"])
                            .output()
                            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                            .unwrap_or_default();

                        let body_prompt = format!(
                            "Write a GitHub PR body in markdown based on the following git log.\n\n{}", git_log
                        );
                        let body_msgs = vec![
                            Message::system("You are an expert GitHub PR author."),
                            Message::user(&body_prompt),
                        ];
                        let pr_body = match client.chat_stream(body_msgs, |_| {}).await {
                            Ok(t) => t,
                            Err(_) => "AI-generated PR body".to_string(),
                        };

                        let opts = crate::agent::github::PrOptions::new(&title, &pr_body);
                        match crate::agent::github::create_pr(&opts) {
                            Ok(result) => println!("✅ PR created: {}\n", result.url),
                            Err(e) => println!("PR creation error: {}\n", e),
                        }
                    }
                    _ => println!("Usage: /pr [list|create]\n"),
                }
                continue;
            }

            // Board status query
            "/board" => {
                let project = if arg1.is_empty() {
                    std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string())
                } else { arg1.to_string() };
                let board = crate::agile::AgileBoard::load_or_new(&project);
                println!("{}", board.render());
                println!("{}", board.render_burndown());
                continue;
            }

            // Security audit (HackerAgent standalone)
            "/security" | "/hack" | "/audit" => {
                let project_path = if arg1.is_empty() { "." } else { arg1 };
                let project = std::env::var("AI_PROJECT").unwrap_or_else(|_| "project".to_string());
                let board = crate::agile::AgileBoard::load_or_new(&project);
                let hub = crate::agent::node::NodeHub::new();

                // Find in-progress story on board or create temporary story
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
                    // No story found, create temporary story for manual audit
                    let new_sid = board.next_story_id();
                    let mut tmp = crate::agile::story::UserStory::new(
                        &new_sid, "Manual security audit", "User-requested manual security audit",
                        crate::agile::story::Priority::High, 3,
                    );
                    tmp.implementation = Some(format!("Project path: {}", project_path));
                    let _ = board.add_story(tmp);
                    new_sid
                };

                println!("\n🔒 Security audit started — path: {}", project_path);
                let sec = crate::agile::hacker::run_security_fix_loop(
                    &client, &board, &hub, &sid, project_path,
                    |msg| println!("{}", msg),
                ).await;
                println!("{}", sec.final_report.render());
                println!(
                    "\nResult: {} | rounds: {} | total vulnerabilities: {} | unresolved: {}",
                    if sec.approved { "✅ Passed" } else { "⚠️ Unresolved issues" },
                    sec.rounds,
                    sec.final_report.vulnerabilities.len(),
                    sec.final_report.unfixed_count(),
                );
                continue;
            }

            // Start IPC server (can be done during chat)
            "/ipc" | "/ipc-server" => {
                let port: u16 = arg1.parse().unwrap_or(8765);
                println!("Starting IPC HTTP server (port {})...", port);
                println!("Other AIs can send JSON-RPC requests to POST http://localhost:{}", port);
                println!("Example: curl -X POST http://localhost:{} -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"params\":{{}}}}'", port);
                let new_client = crate::agent::OllamaClient::from_env();
                let server = crate::ipc::AgentServer::new(new_client);
                tokio::spawn(async move {
                    if let Err(e) = server.run_http_server(port).await {
                        eprintln!("[IPC] Server error: {}", e);
                    }
                });
                println!("IPC server running in background.\n");
                continue;
            }

            // Multi-agent pipeline
            "/pipeline" | "/pipe" => {
                let task = if arg2.is_empty() {
                    arg1.to_string()
                } else {
                    format!("{} {}", arg1, arg2)
                };
                if task.is_empty() {
                    println!("Usage: /pipeline <task description>\n");
                } else {
                    match crate::agent::orchestrator::run_pipeline(client, &task).await {
                        Ok(result) => {
                            println!("\n📋 Plan:\n{}", crate::utils::trunc(&result.plan, 500));
                            println!("\n💻 Implementation:\n{}", crate::utils::trunc(&result.implementation, 500));
                            println!("\n🔍 Verification:\n{}", crate::utils::trunc(&result.verification, 400));
                            if let Some(ref r) = result.review {
                                println!("\n👁️ Review:\n{}", crate::utils::trunc(r, 400));
                            }
                            println!();
                            history.push(Message::tool(format!(
                                "Pipeline done:\nPlan: {}\nImplementation: {}\nVerification: {}",
                                crate::utils::trunc(&result.plan, 300),
                                crate::utils::trunc(&result.implementation, 300),
                                crate::utils::trunc(&result.verification, 200),
                            )));
                        }
                        Err(e) => println!("Pipeline error: {}\n", e),
                    }
                }
                continue;
            }

            // Impact analysis
            "/impact" => {
                if arg1.is_empty() {
                    println!("Usage: /impact <file_path> [change_description]\n");
                } else {
                    print!("Analyzing impact... ");
                    stdout.flush()?;
                    match crate::agent::react::analyze_impact(client, arg1, arg2).await {
                        Ok(analysis) => println!("\n{}\n", analysis),
                        Err(e) => println!("Analysis failed: {}\n", e),
                    }
                }
                continue;
            }

            // /monitor — real-time status bar toggle
            "/monitor" | "/mon" => {
                monitor_enabled = !monitor_enabled;
                if monitor_enabled {
                    println!("System monitor ON — status displayed above each prompt\n");
                } else {
                    println!("System monitor OFF\n");
                }
                continue;
            }

            // /sysinfo — one-shot system status print
            "/sysinfo" | "/sys" => {
                print!("Collecting system info... ");
                stdout.flush()?;
                let sys = crate::monitor::SystemStats::collect();
                println!("\n");
                println!("=== System status ===");
                println!("  CPU usage    : {:.1}%", sys.cpu_pct);
                println!("  Memory       : {} / {} MB ({:.0}%)",
                    sys.mem_used_mb, sys.mem_total_mb,
                    sys.mem_used_mb as f32 * 100.0 / sys.mem_total_mb.max(1) as f32);
                if let Some(pct) = sys.gpu_pct {
                    println!("  GPU name     : {}", sys.gpu_name.as_deref().unwrap_or("Unknown"));
                    println!("  GPU usage    : {:.1}%", pct);
                    if let (Some(used), Some(total)) = (sys.gpu_mem_used_mb, sys.gpu_mem_total_mb) {
                        println!("  VRAM         : {} / {} MB ({:.0}%)", used, total,
                            used as f32 * 100.0 / total.max(1) as f32);
                    }
                } else {
                    println!("  GPU          : nvidia-smi / rocm-smi not found");
                }

                // Ollama model status
                print!("\n  Ollama       : ");
                stdout.flush()?;
                let model_status = crate::monitor::get_model_status(&current_model).await;
                if model_status.running {
                    println!("Running ({}{})", current_model,
                        model_status.vram_mb.map(|m| format!(" {:.1}GB VRAM", m as f64 / 1024.0)).unwrap_or_default());
                } else {
                    println!("Idle (model not loaded)");
                }

                // Context
                let used = estimate_tokens(&history);
                let pct = used * 100 / ctx_limit_tokens.max(1);
                println!("  Context      : ~{}k / {}k tokens ({:.0}%)",
                    used / 1000, ctx_limit_tokens / 1000, pct);
                println!("  Session      : {} turns, {} tool calls, {}s elapsed",
                    stats.turns, stats.tool_calls, stats.start.elapsed().as_secs());
                println!();
                continue;
            }

            "/skills" | "/skill-list" => {
                println!("\n{}\n", tool_descriptions());
                // Loaded user skill list
                let mut skill_reg = crate::skills::SkillRegistry::new();
                skill_reg.load_all();
                if !skill_reg.is_empty() {
                    println!("=== User skills ({}) ===", skill_reg.len());
                    for s in skill_reg.all() {
                        println!("  /{} — {}", s.name, s.description);
                    }
                    println!();
                }
                continue;
            }

            // /skill <name> [args...] — execute skill
            "/skill" => {
                if arg1.is_empty() {
                    println!("Usage: /skill <name> [args...]\n");
                } else {
                    let mut skill_reg = crate::skills::SkillRegistry::new();
                    skill_reg.load_all();
                    // Collect remaining args
                    let extra_args: Vec<&str> = input.splitn(3, ' ').skip(2).collect();
                    let args_refs: Vec<&str> = std::iter::once(arg2)
                        .chain(extra_args.iter().map(|s| *s))
                        .filter(|s| !s.is_empty())
                        .collect();
                    print!("{}{} ", crate::ui::agent_prefix(), format!("[skill: {}]", arg1).bright_yellow());
                    stdout.flush()?;
                    match crate::skills::execute_skill(&skill_reg, client, arg1, &args_refs, |tok| {
                        print!("{}", tok);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }).await {
                        Ok(result) => {
                            println!("\n");
                            history.push(Message::tool(format!("Skill '{}' result:\n{}", arg1, result)));
                        }
                        Err(e) => println!("\nSkill error: {}\n", e),
                    }
                }
                continue;
            }

            // /skill-new <name> <description> — create new skill
            "/skill-new" => {
                if arg1.is_empty() || arg2.is_empty() {
                    println!("Usage: /skill-new <name> <description>\n");
                } else {
                    let template = format!(
                        "Handle the following request:\n{{{{args}}}}\n\nTask: {}",
                        arg1
                    );
                    match crate::skills::loader::SkillRegistry::create_skill_file(arg1, arg2, &[], &template) {
                        Ok(path) => println!("Skill created: {}\nEdit it then run with /skill {}.\n", path, arg1),
                        Err(e) => println!("Failed to create skill: {}\n", e),
                    }
                }
                continue;
            }

            // /mcp — MCP server and tool list
            "/mcp" => {
                let mut reg = crate::mcp::McpRegistry::from_config();
                if reg.server_count() == 0 {
                    println!("No MCP servers. Configure in ~/.claude/mcp_servers.json or ./.mcp_servers.json.\n");
                    println!("Example format:");
                    println!(r#"[{{"name":"filesystem","type":"stdio","command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","/"]}}]"#);
                    println!();
                } else {
                    println!("MCP servers ({}): {}", reg.server_count(), reg.server_names().join(", "));
                    print!("Loading tool list... ");
                    stdout.flush()?;
                    let count = reg.discover_tools().await;
                    println!("Done ({} tools)\n", count);
                    for tool in reg.tools() {
                        println!("  [{}] {} — {}", tool.server, tool.name, tool.description);
                    }
                    println!();
                }
                continue;
            }

            // /mcp-call <server> <tool> <json_args> — direct MCP tool call
            "/mcp-call" => {
                // arg1 = server, arg2 = tool, rest = json args
                let parts: Vec<&str> = input.splitn(5, ' ').collect();
                let server = parts.get(2).copied().unwrap_or("");
                let tool = parts.get(3).copied().unwrap_or("");
                let json_str = parts.get(4).copied().unwrap_or("{}");
                if server.is_empty() || tool.is_empty() {
                    println!("Usage: /mcp-call <server> <tool> <json_args>\n");
                } else {
                    let mut reg = crate::mcp::McpRegistry::from_config();
                    reg.discover_tools().await;
                    let args: serde_json::Value = serde_json::from_str(json_str)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    print!("{}{} ", "MCP call".bright_cyan(), format!(" [{}/{}]...", server, tool).dimmed());
                    stdout.flush()?;
                    match reg.call_tool(tool, args).await {
                        Ok(result) => {
                            println!("\n{}\n", result.output);
                            history.push(Message::tool(format!("MCP [{}/{}] result:\n{}", server, tool, result.output)));
                        }
                        Err(e) => println!("\nMCP error: {}\n", e),
                    }
                }
                continue;
            }

            // /nodes — node pipeline execution
            "/nodes" | "/node-pipeline" => {
                let task = if arg2.is_empty() { arg1.to_string() } else { format!("{} {}", arg1, arg2) };
                if task.is_empty() {
                    println!("Usage: /nodes <task>\nRuns a node pipeline: Planner→Developer→Debugger.\n");
                } else {
                    println!("Starting node pipeline: {}", crate::utils::trunc(&task, 80));
                    let hub = crate::agent::node::NodeHub::new();
                    match crate::agent::node::run_node_pipeline(&hub, client, &task, |msg| {
                        println!("  {}", msg);
                    }).await {
                        Ok(result) => {
                            println!("\nDone:\n{}\n", crate::utils::trunc(&result, 600));
                            history.push(Message::tool(format!("Node pipeline result:\n{}", result)));
                        }
                        Err(e) => println!("Pipeline error: {}\n", e),
                    }
                }
                continue;
            }

            _ if cmd.starts_with('/') => {
                // Check if user-defined skill
                let skill_name = &cmd[1..];
                let mut skill_reg = crate::skills::SkillRegistry::new();
                skill_reg.load_all();
                if skill_reg.get(skill_name).is_some() {
                    let extra: Vec<&str> = input.splitn(3, ' ').skip(1).collect();
                    let args_refs: Vec<&str> = extra.iter().map(|s| *s).collect();
                    print!("Running skill '{}'> ", skill_name);
                    stdout.flush()?;
                    match crate::skills::execute_skill(&skill_reg, client, skill_name, &args_refs, |tok| {
                        print!("{}", tok);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }).await {
                        Ok(result) => {
                            println!("\n");
                            history.push(Message::tool(format!("Skill '{}' result:\n{}", skill_name, result)));
                        }
                        Err(e) => println!("\nSkill error: {}\n", e),
                    }
                } else {
                    crate::ui::print_warn(&format!("Unknown command: '{}'. See {}", cmd, "/help".bright_cyan()));
                }
                continue;
            }

            _ => {}
        }

        stats.add_prompt(&input);
        history.push(Message::user(&input));

        // Auto AI summarization if context exceeds 80%
        let used_tokens = estimate_tokens(&history);
        if used_tokens > ctx_limit_tokens * 80 / 100 {
            print!("[Context {}% — auto-compressing with AI...]", used_tokens * 100 / ctx_limit_tokens);
            stdout.flush()?;
            compact_with_summary(&mut history, client).await;
            println!(" Done\n");
        } else {
            compact_history(&mut history);
        }

        // ── Max 20-turn tool call loop ───────────────────
        for turn in 0..20 {
            debug!("Requesting AI response (turn={})", turn);

            print!("\n{}", crate::ui::agent_prefix());
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
                    println!("\nAgent exit requested.");
                    return Ok(());
                }

                AgentResponse::Text(_) => {
                    println!();
                    stats.add_response(&ai_text);
                    history.push(Message::assistant(&ai_text));
                    break;
                }

                AgentResponse::ToolCall(tool_call) if tool_call.name == "__multi__" => {
                    // Execute multi-tool
                    crate::ui::print_multi_tool_header(tool_call.args.len());
                    stats.add_response(&ai_text);
                    stats.tool_calls += tool_call.args.len();
                    let _any_ok = execute_multi_tool(&tool_call.args, &mut history, &ai_text).await;
                    println!();

                    if turn >= 4 {
                        history.push(Message::tool(
                            "[WARNING] Repeated failure. Try a different approach.".to_string()
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

                    crate::ui::print_tool_start(&tool_call.name, &args_preview.join(" "));

                    let result = dispatch_tool(&tool_call).await;
                    crate::ui::print_tool_result(result.success, &crate::utils::trunc(&result.output, 300).to_string());
                    println!();

                    history.push(Message::assistant(&ai_text));
                    history.push(Message::tool(format!(
                        "Tool '{}' result:\n{}", tool_call.name, result.output
                    )));

                    if !result.success && turn >= 4 {
                        history.push(Message::tool(
                            "[WARNING] Same tool repeatedly failing. Try a different approach.".to_string()
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
        let r = parse_response_pub("Hello there");
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
