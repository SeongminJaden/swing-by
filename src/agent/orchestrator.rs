//! 멀티에이전트 오케스트레이터
#![allow(dead_code)]
//!
//! 에이전트 역할:
//!   Planner   → 태스크 분해 및 계획 수립
//!   Developer → 코드 구현
//!   Debugger  → 테스트 및 검증, 버그 수정
//!   Reviewer  → 코드 리뷰 및 품질 검증
//!
//! Pipeline:
//!   사용자 요청 → Planner → Developer → Debugger → (Reviewer) → 결과

use anyhow::Result;
use crate::agent::ollama::OllamaClient;
use crate::agent::tools::dispatch_tool;
use crate::models::{AgentResponse, Message, ToolCall};

// ─── 에이전트 역할 ────────────────────────────────────────────────────────────

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
            AgentRole::General => "당신은 풀스택 AI 에이전트입니다.",
            AgentRole::Planner => {
                "당신은 소프트웨어 기획 전문가입니다.\n\
                 주어진 작업을 분석하여 다음을 작성하세요:\n\
                 1. 작업 목표 및 범위\n\
                 2. 기술 스택 및 아키텍처 결정\n\
                 3. 단계별 구현 계획 (각 단계에 예상 시간)\n\
                 4. 의존성 및 위험 요소\n\
                 5. 완료 기준 (Definition of Done)\n\n\
                 출력 형식: JSON\n\
                 {\n\
                   \"objective\": \"작업 목표\",\n\
                   \"stack\": [\"기술1\", ...],\n\
                   \"steps\": [{\"id\": 1, \"title\": \"제목\", \"description\": \"설명\", \"files\": [\"파일\"]}, ...],\n\
                   \"risks\": [\"위험1\", ...],\n\
                   \"done_criteria\": [\"기준1\", ...]\n\
                 }"
            }
            AgentRole::Developer => {
                "당신은 시니어 소프트웨어 엔지니어입니다.\n\
                 주어진 구현 계획에 따라 실제 코드를 작성하세요.\n\
                 원칙:\n\
                 - 에러 처리 철저히 (예외, None, 경계값)\n\
                 - 단위 테스트 포함\n\
                 - 복잡한 로직에 주석\n\
                 - 보안 취약점 방지\n\
                 - 성능 고려\n\n\
                 파일을 실제로 작성하고 빌드를 확인하세요."
            }
            AgentRole::Debugger => {
                "당신은 디버깅 및 테스트 전문가입니다.\n\
                 주어진 구현을 다음 순서로 검증하세요:\n\
                 1. 코드 정적 분석 (lint/check)\n\
                 2. 빌드 확인\n\
                 3. 테스트 실행\n\
                 4. 버그 발견 시 → 근본 원인 분석 → 수정 → 재검증\n\
                 5. 엣지 케이스 확인\n\n\
                 수정 후에는 반드시 재검증하세요."
            }
            AgentRole::Reviewer => {
                "당신은 코드 리뷰 전문가입니다.\n\
                 다음 항목을 체계적으로 검토하세요:\n\
                 1. 정확성: 요구사항 충족 여부\n\
                 2. 안전성: 보안 취약점, 에러 처리\n\
                 3. 성능: 알고리즘 효율, 불필요한 연산\n\
                 4. 유지보수성: 가독성, 중복 코드\n\
                 5. 테스트 커버리지\n\n\
                 각 항목에 점수(1-5)와 구체적 피드백을 제공하세요."
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

// ─── 에이전트 실행 결과 ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub role: AgentRole,
    pub content: String,
    pub tool_calls_made: usize,
    pub success: bool,
}

/// Pipeline 전체 결과
#[derive(Debug)]
pub struct PipelineResult {
    pub plan: String,
    pub implementation: String,
    pub verification: String,
    pub review: Option<String>,
    pub success: bool,
}

// ─── 단일 에이전트 실행 ───────────────────────────────────────────────────────

