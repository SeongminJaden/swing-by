//! 고도화된 ReAct 루프
#![allow(dead_code)]
//!
//! 표준 ReAct: Reason → Act → Observe
//! 고도화:     Reason → Plan → Act → Observe → Verify → Reflect → (반복)
//!
//! 추가 기능:
//! - 자동 검증: 각 툴 실행 후 결과가 예상과 일치하는지 확인
//! - 반성 루프: 같은 오류가 반복되면 접근법을 변경
//! - TDD 모드: 테스트 먼저 작성, 구현 후 검증
//! - 의존성 분석: 코드 변경 전 영향 범위 파악

use anyhow::Result;
use crate::agent::ollama::OllamaClient;
use crate::agent::tools::dispatch_tool;
use crate::models::{AgentResponse, Message, ToolCall};

// ─── ReAct 단계 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReActStep {
    pub thought: String,
    pub action: Option<String>,     // 툴 이름
    pub action_input: Vec<String>,  // 툴 인자
    pub observation: String,        // 툴 실행 결과
    pub verification: VerifyResult, // 검증 결과
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerifyResult {
    NotNeeded,
    Pass,
    Fail(String),  // 실패 이유
}

// ─── ReAct 설정 ──────────────────────────────────────────────────────────────

pub struct ReActConfig {
    pub max_turns: usize,
    pub max_retries_per_error: usize,  // 같은 에러 최대 재시도
    pub verify_enabled: bool,          // 결과 자동 검증
    pub tdd_mode: bool,                // TDD 모드
    pub reflection_enabled: bool,      // 실패 반성 루프
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

// ─── ReAct 실행 결과 ─────────────────────────────────────────────────────────

pub struct ReActResult {
    pub final_answer: String,
    pub steps: Vec<ReActStep>,
    pub retries: usize,
    pub success: bool,
}

// ─── 고도화 ReAct 루프 ───────────────────────────────────────────────────────

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

    // TDD 모드: 테스트 먼저
    if config.tdd_mode {
        inject_tdd_instruction(history);
    }

