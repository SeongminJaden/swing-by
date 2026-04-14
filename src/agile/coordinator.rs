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
    pub role: String,     // 담당 역할 힌트
    pub priority: u8,     // 1=높음
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
            format!("║  🤝 Coordinator 결과                              ║"),
            format!("╚══════════════════════════════════════════════════╝"),
            format!("태스크: {}", crate::utils::trunc(&self.task, 60)),
            format!("워커: {} 명 병렬 실행\n", self.total_workers),
            format!("── 서브태스크 결과 ──"),
        ];

        for r in &self.worker_results {
            let status = if r.success { "✅" } else { "⚠️" };
            out.push(format!("  {} [{}] {}: {}",
                status, r.subtask_id, r.role,
                crate::utils::trunc(&r.output, 100)));
        }

        out.push(format!("\n── 종합 결과 ──"));
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
    on_progress(&format!("\n🤝 ══ Coordinator 시작 ══\n태스크: {}", crate::utils::trunc(task, 80)));

    // ── 1단계: 리더가 태스크를 서브태스크로 분해 ────────────────────────────
    on_progress("🎯 리더: 태스크 분석 및 서브태스크 분해 중...");

    let decompose_prompt = format!(
        "당신은 리더 에이전트입니다. 다음 태스크를 병렬로 처리할 수 있는 독립적인 서브태스크로 분해하세요.\n\
         각 서브태스크는 독립적이어야 하며 (순서 의존성 없음), 3~{} 개로 나누세요.\n\n\
         태스크: {}\n\n\
         JSON 출력:\n\
         [{{\"id\": \"ST-1\", \"title\": \"...\", \"description\": \"...\", \"role\": \"Developer\", \"priority\": 1}}, ...]",
        MAX_PARALLEL_WORKERS, task
    );

    let system = format!("모델: {}\n\n당신은 복잡한 태스크를 병렬 서브태스크로 분해하는 전문 Coordinator입니다.", client.model());
    let decompose_output = run_agent_simple(client, &system, &decompose_prompt, 3, &on_progress).await;

    let subtasks = parse_subtasks(&decompose_output);
    if subtasks.is_empty() {
        // Fall back to single task on decomposition failure
        on_progress("⚠️ 서브태스크 분해 실패 — 단일 실행으로 대체");
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
            elapsed_hint: "단일 실행".to_string(),
        });
    }

    on_progress(&format!("✅ 서브태스크 {} 개 생성", subtasks.len()));
    for st in &subtasks {
        on_progress(&format!("  📋 [{}] {} (담당: {})", st.id, st.title, st.role));
    }

    // ── 2단계: 워커 병렬 실행 ────────────────────────────────────────────────
    on_progress(&format!("\n⚡ {} 개 워커 병렬 실행 시작...", subtasks.len()));

    // Arc로 공유 (tokio::spawn은 'static 요구)
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
            let _ = tx_clone.send(format!("  🔨 워커 [{}] {} 시작...", st_clone.id, st_clone.title));

            let worker_system = format!(
                "모델: {}\n\n당신은 {} 역할의 전문가입니다. 주어진 태스크를 완전하게 처리하세요.",
                client_clone.model(), st_clone.role
            );
            let worker_prompt = format!(
                "## 서브태스크 [{}]: {}\n\n{}\n\n최대한 구체적이고 실행 가능한 결과를 제공하세요.",
                st_clone.id, st_clone.title, st_clone.description
            );

            let tx2 = tx_clone.clone();
            let output = run_agent_simple(&client_clone, &worker_system, &worker_prompt, 8,
                &move |msg: &str| { let _ = tx2.send(msg.to_string()); }).await;
            let success = !output.is_empty() && !output.starts_with("오류:");

            let _ = tx_clone.send(format!("  {} 워커 [{}] 완료",
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

    // 모든 워커 결과 수집
    let mut worker_results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(result) => worker_results.push(result),
            Err(e) => worker_results.push(WorkerResult {
                subtask_id: "ERR".to_string(),
                role: "Worker".to_string(),
                output: format!("워커 패닉: {}", e),
                success: false,
            }),
        }
    }

    let success_count = worker_results.iter().filter(|r| r.success).count();
    on_progress(&format!("📊 워커 완료: {}/{} 성공", success_count, worker_results.len()));

    // ── 3단계: 리더가 결과 종합 ──────────────────────────────────────────────
    on_progress("🎯 리더: 결과 종합 중...");

    let results_text: String = worker_results.iter().map(|r| {
        format!("=== [{}] {} ===\n{}", r.subtask_id, r.role, crate::utils::trunc(&r.output, 800))
    }).collect::<Vec<_>>().join("\n\n");

    let synthesis_prompt = format!(
        "다음은 병렬 실행된 워커들의 결과입니다. 이를 종합하여 일관성 있는 최종 결과를 작성하세요.\n\n\
         원래 태스크: {}\n\n\
         워커 결과:\n{}\n\n\
         중복 제거, 모순 해결, 최선의 접근법 선택하여 통합된 솔루션을 제시하세요.",
        task, results_text
    );

    let synthesis_system = format!(
        "모델: {}\n\n당신은 여러 에이전트의 결과를 종합하는 수석 Coordinator입니다.", client.model()
    );
    let synthesis = run_agent_simple(client, &synthesis_system, &synthesis_prompt, 4, &on_progress).await;

    let result = CoordinatorResult {
        task: task.to_string(),
        subtasks: subtasks.clone(),
        worker_results,
        synthesis,
        total_workers: subtasks.len(),
        elapsed_hint: format!("{}개 병렬", subtasks.len()),
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
        let text = r#"[{"id":"ST-1","title":"백엔드","description":"API 구현","role":"Developer","priority":1}]"#;
        let tasks = parse_subtasks(text);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "ST-1");
        assert_eq!(tasks[0].role, "Developer");
    }

    #[test]
    fn test_parse_subtasks_empty_title_filtered() {
        let text = r#"[{"id":"ST-1","title":"","description":"없음","role":"Dev","priority":1}]"#;
        let tasks = parse_subtasks(text);
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_subtasks_invalid_returns_empty() {
        let tasks = parse_subtasks("아무 JSON도 없는 텍스트");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_coordinator_result_render() {
        let result = CoordinatorResult {
            task: "테스트 태스크".to_string(),
            subtasks: vec![],
            worker_results: vec![WorkerResult {
                subtask_id: "ST-1".to_string(),
                role: "Developer".to_string(),
                output: "구현 완료".to_string(),
                success: true,
            }],
            synthesis: "종합 결과".to_string(),
            total_workers: 1,
            elapsed_hint: "1개 병렬".to_string(),
        };
        let rendered = result.render();
        assert!(rendered.contains("Coordinator"));
        assert!(rendered.contains("ST-1"));
    }
}
