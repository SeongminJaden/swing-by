//! Common agent runner
//!
//! pipeline.rs / postmortem.rs / retrospective.rs / techdebt.rs 등
//! Low-level agent execution functions shared by all pipelines.
//!
//! Claude Code 아키텍처 참조:
//!   - __multi__ 배치 툴은 읽기 전용인 경우 tokio::spawn으로 병렬 실행
//!   - 쓰기/부작용 툴은 순차 실행 (안전성)

use crate::agent::ollama::OllamaClient;
use crate::agent::node::{NodeHub, NodeMessage, MsgType};
use crate::agile::story::UserStory;
use crate::agile::team::AgileRole;
use crate::models::Message;

// List of read-only tools safe for parallel execution
const SAFE_PARALLEL_TOOLS: &[&str] = &[
    "read_file", "list_dir", "glob", "grep", "current_dir",
    "git_status", "git_log", "git_diff", "git_blame", "git_root",
    "git_changed_files", "git_staged_files", "git_remote_branches",
    "git_tag_list", "git_remote_list",
    "web_fetch", "web_search", "research", "docs_fetch",
    "pkg_info", "pkg_versions", "pkg_search", "pkg_list",
    "sysinfo", "process_list", "env_list", "get_env",
    "docker_ps", "docker_images", "docker_stats",
    "docker_network_ls", "docker_volume_ls", "docker_inspect",
    "todo_read",
];

// ─── Tool-using agent (multi-turn, tool calls enabled) ──────────────────────────────

pub async fn run_agile_agent(
    client: &OllamaClient,
    role: AgileRole,
    story: &UserStory,
    extra_ctx: &str,
    hub: &NodeHub,
    on_progress: &impl Fn(&str),
) -> String {
    let board_ctx = format!(
        "## 유저 스토리\nID: {} | 제목: {}\n설명: {}\n수락 기준:\n{}\n\n{}",
        story.id, story.title, story.description,
        story.acceptance_criteria.iter().enumerate()
            .map(|(i, c)| format!("  {}. {}", i+1, c))
            .collect::<Vec<_>>().join("\n"),
        extra_ctx,
    );

    // Role-specific model override (config file takes priority)
    let role_model = crate::config::AppConfig::load().roles.model_for(role.name());
    let role_client_holder;
    let effective_client: &OllamaClient = if let Some(ref model) = role_model {
        role_client_holder = OllamaClient::new(
            std::env::var("OLLAMA_API_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
            model.clone(),
        );
        on_progress(&format!("  🔀 [{}] 모델 전환: {}", role.name(), model));
        &role_client_holder
    } else {
        client
    };

    let system = format!(
        "모델: {}\n\n{}\n\n{}",
        effective_client.model(),
        crate::agent::tools::tool_descriptions(),
        role.system_prompt(&board_ctx),
    );

    let user_msg = format!(
        "다음 유저 스토리를 {} 역할로 처리하세요.\n\
         🔍 중요: 최신 기술 동향, 베스트 프랙티스, 관련 논문을 web_search 툴로 먼저 검색하여 최선의 방법을 적용하세요.\n\n{}",
        role.name(), board_ctx
    );

    let mut history = vec![
        Message::system(&system),
        Message::user(&user_msg),
    ];

    let _ = hub.send(NodeMessage {
        from: role.name().to_string(), to: String::new(),
        msg_type: MsgType::Status,
        content: format!("{} [{}] 시작", role.name(), story.id),
        metadata: Default::default(),
    }).await;

    let mut final_output = String::new();
    let mut tool_calls = 0usize;

    for turn in 0..role.max_turns() {
        let ai_text = match effective_client.chat_stream(history.clone(), |_| {}).await {
            Ok(t) => t,
            Err(e) => return format!("오류: {}", e),
        };

        match crate::agent::chat::parse_response_pub(&ai_text) {
            crate::models::AgentResponse::Exit => break,
            crate::models::AgentResponse::Text(_) => {
                final_output = ai_text.clone();
                history.push(Message::assistant(&ai_text));
                break;
            }
            crate::models::AgentResponse::ToolCall(tc) if tc.name == "__multi__" => {
                history.push(Message::assistant(&ai_text));

                // Parse each tool
                let parsed: Vec<(String, Vec<String>)> = tc.args.iter().filter_map(|raw| {
                    let val = serde_json::from_str::<serde_json::Value>(raw).ok()?;
                    let name = val["name"].as_str()?.to_string();
                    let args: Vec<String> = val["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    Some((name, args))
                }).collect();

                // All safe tools → parallel execution (Claude Code reference)
                let all_safe = parsed.iter().all(|(n, _)| SAFE_PARALLEL_TOOLS.contains(&n.as_str()));
                let results: Vec<String>;

                if all_safe && parsed.len() > 1 {
                    on_progress(&format!("  ⚡ [{}] {}개 툴 병렬 실행...", role.name(), parsed.len()));
                    let handles: Vec<_> = parsed.iter().map(|(name, args)| {
                        let tc = crate::models::ToolCall { name: name.clone(), args: args.clone() };
                        tokio::spawn(async move { crate::agent::tools::dispatch_tool(&tc).await })
                    }).collect();
                    let mut ordered = Vec::new();
                    for (handle, (name, _)) in handles.into_iter().zip(parsed.iter()) {
                        let r = handle.await.unwrap_or_else(|_| crate::agent::tools::ToolResult::err(name, "spawn 오류"));
                        ordered.push(format!("툴 '{}' 결과:\n{}", name, r.output));
                    }
                    results = ordered;
                } else {
                    // Sequential execution (includes write tools)
                    let mut seq = Vec::new();
                    for (name, args) in &parsed {
                        on_progress(&format!("  🔧 [{}] {}...", role.name(), name));
                        let result = crate::agent::tools::dispatch_tool(
                            &crate::models::ToolCall { name: name.clone(), args: args.clone() }
                        ).await;
                        seq.push(format!("툴 '{}' 결과:\n{}", name, result.output));
                    }
                    results = seq;
                }

                tool_calls += parsed.len();
                history.push(Message::tool(results.join("\n\n")));
                if turn == role.max_turns() - 1 { final_output = results.join("\n\n"); }
            }
            crate::models::AgentResponse::ToolCall(tc) => {
                on_progress(&format!("  🔧 [{}] {}...", role.name(), tc.name));
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                tool_calls += 1;
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("툴 '{}' 결과:\n{}", tc.name, result.output)));
                if turn == role.max_turns() - 1 { final_output = result.output; }
            }
        }
    }

    on_progress(&format!("  {} {} 완료 (툴: {})", role.icon(), role.name(), tool_calls));
    let _ = hub.send(NodeMessage {
        from: role.name().to_string(), to: String::new(),
        msg_type: MsgType::Status,
        content: format!("{} [{}] 완료", role.name(), story.id),
        metadata: Default::default(),
    }).await;

    final_output
}

