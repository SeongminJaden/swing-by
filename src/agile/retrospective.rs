//! Sprint Retrospective pipeline
//!
//! Official Scrum retrospective: Keep / Problem / Try (KPT) format
//!
//! 흐름:
//!   ScrumMaster  → Facilitate retrospective + collect data
//!   Each role → submit feedback from own perspective (single turn)
//!   ScrumMaster  → Synthesis + next sprint action items

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::agent::ollama::OllamaClient;
use crate::agile::board::AgileBoard;
use crate::agile::runner::run_agent_simple;
use crate::agile::team::AgileRole;

// ─── Retrospective result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePerspective {
    pub role: String,
    pub went_well: Vec<String>,
    pub problems: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetroResult {
    pub sprint_id: String,
    pub went_well: Vec<String>,
    pub problems: Vec<String>,
    pub action_items: Vec<String>,
    pub next_sprint_focus: String,
    pub velocity_trend: String,
    pub team_health_score: u8,         // 1-10
    pub perspectives: Vec<RolePerspective>,
}

impl RetroResult {
    pub fn render(&self) -> String {
        let well: Vec<String> = self.went_well.iter().map(|s| format!("  ✅ {}", s)).collect();
        let probs: Vec<String> = self.problems.iter().map(|s| format!("  ⚠️  {}", s)).collect();
        let acts: Vec<String> = self.action_items.iter().map(|s| format!("  🎯 {}", s)).collect();

        format!(
            "\n╔══════════════════════════════════════════════════╗\n\
             ║  🔄 스프린트 회고 — {}                            \n\
             ╚══════════════════════════════════════════════════╝\n\
             팀 건강도: {}/10  |  속도 트렌드: {}\n\n\
             ✅ 잘된 것 ({}):\n{}\n\n\
             ⚠️  문제점 ({}):\n{}\n\n\
             🎯 다음 스프린트 액션 ({}):\n{}\n\n\
             🚀 다음 스프린트 포커스:\n  {}\n",
            self.sprint_id,
            self.team_health_score, self.velocity_trend,
            self.went_well.len(), well.join("\n"),
            self.problems.len(), probs.join("\n"),
            self.action_items.len(), acts.join("\n"),
            self.next_sprint_focus,
        )
    }
}

// ─── Pipeline ─────────────────────────────────────────────────────────────

pub async fn run_retrospective(
    client: &OllamaClient,
    board: &AgileBoard,
    sprint_id: Option<&str>,
    on_progress: impl Fn(&str) + Clone,
) -> Result<RetroResult> {
    let state = board.shared_state();
    let state_guard = state.lock().unwrap();

    let actual_sprint_id = sprint_id
        .map(|s| s.to_string())
        .or_else(|| state_guard.current_sprint_id.clone())
        .unwrap_or_else(|| "LATEST".to_string());

    let board_summary = format!(
        "프로젝트: {}\n스프린트: {}\n\n{}",
        state_guard.project_name,
        actual_sprint_id,
        board_render_for_retro(&state_guard),
    );
    drop(state_guard);

    on_progress(&format!("\n🔄 ══ 스프린트 회고 시작: {} ══", actual_sprint_id));

    // 회고에 참여하는 역할 목록
    let retro_roles = [
        AgileRole::Developer,
        AgileRole::QAEngineer,
        AgileRole::Reviewer,
        AgileRole::Architect,
        AgileRole::DevOpsEngineer,
        AgileRole::TechLead,
    ];

    let mut perspectives: Vec<RolePerspective> = Vec::new();
    let mut all_feedback = String::new();

    // ── 각 역할에서 단일 턴 피드백 수집 ─────────────────────────────────────
    for role in &retro_roles {
        on_progress(&format!("  {} {} 관점 수집 중...", role.icon(), role.name()));

        let prompt = format!(
            "당신은 {} 역할로 방금 끝난 스프린트를 회고합니다.\n\n\
             스프린트 정보:\n{}\n\n\
             KPT 형식으로 간결하게 답하세요 (각 항목 2-3개):\n\
             JSON 출력:\n\
             {{\n\
               \"went_well\": [\"잘 된 것\"],\n\
               \"problems\": [\"문제점\"],\n\
               \"suggestions\": [\"개선 제안\"]\n\
             }}",
            role.name(),
            crate::utils::trunc(&board_summary, 1500),
        );

        let system = format!("모델: {}\n\n{}", client.model(), role.system_prompt(""));
        let output = run_agent_simple(client, &system, &prompt, 3, &on_progress).await;

        let perspective = parse_perspective(&output, role.name());
        all_feedback.push_str(&format!(
            "\n=== {} 피드백 ===\n잘된것: {}\n문제: {}\n제안: {}\n",
            role.name(),
            perspective.went_well.join(", "),
            perspective.problems.join(", "),
            perspective.suggestions.join(", "),
        ));
        perspectives.push(perspective);
    }

    // ── ScrumMaster — 종합 ───────────────────────────────────────────────────
    on_progress("🏃 ScrumMaster: 회고 종합 및 액션 아이템 도출 중...");

    let sm_prompt = format!(
        "당신은 ScrumMaster로 팀 회고를 종합합니다.\n\n\
         스프린트 정보:\n{}\n\n\
         팀 피드백:\n{}\n\n\
         종합하여 다음 스프린트 개선 계획을 수립하세요.\n\
         JSON 출력:\n\
         {{\n\
           \"went_well\": [\"종합된 잘된 것\"],\n\
           \"problems\": [\"종합된 문제점\"],\n\
           \"action_items\": [\"SMART 액션 아이템\"],\n\
           \"next_sprint_focus\": \"다음 스프린트 핵심 목표\",\n\
           \"velocity_trend\": \"상승|유지|하락|측정불가\",\n\
           \"team_health_score\": 8\n\
         }}",
        crate::utils::trunc(&board_summary, 1000),
        crate::utils::trunc(&all_feedback, 2000),
    );

    let sm_system = format!("모델: {}\n\n{}", client.model(),
        AgileRole::ScrumMaster.system_prompt(""));
    let sm_output = run_agent_simple(client, &sm_system, &sm_prompt, 4, &on_progress).await;

    let result = parse_retro_result(&sm_output, &actual_sprint_id, perspectives);
    on_progress(&result.render());

    Ok(result)
}

