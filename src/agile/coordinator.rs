//! Coordinator multi-agent mode
//!
//! Claude Code architecture reference:
//!   - Leader agent decomposes tasks into subtasks
//!   - Worker agents run in parallel via tokio::spawn
//!   - Results are synthesized into a final report
//!
//! Flow:
//!   Coordinator  → task decomposition (N subtasks)
//!   Worker × N   → parallel execution (independent contexts)
//!   Coordinator  → result synthesis + final output

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::agent::ollama::OllamaClient;
use crate::agile::runner::run_agent_simple;

// ─── Data structures ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub role: String,     // assigned role hint
    pub priority: u8,     // 1=high
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub subtask_id: String,
    pub role: String,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorResult {
    pub task: String,
    pub subtasks: Vec<SubTask>,
    pub worker_results: Vec<WorkerResult>,
    pub synthesis: String,
    pub total_workers: usize,
    pub elapsed_hint: String,
}

impl CoordinatorResult {
    pub fn render(&self) -> String {
        let mut out = vec![
            format!("\n╔══════════════════════════════════════════════════╗"),
            format!("║  🤝 Coordinator Result                            ║"),
            format!("╚══════════════════════════════════════════════════╝"),
            format!("Task: {}", crate::utils::trunc(&self.task, 60)),
            format!("Workers: {} running in parallel\n", self.total_workers),
            format!("── Subtask results ──"),
        ];

        for r in &self.worker_results {
            let status = if r.success { "✅" } else { "⚠️" };
            out.push(format!("  {} [{}] {}: {}",
                status, r.subtask_id, r.role,
                crate::utils::trunc(&r.output, 100)));
        }

        out.push(format!("\n── Synthesis ──"));
        out.push(self.synthesis.clone());
        out.push(String::new());
        out.join("\n")
    }
}

// ─── Pipeline ─────────────────────────────────────────────────────────────

const MAX_PARALLEL_WORKERS: usize = 8;

pub async fn run_coordinator(
    client: &OllamaClient,
    task: &str,
    on_progress: impl Fn(&str) + Clone + Send + 'static,
) -> Result<CoordinatorResult> {
    on_progress(&format!("\n🤝 ══ Coordinator starting ══\nTask: {}", crate::utils::trunc(task, 80)));

    // ── Step 1: leader decomposes task into subtasks ─────────────────────────
    on_progress("🎯 Leader: analyzing task and decomposing into subtasks...");

    let decompose_prompt = format!(
        "You are the leader agent. Decompose the following task into independent subtasks that can be processed in parallel.\n\
         Each subtask must be independent (no ordering dependencies). Create 3–{} subtasks.\n\n\
         Task: {}\n\n\
         JSON output:\n\
         [{{\"id\": \"ST-1\", \"title\": \"...\", \"description\": \"...\", \"role\": \"Developer\", \"priority\": 1}}, ...]",
        MAX_PARALLEL_WORKERS, task
    );

    let system = format!("Model: {}\n\nYou are an expert Coordinator who decomposes complex tasks into parallel subtasks.", client.model());
    let decompose_output = run_agent_simple(client, &system, &decompose_prompt, 3, &on_progress).await;

    let subtasks = parse_subtasks(&decompose_output);
    if subtasks.is_empty() {
        // Fall back to single task on decomposition failure
        on_progress("⚠️ Subtask decomposition failed — falling back to single execution");
        let output = run_agent_simple(client, &system, task, 5, &on_progress).await;
        return Ok(CoordinatorResult {
            task: task.to_string(),
            subtasks: vec![],
            worker_results: vec![WorkerResult {
                subtask_id: "ST-1".to_string(),
                role: "General".to_string(),
                output: output.clone(),
                success: true,
            }],
            synthesis: output,
            total_workers: 1,
            elapsed_hint: "single execution".to_string(),
        });
    }

    on_progress(&format!("✅ {} subtask(s) created", subtasks.len()));
    for st in &subtasks {
        on_progress(&format!("  📋 [{}] {} (role: {})", st.id, st.title, st.role));
    }

    // ── Step 2: run workers in parallel ───────────────────────────────────────
    on_progress(&format!("\n⚡ Starting {} worker(s) in parallel...", subtasks.len()));

    // Share via Arc (tokio::spawn requires 'static)
    use std::sync::Arc;
    let client_arc = Arc::new(OllamaClient::new(
        std::env::var("OLLAMA_API_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
        client.model().to_string(),
    ));

    // Collect progress messages via channel (tokio::spawn requires Send)
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let handles: Vec<_> = subtasks.iter().map(|st| {
        let client_clone = Arc::clone(&client_arc);
        let st_clone = st.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            let _ = tx_clone.send(format!("  🔨 Worker [{}] {} starting...", st_clone.id, st_clone.title));

            let worker_system = format!(
                "Model: {}\n\nYou are an expert in the {} role. Complete the given task thoroughly.",
                client_clone.model(), st_clone.role
            );
            let worker_prompt = format!(
                "## Subtask [{}]: {}\n\n{}\n\nProvide results that are as specific and actionable as possible.",
                st_clone.id, st_clone.title, st_clone.description
            );

            let tx2 = tx_clone.clone();
            let output = run_agent_simple(&client_clone, &worker_system, &worker_prompt, 8,
                &move |msg: &str| { let _ = tx2.send(msg.to_string()); }).await;
            let success = !output.is_empty() && !output.starts_with("Error:");

            let _ = tx_clone.send(format!("  {} Worker [{}] complete",
                if success { "✅" } else { "⚠️" }, st_clone.id));

            WorkerResult {
                subtask_id: st_clone.id.clone(),
                role: st_clone.role.clone(),
                output,
                success,
            }
        })
    }).collect();

    // Close channel (drop sender)
    drop(tx);

    // Stream progress messages in real time (async)
    tokio::spawn({
        let on_prog = on_progress.clone();
        async move {
            while let Some(msg) = rx.recv().await {
                on_prog(&msg);
            }
        }
    });

    // Collect all worker results
    let mut worker_results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(result) => worker_results.push(result),
            Err(e) => worker_results.push(WorkerResult {
                subtask_id: "ERR".to_string(),
                role: "Worker".to_string(),
                output: format!("Worker panic: {}", e),
                success: false,
            }),
        }
    }

    let success_count = worker_results.iter().filter(|r| r.success).count();
    on_progress(&format!("📊 Workers complete: {}/{} succeeded", success_count, worker_results.len()));

    // ── Step 3: leader synthesizes results ──────────────────────────────────────
    on_progress("🎯 Leader: synthesizing results...");

    let results_text: String = worker_results.iter().map(|r| {
        format!("=== [{}] {} ===\n{}", r.subtask_id, r.role, crate::utils::trunc(&r.output, 800))
    }).collect::<Vec<_>>().join("\n\n");

    let synthesis_prompt = format!(
        "Here are the results from workers run in parallel. Synthesize them into a coherent final result.\n\n\
         Original task: {}\n\n\
         Worker results:\n{}\n\n\
         Remove duplicates, resolve contradictions, select the best approach, and present an integrated solution.",
        task, results_text
    );

    let synthesis_system = format!(
        "Model: {}\n\nYou are a senior Coordinator who synthesizes the results of multiple agents.", client.model()
    );
    let synthesis = run_agent_simple(client, &synthesis_system, &synthesis_prompt, 4, &on_progress).await;

    let result = CoordinatorResult {
        task: task.to_string(),
        subtasks: subtasks.clone(),
        worker_results,
        synthesis,
        total_workers: subtasks.len(),
        elapsed_hint: format!("{} parallel", subtasks.len()),
    };

    on_progress(&result.render());
    Ok(result)
}