// ─── Lightweight agent (no tools, text only) ──────────────────────────────────────

pub async fn run_agent_simple(
    client: &OllamaClient,
    system: &str,
    task: &str,
    max_turns: usize,
    on_progress: &impl Fn(&str),
) -> String {
    let mut history = vec![Message::system(system), Message::user(task)];
    for turn in 0..max_turns {
        match client.chat_stream(history.clone(), |_| {}).await {
            Ok(text) => {
                if text.contains("TOOL:") && turn < max_turns - 1 {
                    match crate::agent::chat::parse_response_pub(&text) {
                        crate::models::AgentResponse::ToolCall(tc) => {
                            on_progress(&format!("  🔧 {}...", tc.name));
                            let result = crate::agent::tools::dispatch_tool(&tc).await;
                            history.push(Message::assistant(&text));
                            history.push(Message::tool(format!("툴 결과:\n{}", result.output)));
                            continue;
                        }
                        _ => return text,
                    }
                }
                return text;
            }
            Err(e) => {
                on_progress(&format!("오류: {}", e));
                return String::new();
            }
        }
    }
    String::new()
}

// ─── Standalone role execution (task only, no story) ─────────────────────────────────

pub async fn run_role_standalone(
    client: &OllamaClient,
    role: AgileRole,
    task: &str,
    context: &str,
    hub: &NodeHub,
    on_progress: &impl Fn(&str),
) -> String {
    // Create temporary story
    let mut story = crate::agile::story::UserStory::new(
        "STANDALONE", task, task,
        crate::agile::story::Priority::High, 3,
    );
    story.add_acceptance_criteria("태스크 완료");
    run_agile_agent(client, role, &story, context, hub, on_progress).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_parallel_tools_list_is_not_empty() {
        assert!(!SAFE_PARALLEL_TOOLS.is_empty());
    }

    #[test]
    fn read_only_tools_are_safe() {
        assert!(SAFE_PARALLEL_TOOLS.contains(&"read_file"));
        assert!(SAFE_PARALLEL_TOOLS.contains(&"web_search"));
        assert!(SAFE_PARALLEL_TOOLS.contains(&"grep"));
    }

    #[test]
    fn write_tools_are_not_safe() {
        assert!(!SAFE_PARALLEL_TOOLS.contains(&"write_file"));
        assert!(!SAFE_PARALLEL_TOOLS.contains(&"shell"));
        assert!(!SAFE_PARALLEL_TOOLS.contains(&"git_commit"));
    }
}