// ─── Parsing Helpers ──────────────────────────────────────────────────────────────

fn parse_perspective(text: &str, role_name: &str) -> RolePerspective {
    if let Some(v) = extract_json(text) {
        RolePerspective {
            role: role_name.to_string(),
            went_well: str_array(&v["went_well"]),
            problems: str_array(&v["problems"]),
            suggestions: str_array(&v["suggestions"]),
        }
    } else {
        RolePerspective {
            role: role_name.to_string(),
            went_well: Vec::new(),
            problems: vec![crate::utils::trunc(text, 100).to_string()],
            suggestions: Vec::new(),
        }
    }
}

fn parse_retro_result(
    text: &str,
    sprint_id: &str,
    perspectives: Vec<RolePerspective>,
) -> RetroResult {
    parse_retro_result_pub(text, sprint_id, perspectives)
}

pub fn parse_retro_result_pub(
    text: &str,
    sprint_id: &str,
    perspectives: Vec<RolePerspective>,
) -> RetroResult {
    if let Some(v) = extract_json(text) {
        RetroResult {
            sprint_id: sprint_id.to_string(),
            went_well: str_array(&v["went_well"]),
            problems: str_array(&v["problems"]),
            action_items: str_array(&v["action_items"]),
            next_sprint_focus: v["next_sprint_focus"].as_str()
                .unwrap_or("품질 개선 집중").to_string(),
            velocity_trend: v["velocity_trend"].as_str()
                .unwrap_or("측정불가").to_string(),
            team_health_score: v["team_health_score"].as_u64()
                .unwrap_or(7).min(10) as u8,
            perspectives,
        }
    } else {
        RetroResult {
            sprint_id: sprint_id.to_string(),
            went_well: vec!["스프린트 완료".to_string()],
            problems: vec!["회고 파싱 실패".to_string()],
            action_items: vec!["수동 회고 필요".to_string()],
            next_sprint_focus: "개선 필요".to_string(),
            velocity_trend: "측정불가".to_string(),
            team_health_score: 5,
            perspectives,
        }
    }
}

fn str_array(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
}

fn board_render_for_retro(state: &crate::agile::board::BoardState) -> String {
    let done = state.stories.values()
        .filter(|s| matches!(s.status,
            crate::agile::story::StoryStatus::Done | crate::agile::story::StoryStatus::Released))
        .count();
    let total = state.stories.len();
    let total_bugs: usize = state.stories.values()
        .map(|s| s.bug_reports.len()).sum();

    format!(
        "완료 스토리: {}/{}\n총 버그: {}\n최근 활동:\n{}",
        done, total, total_bugs,
        state.activity_log.iter().rev().take(10)
            .map(|a| format!("  {}", a.format()))
            .collect::<Vec<_>>().join("\n"),
    )
}

fn extract_json(text: &str) -> Option<serde_json::Value> {
    let candidate = if let Some(s) = text.find("```json") {
        let after = &text[s + 7..];
        if let Some(e) = after.find("```") { &after[..e] } else { after }
    } else if let Some(s) = text.find('{') {
        if let Some(e) = text.rfind('}') { &text[s..=e] } else { return None }
    } else { return None };
    serde_json::from_str(candidate.trim()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retro_result_render_contains_sprint_id() {
        let result = RetroResult {
            sprint_id: "S5".to_string(),
            went_well: vec!["빠른 배포".to_string()],
            problems: vec!["테스트 부족".to_string()],
            action_items: vec!["테스트 커버리지 80% 달성".to_string()],
            next_sprint_focus: "품질 개선".to_string(),
            velocity_trend: "상승".to_string(),
            team_health_score: 8,
            perspectives: vec![],
        };
        let rendered = result.render();
        assert!(rendered.contains("S5"));
        assert!(rendered.contains("빠른 배포"));
        assert!(rendered.contains("8/10"));
    }

    #[test]
    fn test_team_health_score_clamp() {
        // parse_retro_result should clamp score to max 10
        let json_text = r#"{"went_well":[],"problems":[],"action_items":[],
            "next_sprint_focus":"focus","velocity_trend":"상승","team_health_score":99}"#;
        let result = super::parse_retro_result_pub(json_text, "S1", vec![]);
        assert!(result.team_health_score <= 10);
    }

    #[test]
    fn test_parse_perspective_invalid_returns_fallback() {
        let perspective = parse_perspective("invalid text with no JSON", "Developer");
        assert_eq!(perspective.role, "Developer");
        assert!(!perspective.problems.is_empty()); // fallback puts text in problems
    }

    #[test]
    fn test_parse_perspective_valid_json() {
        let json = r#"{"went_well":["배포 성공"],"problems":["버그 다수"],"suggestions":["자동화 테스트"]}"#;
        let perspective = parse_perspective(json, "QAEngineer");
        assert_eq!(perspective.went_well, vec!["배포 성공"]);
        assert_eq!(perspective.problems, vec!["버그 다수"]);
    }
}
