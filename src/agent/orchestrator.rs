//! Multi-agent orchestrator
#![allow(dead_code)]
//!
//! Agent roles:
//!   Planner   → task decomposition and planning
//!   Developer → code implementation
//!   Debugger  → testing, verification, and bug fixing
//!   Reviewer  → code review and quality verification
//!
//! Pipeline:
//!   User request → Planner → Developer → Debugger → (Reviewer) → result

use anyhow::Result;
use crate::agent::ollama::OllamaClient;
use crate::agent::tools::dispatch_tool;
use crate::models::{AgentResponse, Message, ToolCall};

// ─── Agent roles ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentRole {
    General,
    Planner,
    Developer,
    Debugger,
    Reviewer,
}

impl AgentRole {
    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentRole::General => "You are a full-stack AI agent.",
            AgentRole::Planner => {
                "You are a software planning expert.\n\
                 Analyze the given task and produce:\n\
                 1. Task objective and scope\n\
                 2. Tech stack and architecture decisions\n\
                 3. Step-by-step implementation plan (with estimated time per step)\n\
                 4. Dependencies and risks\n\
                 5. Definition of Done\n\n\
                 Output format: JSON\n\
                 {\n\
                   \"objective\": \"task objective\",\n\
                   \"stack\": [\"tech1\", ...],\n\
                   \"steps\": [{\"id\": 1, \"title\": \"title\", \"description\": \"description\", \"files\": [\"file\"]}, ...],\n\
                   \"risks\": [\"risk1\", ...],\n\
                   \"done_criteria\": [\"criterion1\", ...]\n\
                 }"
            }
            AgentRole::Developer => {
                "You are a senior software engineer.\n\
                 Write actual code following the given implementation plan.\n\
                 Principles:\n\
                 - Thorough error handling (exceptions, None, boundary values)\n\
                 - Include unit tests\n\
                 - Comment complex logic\n\
                 - Prevent security vulnerabilities\n\
                 - Consider performance\n\n\
                 Write the files and verify the build."
            }
            AgentRole::Debugger => {
                "You are a debugging and testing expert.\n\
                 Verify the given implementation in this order:\n\
                 1. Static code analysis (lint/check)\n\
                 2. Build verification\n\
                 3. Run tests\n\
                 4. When a bug is found → root cause analysis → fix → re-verify\n\
                 5. Check edge cases\n\n\
                 Always re-verify after making fixes."
            }
            AgentRole::Reviewer => {
                "You are a code review expert.\n\
                 Systematically review the following items:\n\
                 1. Correctness: whether requirements are met\n\
                 2. Safety: security vulnerabilities, error handling\n\
                 3. Performance: algorithm efficiency, unnecessary operations\n\
                 4. Maintainability: readability, duplicate code\n\
                 5. Test coverage\n\n\
                 Provide a score (1-5) and specific feedback for each item."
            }
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AgentRole::General   => "🤖",
            AgentRole::Planner   => "📋",
            AgentRole::Developer => "💻",
            AgentRole::Debugger  => "🔍",
            AgentRole::Reviewer  => "👁️",
        }
    }
}

// ─── Agent execution result ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub role: AgentRole,
    pub content: String,
    pub tool_calls_made: usize,
    pub success: bool,
}

/// Full pipeline result
#[derive(Debug)]
pub struct PipelineResult {
    pub plan: String,
    pub implementation: String,
    pub verification: String,
    pub review: Option<String>,
    pub success: bool,
}

// ─── Single agent execution ─────────────────────────────────────────────────────────────────

pub async fn run_agent(
    client: &OllamaClient,
    role: AgentRole,
    task: &str,
    context: &str,  // additional context such as previous agent output
    max_turns: usize,
    on_progress: impl Fn(&str),
) -> AgentOutput {
    let system = format!(
        "Model: {}\n\n{}\n\n{}",
        client.model(),
        crate::agent::tools::tool_descriptions(),
        role.system_prompt()
    );

    let user_content = if context.is_empty() {
        task.to_string()
    } else {
        format!("{}\n\n## Context\n{}", task, context)
    };

    let mut history = vec![
        Message::system(&system),
        Message::user(&user_content),
    ];

    on_progress(&format!("{} {} agent starting...", role.icon(), format!("{:?}", role)));

    let mut tool_calls = 0usize;
    let mut final_output = String::new();

    for turn in 0..max_turns {
        let ai_text = match client.chat_stream(history.clone(), |tok| {
            // streaming progress differs for Discord vs console
            let _ = tok;
        }).await {
            Ok(t) => t,
            Err(e) => {
                return AgentOutput {
                    role,
                    content: format!("Agent error: {}", e),
                    tool_calls_made: tool_calls,
                    success: false,
                };
            }
        };

        match crate::agent::chat::parse_response_pub(&ai_text) {
            AgentResponse::Exit => break,

            AgentResponse::Text(_) => {
                final_output = ai_text.clone();
                history.push(Message::assistant(&ai_text));
                break;
            }

            AgentResponse::ToolCall(tc) if tc.name == "__multi__" => {
                history.push(Message::assistant(&ai_text));
                let mut results = Vec::new();
                for raw in &tc.args {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
                    let name = val["name"].as_str().unwrap_or("").to_string();
                    let args: Vec<String> = val["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    on_progress(&format!("  🔧 {}...", name));
                    let result = dispatch_tool(&ToolCall { name: name.clone(), args }).await;
                    results.push(format!("Tool '{}' result:\n{}", name, result.output));
                    tool_calls += 1;
                }
                history.push(Message::tool(results.join("\n\n")));

                // collect result on the last turn
                if turn == max_turns - 1 {
                    final_output = results.join("\n\n");
                }
            }

            AgentResponse::ToolCall(tc) => {
                on_progress(&format!("  🔧 {}...", tc.name));
                let result = dispatch_tool(&tc).await;
                tool_calls += 1;

                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("Tool '{}' result:\n{}", tc.name, result.output)));

                if turn == max_turns - 1 {
                    final_output = result.output;
                }
            }
        }
    }

    on_progress(&format!("{} {:?} complete (tools: {} call(s))", role.icon(), role, tool_calls));

    AgentOutput {
        role,
        content: final_output,
        tool_calls_made: tool_calls,
        success: true,
    }
}

