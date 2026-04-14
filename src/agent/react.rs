//! Enhanced ReAct loop
#![allow(dead_code)]
//!
//! Standard ReAct: Reason → Act → Observe
//! Enhanced:  Reason → Plan → Act → Observe → Verify → Reflect → (repeat)
//!
//! Additional features:
//! - Auto-verify: check if each tool result matches expectations
//! - Reflection loop: change approach when the same error repeats
//! - TDD mode: write tests first, implement, then verify
//! - Impact analysis: assess blast radius before changing code

use anyhow::Result;
use crate::agent::ollama::OllamaClient;
use crate::agent::tools::dispatch_tool;
use crate::models::{AgentResponse, Message, ToolCall};

// ─── ReAct step ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReActStep {
    pub thought: String,
    pub action: Option<String>,     // tool name
    pub action_input: Vec<String>,  // tool arguments
    pub observation: String,        // tool execution result
    pub verification: VerifyResult, // verification result
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerifyResult {
    NotNeeded,
    Pass,
    Fail(String),  // failure reason
}

// ─── ReAct config ──────────────────────────────────────────────────────────────────────────

pub struct ReActConfig {
    pub max_turns: usize,
    pub max_retries_per_error: usize,  // max retries for the same error
    pub verify_enabled: bool,          // enable auto-verification
    pub tdd_mode: bool,                // TDD mode
    pub reflection_enabled: bool,      // enable reflection loop on failure
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_turns: 20,
            max_retries_per_error: 3,
            verify_enabled: true,
            tdd_mode: false,
            reflection_enabled: true,
        }
    }
}

// ─── ReAct execution result ──────────────────────────────────────────────────────────────

pub struct ReActResult {
    pub final_answer: String,
    pub steps: Vec<ReActStep>,
    pub retries: usize,
    pub success: bool,
}

// ─── Enhanced ReAct loop ───────────────────────────────────────────────────────────────────