// ─── Parsing ────────────────────────────────────────────────────────────────────

fn parse_subtasks(text: &str) -> Vec<SubTask> {
    let candidate = if let Some(s) = text.find("```json") {
        let after = &text[s + 7..];
        if let Some(e) = after.find("```") { &after[..e] } else { after }
    } else if let Some(s) = text.find('[') {
        if let Some(e) = text.rfind(']') { &text[s..=e] } else { return vec![] }
    } else { return vec![]; };

    let arr: Vec<serde_json::Value> = serde_json::from_str(candidate.trim()).unwrap_or_default();
    arr.iter().map(|v| SubTask {
        id: v["id"].as_str().unwrap_or("ST-?").to_string(),
        title: v["title"].as_str().unwrap_or("").to_string(),
        description: v["description"].as_str().unwrap_or("").to_string(),
        role: v["role"].as_str().unwrap_or("Developer").to_string(),
        priority: v["priority"].as_u64().unwrap_or(2) as u8,
    }).filter(|st| !st.title.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subtasks_valid_json() {
        let text = r#"[{"id":"ST-1","title":"Backend","description":"Implement API","role":"Developer","priority":1}]"#;
        let tasks = parse_subtasks(text);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "ST-1");
        assert_eq!(tasks[0].role, "Developer");
    }

    #[test]
    fn test_parse_subtasks_empty_title_filtered() {
        let text = r#"[{"id":"ST-1","title":"","description":"none","role":"Dev","priority":1}]"#;
        let tasks = parse_subtasks(text);
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_subtasks_invalid_returns_empty() {
        let tasks = parse_subtasks("no JSON here");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_coordinator_result_render() {
        let result = CoordinatorResult {
            task: "test task".to_string(),
            subtasks: vec![],
            worker_results: vec![WorkerResult {
                subtask_id: "ST-1".to_string(),
                role: "Developer".to_string(),
                output: "Implementation complete".to_string(),
                success: true,
            }],
            synthesis: "Synthesis result".to_string(),
            total_workers: 1,
            elapsed_hint: "1 parallel".to_string(),
        };
        let rendered = result.render();
        assert!(rendered.contains("Coordinator"));
        assert!(rendered.contains("ST-1"));
    }
}