// ─── Pipeline orchestrator ─────────────────────────────────────────────────────────────────

/// Execute the full planning → development → debugging pipeline
pub async fn run_pipeline(
    client: &OllamaClient,
    task: &str,
) -> Result<PipelineResult> {
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║    Multi-agent pipeline starting             ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("Task: {}\n", crate::utils::trunc(task, 100));

    // ── Step 1: Planner ────────────────────────────────────────────────────
    let plan_output = run_agent(
        client,
        AgentRole::Planner,
        task,
        "",
        8,
        |msg| println!("[Planner] {}", msg),
    ).await;

    // attempt to parse plan as JSON, fall back to raw text
    let plan_text = plan_output.content.clone();
    let plan_summary = extract_plan_summary(&plan_text);

    println!("\n📋 Plan complete:\n{}\n", crate::utils::trunc(&plan_summary, 300));

    // ── Step 2: Developer ─────────────────────────────────────────────────
    let dev_context = format!(
        "Implement according to the following plan:\n\n{}",
        crate::utils::trunc(&plan_text, 2000)
    );

    let dev_output = run_agent(
        client,
        AgentRole::Developer,
        task,
        &dev_context,
        15,
        |msg| println!("[Developer] {}", msg),
    ).await;

    println!("\n💻 Implementation complete (tools used: {})\n", dev_output.tool_calls_made);

    // ── Step 3: Debugger ─────────────────────────────────────────────────
    let debug_context = format!(
        "Verify the following implementation and fix any bugs:\n\nPlan:\n{}\n\nImplementation:\n{}",
        crate::utils::trunc(&plan_text, 1000),
        crate::utils::trunc(&dev_output.content, 1000)
    );

    let debug_output = run_agent(
        client,
        AgentRole::Debugger,
        task,
        &debug_context,
        12,
        |msg| println!("[Debugger] {}", msg),
    ).await;

    println!("\n🔍 Verification complete (tools used: {})\n", debug_output.tool_calls_made);

    // ── Step 4: Reviewer (optional, only when many tool calls were made) ──
    let review = if dev_output.tool_calls_made + debug_output.tool_calls_made > 3 {
        let review_context = format!(
            "Plan:\n{}\n\nImplementation result:\n{}\n\nVerification result:\n{}",
            crate::utils::trunc(&plan_text, 800),
            crate::utils::trunc(&dev_output.content, 800),
            crate::utils::trunc(&debug_output.content, 600),
        );

        let review_output = run_agent(
            client,
            AgentRole::Reviewer,
            task,
            &review_context,
            6,
            |msg| println!("[Reviewer] {}", msg),
        ).await;

        println!("\n👁️ Review complete\n");
        Some(review_output.content)
    } else {
        None
    };

    // ── print results ────────────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════╗");
    println!("║    Pipeline complete                         ║");
    println!("╚══════════════════════════════════════════════╝\n");

    Ok(PipelineResult {
        plan: plan_text,
        implementation: dev_output.content,
        verification: debug_output.content,
        review,
        success: true,
    })
}

/// Run agents in parallel (independent tasks)
pub async fn run_parallel_agents(
    _client: &OllamaClient,
    tasks: Vec<(AgentRole, String)>,
) -> Vec<AgentOutput> {
    use tokio::task::JoinSet;

    let mut set = JoinSet::new();

    for (role, task) in tasks {
        let client_clone = OllamaClient::new(
            std::env::var("OLLAMA_API_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gemma4:e4b".to_string()),
        );
        set.spawn(async move {
            run_agent(&client_clone, role, &task, "", 10, |msg| println!("[Parallel] {}", msg)).await
        });
    }

    let mut results = Vec::new();
    while let Some(r) = set.join_next().await {
        if let Ok(output) = r {
            results.push(output);
        }
    }
    results
}

// ─── Plan parsing helpers ───────────────────────────────────────────────────────────────────

fn extract_plan_summary(plan_text: &str) -> String {
    // attempt JSON parsing
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(plan_text) {
        let objective = v["objective"].as_str().unwrap_or("").to_string();
        let steps: Vec<String> = v["steps"].as_array()
            .map(|arr| arr.iter()
                .filter_map(|s| s["title"].as_str().map(|t| format!("  • {}", t)))
                .collect())
            .unwrap_or_default();
        if !objective.is_empty() {
            return format!("Objective: {}\nSteps:\n{}", objective, steps.join("\n"));
        }
    }

    // no JSON: return first 500 chars of raw text
    crate::utils::trunc(plan_text, 500).to_string()
}