pub async fn run_react(
    client: &OllamaClient,
    history: &mut Vec<Message>,
    config: &ReActConfig,
    on_step: impl Fn(&ReActStep),
) -> ReActResult {
    let mut steps = Vec::new();
    let mut error_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_retries = 0usize;
    let mut reflection_msgs: Vec<String> = Vec::new();

    // TDD mode: write tests first
    if config.tdd_mode {
        inject_tdd_instruction(history);
    }

    for turn in 0..config.max_turns {
        // inject reflection into history if available
        if !reflection_msgs.is_empty() {
            let reflection = format!(
                "⚠️ Previous attempt analysis:\n{}\n\nTry a different approach.",
                reflection_msgs.last().unwrap()
            );
            history.push(Message::tool(reflection));
            reflection_msgs.clear();
        }

        // generate AI response
        let ai_text = match client.chat_stream(history.clone(), |_| {}).await {
            Ok(t) => t,
            Err(e) => {
                return ReActResult {
                    final_answer: format!("AI error: {}", e),
                    steps,
                    retries: total_retries,
                    success: false,
                };
            }
        };

        match crate::agent::chat::parse_response_pub(&ai_text) {
            AgentResponse::Exit | AgentResponse::Text(_) => {
                // final answer
                let step = ReActStep {
                    thought: ai_text.clone(),
                    action: None,
                    action_input: vec![],
                    observation: String::new(),
                    verification: VerifyResult::NotNeeded,
                };
                on_step(&step);
                steps.push(step);
                history.push(Message::assistant(&ai_text));
                return ReActResult {
                    final_answer: ai_text,
                    steps,
                    retries: total_retries,
                    success: true,
                };
            }

            AgentResponse::ToolCall(tc) if tc.name == "__multi__" => {
                // handle multiple tools
                history.push(Message::assistant(&ai_text));
                let mut results = Vec::new();
                for raw in &tc.args {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
                    let name = val["name"].as_str().unwrap_or("").to_string();
                    let args: Vec<String> = val["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    let result = dispatch_tool(&ToolCall { name: name.clone(), args: args.clone() }).await;
                    results.push(format!("Tool '{}' result:\n{}", name, result.output));

                    let step = ReActStep {
                        thought: format!("Multi-tool: {}", name),
                        action: Some(name),
                        action_input: args,
                        observation: result.output,
                        verification: VerifyResult::NotNeeded,
                    };
                    on_step(&step);
                    steps.push(step);
                }
                history.push(Message::tool(results.join("\n\n")));
            }

            AgentResponse::ToolCall(tc) => {
                // check error frequency
                let err_key = tc.name.clone();
                let result = dispatch_tool(&tc).await;

                // verify
                let verification = if config.verify_enabled {
                    verify_result(&tc.name, &tc.args, &result.output, result.success)
                } else {
                    VerifyResult::NotNeeded
                };

                let step = ReActStep {
                    thought: format!("Turn {}: {}", turn + 1, ai_text.lines().next().unwrap_or("")),
                    action: Some(tc.name.clone()),
                    action_input: tc.args.clone(),
                    observation: result.output.clone(),
                    verification: verification.clone(),
                };
                on_step(&step);
                steps.push(step.clone());

                history.push(Message::assistant(&ai_text));

                // handle failure
                if !result.success {
                    let count = error_counts.entry(err_key).or_insert(0);
                    *count += 1;
                    total_retries += 1;

                    if *count >= config.max_retries_per_error && config.reflection_enabled {
                        // generate reflection message
                        let reflection = generate_reflection(&steps);
                        reflection_msgs.push(reflection.clone());
                        history.push(Message::tool(format!(
                            "Tool '{}' result:\n{}\n\n[Failure #{} — consider changing approach]",
                            tc.name, result.output, count
                        )));
                        *count = 0; // reset counter
                    } else {
                        history.push(Message::tool(format!(
                            "Tool '{}' result (failed):\n{}", tc.name, result.output
                        )));
                    }
                } else if matches!(verification, VerifyResult::Fail(_)) {
                    // verify failed: continue with warning
                    if let VerifyResult::Fail(ref reason) = verification {
                        history.push(Message::tool(format!(
                            "Tool '{}' result:\n{}\n\n⚠️ Verification warning: {}",
                            tc.name, result.output, reason
                        )));
                    }
                } else {
                    history.push(Message::tool(format!(
                        "Tool '{}' result:\n{}", tc.name, result.output
                    )));
                }
            }
        }
    }

    ReActResult {
        final_answer: "Max turns exceeded".to_string(),
        steps,
        retries: total_retries,
        success: false,
    }
}

// ─── Verification logic ────────────────────────────────────────────────────────────────────

/// Semantically verify a tool execution result
fn verify_result(
    tool_name: &str,
    args: &[String],
    output: &str,
    success: bool,
) -> VerifyResult {
    if !success {
        return VerifyResult::Fail(format!("Tool failed: {}", crate::utils::trunc(output, 100)));
    }

    match tool_name {
        // write_file → check the file exists
        "write_file" => {
            if let Some(path) = args.first() {
                if !std::path::Path::new(path).exists() {
                    return VerifyResult::Fail(format!("File was not created: {}", path));
                }
            }
            VerifyResult::Pass
        }

        // build → check for no errors in stderr
        "run_shell" => {
            let lower = output.to_lowercase();
            if lower.contains("error[") || lower.contains("error:") {
                // warnings are OK, only check for errors
                let has_real_error = output.lines()
                    .any(|l| l.trim_start().starts_with("error") && !l.contains("warning"));
                if has_real_error {
                    return VerifyResult::Fail("Build/run error detected".to_string());
                }
            }
            VerifyResult::Pass
        }

        // tests → check test results
        "run_tests" => {
            if output.contains("FAILED") || output.contains("failures:") {
                let failed: Vec<&str> = output.lines()
                    .filter(|l| l.contains("FAILED") || l.starts_with("test ") && l.ends_with("FAILED"))
                    .collect();
                return VerifyResult::Fail(format!(
                    "Test failures: {}",
                    crate::utils::trunc(&failed.join(", "), 200)
                ));
            }
            VerifyResult::Pass
        }

        // git commit → check for commit hash
        "git_commit" | "git_commit_all" => {
            if !output.contains('[') {
                return VerifyResult::Fail("Commit does not appear to have been created".to_string());
            }
            VerifyResult::Pass
        }

        _ => VerifyResult::NotNeeded,
    }
}

// ─── Reflection generation ────────────────────────────────────────────────────────────────

/// Analyze failed steps and generate a reflection message
fn generate_reflection(steps: &[ReActStep]) -> String {
    let failures: Vec<&ReActStep> = steps.iter()
        .filter(|s| matches!(s.verification, VerifyResult::Fail(_)))
        .collect();

    if failures.is_empty() {
        return "Repeated failures detected — try a different approach".to_string();
    }

    let failure_summary: Vec<String> = failures.iter()
        .take(3)
        .map(|s| format!(
            "- {} → {}",
            s.action.as_deref().unwrap_or("?"),
            crate::utils::trunc(&s.observation, 80)
        ))
        .collect();

    format!(
        "The following approaches failed:\n{}\n\nUse a completely different approach.",
        failure_summary.join("\n")
    )
}

// ─── TDD mode ──────────────────────────────────────────────────────────────────────────────

fn inject_tdd_instruction(history: &mut Vec<Message>) {
    let tdd_instruction = "\n\n=== TDD Mode ===\n\
        Follow this implementation order strictly:\n\
        1. Write a failing test first\n\
        2. Confirm the test fails (Red)\n\
        3. Write the minimum code to pass the test (Green)\n\
        4. Improve the code (Refactor)\n\
        5. Confirm all tests pass";

    if let Some(first) = history.first_mut() {
        if matches!(first.role, crate::models::Role::System) {
            first.content.push_str(tdd_instruction);
        }
    }
}

// ─── Impact analysis ───────────────────────────────────────────────────────────────────────

/// Analyze the impact of a file change before making it
pub async fn analyze_impact(
    client: &OllamaClient,
    file_path: &str,
    change_description: &str,
) -> Result<String> {
    // find places that import/use the file
    let filename = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path);

    let grep_result = crate::tools::grep_files(filename, ".")
        .unwrap_or_else(|_| vec![]);

    let affected_files: Vec<String> = grep_result.iter()
        .map(|line| format!("  • {}", crate::utils::trunc(line, 80)))
        .take(20)
        .collect();

    if affected_files.is_empty() {
        return Ok(format!("No references found for '{}'.", filename));
    }

    // request impact analysis from AI
    let prompt = format!(
        "Changing the following file: `{}`\n\
         Change description: {}\n\n\
         Places that reference this file:\n{}\n\n\
         Briefly analyze the impact of this change (under 200 chars).",
        file_path,
        change_description,
        affected_files.join("\n")
    );

    let msgs = vec![
        Message::system("You are an expert in code impact analysis."),
        Message::user(&prompt),
    ];

    let result = client.chat(msgs).await
        .map(|r| r.message.content)
        .unwrap_or_else(|_| format!("{} reference(s) found", affected_files.len()));

    Ok(format!(
        "Impact scope ({} location(s)):\n{}\n\nAnalysis:\n{}",
        affected_files.len(),
        affected_files.join("\n"),
        result
    ))
}