    for turn in 0..config.max_turns {
        // 반성 내용이 있으면 히스토리에 주입
        if !reflection_msgs.is_empty() {
            let reflection = format!(
                "⚠️ 이전 시도 분석:\n{}\n\n다른 접근법을 시도하세요.",
                reflection_msgs.last().unwrap()
            );
            history.push(Message::tool(reflection));
            reflection_msgs.clear();
        }

        // AI 응답 생성
        let ai_text = match client.chat_stream(history.clone(), |_| {}).await {
            Ok(t) => t,
            Err(e) => {
                return ReActResult {
                    final_answer: format!("AI 오류: {}", e),
                    steps,
                    retries: total_retries,
                    success: false,
                };
            }
        };

        match crate::agent::chat::parse_response_pub(&ai_text) {
            AgentResponse::Exit | AgentResponse::Text(_) => {
                // 최종 답변
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
                // 다중 툴 처리
                history.push(Message::assistant(&ai_text));
                let mut results = Vec::new();
                for raw in &tc.args {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
                    let name = val["name"].as_str().unwrap_or("").to_string();
                    let args: Vec<String> = val["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    let result = dispatch_tool(&ToolCall { name: name.clone(), args: args.clone() }).await;
                    results.push(format!("툴 '{}' 결과:\n{}", name, result.output));

                    let step = ReActStep {
                        thought: format!("다중 툴: {}", name),
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
                // 에러 빈도 체크
                let err_key = tc.name.clone();
                let result = dispatch_tool(&tc).await;

                // 검증
                let verification = if config.verify_enabled {
                    verify_result(&tc.name, &tc.args, &result.output, result.success)
                } else {
                    VerifyResult::NotNeeded
                };

                let step = ReActStep {
                    thought: format!("턴 {}: {}", turn + 1, ai_text.lines().next().unwrap_or("")),
                    action: Some(tc.name.clone()),
                    action_input: tc.args.clone(),
                    observation: result.output.clone(),
                    verification: verification.clone(),
                };
                on_step(&step);
                steps.push(step.clone());

                history.push(Message::assistant(&ai_text));

                // 실패 처리
                if !result.success {
                    let count = error_counts.entry(err_key).or_insert(0);
                    *count += 1;
                    total_retries += 1;

                    if *count >= config.max_retries_per_error && config.reflection_enabled {
                        // 반성 메시지 생성
                        let reflection = generate_reflection(&steps);
                        reflection_msgs.push(reflection.clone());
                        history.push(Message::tool(format!(
                            "툴 '{}' 결과:\n{}\n\n[{}번째 실패 — 접근법 변경 권장]",
                            tc.name, result.output, count
                        )));
                        *count = 0; // 카운터 리셋
                    } else {
                        history.push(Message::tool(format!(
                            "툴 '{}' 결과 (실패):\n{}", tc.name, result.output
                        )));
                    }
                } else if matches!(verification, VerifyResult::Fail(_)) {
                    // 검증 실패: 경고와 함께 계속
                    if let VerifyResult::Fail(ref reason) = verification {
                        history.push(Message::tool(format!(
                            "툴 '{}' 결과:\n{}\n\n⚠️ 검증 경고: {}",
                            tc.name, result.output, reason
                        )));
                    }
                } else {
                    history.push(Message::tool(format!(
                        "툴 '{}' 결과:\n{}", tc.name, result.output
                    )));
                }
            }
        }
    }

    ReActResult {
        final_answer: "최대 턴 수 초과".to_string(),
        steps,
        retries: total_retries,
        success: false,
    }
}

// ─── 검증 로직 ───────────────────────────────────────────────────────────────

/// 툴 실행 결과를 의미론적으로 검증
fn verify_result(
    tool_name: &str,
    args: &[String],
    output: &str,
    success: bool,
) -> VerifyResult {
    if !success {
        return VerifyResult::Fail(format!("툴 실패: {}", crate::utils::trunc(output, 100)));
    }

    match tool_name {
        // 파일 쓰기 → 파일이 존재하는지 확인
        "write_file" => {
            if let Some(path) = args.first() {
                if !std::path::Path::new(path).exists() {
                    return VerifyResult::Fail(format!("파일이 생성되지 않음: {}", path));
                }
            }
            VerifyResult::Pass
        }

        // 빌드 → stderr에 error 없는지 확인
        "run_shell" => {
            let lower = output.to_lowercase();
            if lower.contains("error[") || lower.contains("error:") {
                // warning은 허용, error만 체크
                let has_real_error = output.lines()
                    .any(|l| l.trim_start().starts_with("error") && !l.contains("warning"));
                if has_real_error {
                    return VerifyResult::Fail("빌드/실행 에러 감지".to_string());
                }
            }
            VerifyResult::Pass
        }

        // 테스트 → test result 확인
        "run_tests" => {
            if output.contains("FAILED") || output.contains("failures:") {
                let failed: Vec<&str> = output.lines()
                    .filter(|l| l.contains("FAILED") || l.starts_with("test ") && l.ends_with("FAILED"))
                    .collect();
                return VerifyResult::Fail(format!(
                    "테스트 실패: {}",
                    crate::utils::trunc(&failed.join(", "), 200)
                ));
            }
            VerifyResult::Pass
        }

        // git commit → 커밋 해시 확인
        "git_commit" | "git_commit_all" => {
            if !output.contains('[') {
                return VerifyResult::Fail("커밋이 생성되지 않은 것 같음".to_string());
            }
            VerifyResult::Pass
        }

        _ => VerifyResult::NotNeeded,
    }
}

// ─── 반성 생성 ───────────────────────────────────────────────────────────────

/// 실패한 단계들을 분석하여 반성 메시지 생성
fn generate_reflection(steps: &[ReActStep]) -> String {
    let failures: Vec<&ReActStep> = steps.iter()
        .filter(|s| matches!(s.verification, VerifyResult::Fail(_)))
        .collect();

    if failures.is_empty() {
        return "반복 실패 감지 — 다른 접근법 시도".to_string();
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
        "다음 방법들이 실패했습니다:\n{}\n\n완전히 다른 접근법을 사용하세요.",
        failure_summary.join("\n")
    )
}

// ─── TDD 모드 ────────────────────────────────────────────────────────────────

fn inject_tdd_instruction(history: &mut Vec<Message>) {
    let tdd_instruction = "\n\n=== TDD 모드 ===\n\
        구현 순서를 반드시 지키세요:\n\
        1. 실패하는 테스트 먼저 작성\n\
        2. 테스트가 실패하는지 확인 (Red)\n\
        3. 테스트를 통과하는 최소한의 코드 구현 (Green)\n\
        4. 코드 개선 (Refactor)\n\
        5. 모든 테스트 통과 확인";

    if let Some(first) = history.first_mut() {
        if matches!(first.role, crate::models::Role::System) {
            first.content.push_str(tdd_instruction);
        }
    }
}

// ─── 의존성 분석 ─────────────────────────────────────────────────────────────

/// 파일 변경 전 영향 범위 분석
pub async fn analyze_impact(
    client: &OllamaClient,
    file_path: &str,
    change_description: &str,
) -> Result<String> {
    // 파일을 import/use하는 곳 찾기
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
        return Ok(format!("'{}' 에 대한 참조가 없습니다.", filename));
    }

    // AI에게 영향 분석 요청
    let prompt = format!(
        "다음 파일을 변경합니다: `{}`\n\
         변경 내용: {}\n\n\
         이 파일을 참조하는 곳들:\n{}\n\n\
         이 변경이 미칠 영향을 간단히 분석하세요 (200자 이내).",
        file_path,
        change_description,
        affected_files.join("\n")
    );

    let msgs = vec![
        Message::system("당신은 코드 영향도 분석 전문가입니다."),
        Message::user(&prompt),
    ];

    let result = client.chat(msgs).await
        .map(|r| r.message.content)
        .unwrap_or_else(|_| format!("{} 곳에서 참조됨", affected_files.len()));

    Ok(format!(
        "영향 범위 ({} 곳):\n{}\n\n분석:\n{}",
        affected_files.len(),
        affected_files.join("\n"),
        result
    ))
}