pub async fn run_agent(
    client: &OllamaClient,
    role: AgentRole,
    task: &str,
    context: &str,  // 이전 에이전트 출력 등 추가 컨텍스트
    max_turns: usize,
    on_progress: impl Fn(&str),
) -> AgentOutput {
    let system = format!(
        "모델: {}\n\n{}\n\n{}",
        client.model(),
        crate::agent::tools::tool_descriptions(),
        role.system_prompt()
    );

    let user_content = if context.is_empty() {
        task.to_string()
    } else {
        format!("{}\n\n## 컨텍스트\n{}", task, context)
    };

    let mut history = vec![
        Message::system(&system),
        Message::user(&user_content),
    ];

    on_progress(&format!("{} {} 에이전트 시작...", role.icon(), format!("{:?}", role)));

    let mut tool_calls = 0usize;
    let mut final_output = String::new();

    for turn in 0..max_turns {
        let ai_text = match client.chat_stream(history.clone(), |tok| {
            // 진행상황 스트리밍은 Discord/콘솔에 따라 다르게 처리
            let _ = tok;
        }).await {
            Ok(t) => t,
            Err(e) => {
                return AgentOutput {
                    role,
                    content: format!("에이전트 오류: {}", e),
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
                    results.push(format!("툴 '{}' 결과:\n{}", name, result.output));
                    tool_calls += 1;
                }
                history.push(Message::tool(results.join("\n\n")));

                // 마지막 턴에서는 결과 수집
                if turn == max_turns - 1 {
                    final_output = results.join("\n\n");
                }
            }

            AgentResponse::ToolCall(tc) => {
                on_progress(&format!("  🔧 {}...", tc.name));
                let result = dispatch_tool(&tc).await;
                tool_calls += 1;

                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("툴 '{}' 결과:\n{}", tc.name, result.output)));

                if turn == max_turns - 1 {
                    final_output = result.output;
                }
            }
        }
    }

    on_progress(&format!("{} {:?} 완료 (툴: {}회)", role.icon(), role, tool_calls));

    AgentOutput {
        role,
        content: final_output,
        tool_calls_made: tool_calls,
        success: true,
    }
}

// ─── Pipeline 오케스트레이터 ────────────────────────────────────────────────

/// 기획 → 개발 → 디버깅 전체 Pipeline 실행
pub async fn run_pipeline(
    client: &OllamaClient,
    task: &str,
) -> Result<PipelineResult> {
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║    멀티에이전트 파이프라인 시작              ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("작업: {}\n", crate::utils::trunc(task, 100));

    // ── 1단계: Planner ────────────────────────────────────────────
    let plan_output = run_agent(
        client,
        AgentRole::Planner,
        task,
        "",
        8,
        |msg| println!("[Planner] {}", msg),
    ).await;

    // 계획을 JSON Parsing 시도, 실패시 raw 사용
    let plan_text = plan_output.content.clone();
    let plan_summary = extract_plan_summary(&plan_text);

    println!("\n📋 계획 완료:\n{}\n", crate::utils::trunc(&plan_summary, 300));

    // ── 2단계: Developer ──────────────────────────────────────────
    let dev_context = format!(
        "다음 계획에 따라 구현하세요:\n\n{}",
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

    println!("\n💻 구현 완료 (툴 {}회)\n", dev_output.tool_calls_made);

    // ── 3단계: Debugger ───────────────────────────────────────────
    let debug_context = format!(
        "다음 구현을 검증하고 버그를 수정하세요:\n\n계획:\n{}\n\n구현:\n{}",
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

    println!("\n🔍 검증 완료 (툴 {}회)\n", debug_output.tool_calls_made);

    // ── 4단계: Reviewer (선택, 툴 호출이 많았을 때만) ─────────────
    let review = if dev_output.tool_calls_made + debug_output.tool_calls_made > 3 {
        let review_context = format!(
            "계획:\n{}\n\n구현 결과:\n{}\n\n검증 결과:\n{}",
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

        println!("\n👁️ 리뷰 완료\n");
        Some(review_output.content)
    } else {
        None
    };

    // ── 결과 출력 ─────────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════╗");
    println!("║    파이프라인 완료                           ║");
    println!("╚══════════════════════════════════════════════╝\n");

    Ok(PipelineResult {
        plan: plan_text,
        implementation: dev_output.content,
        verification: debug_output.content,
        review,
        success: true,
    })
}

/// 병렬 에이전트 실행 (독립적 태스크들)
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
            run_agent(&client_clone, role, &task, "", 10, |msg| println!("[병렬] {}", msg)).await
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

// ─── 계획 Parsing Helpers ──────────────────────────────────────────────────────────

fn extract_plan_summary(plan_text: &str) -> String {
    // JSON 형식 Parsing 시도
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(plan_text) {
        let objective = v["objective"].as_str().unwrap_or("").to_string();
        let steps: Vec<String> = v["steps"].as_array()
            .map(|arr| arr.iter()
                .filter_map(|s| s["title"].as_str().map(|t| format!("  • {}", t)))
                .collect())
            .unwrap_or_default();
        if !objective.is_empty() {
            return format!("목표: {}\n단계:\n{}", objective, steps.join("\n"));
        }
    }

    // JSON 없으면 raw 텍스트 첫 500자
    crate::utils::trunc(plan_text, 500).to_string()
}
