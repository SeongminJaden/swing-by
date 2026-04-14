//! Sprint Retrospective pipeline
//!
//! Official Scrum retrospective: Keep / Problem / Try (KPT) format
//!
//! Flow:
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
             ║  🔄 Sprint Retrospective — {}                     \n\
             ╚══════════════════════════════════════════════════╝\n\
             Team health: {}/10  |  Velocity trend: {}\n\n\
             ✅ What went well ({}):\n{}\n\n\
             ⚠️  Problems ({}):\n{}\n\n\
             🎯 Next sprint actions ({}):\n{}\n\n\
             🚀 Next sprint focus:\n  {}\n",
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
        "Project: {}\nSprint: {}\n\n{}",
        state_guard.project_name,
        actual_sprint_id,
        board_render_for_retro(&state_guard),
    );
    drop(state_guard);

    on_progress(&format!("\n🔄 ══ Sprint retrospective starting: {} ══", actual_sprint_id));

    // Roles participating in the retrospective
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

    // ── Collect single-turn feedback from each role ──────────────────────────────
    for role in &retro_roles {
        on_progress(&format!("  {} {} perspective gathering...", role.icon(), role.name()));

        let prompt = format!(
            "You are reviewing the just-completed sprint as the {} role.\n\n\
             Sprint info:\n{}\n\n\
             Answer concisely in KPT format (2-3 items each):\n\
             JSON output:\n\
             {{\n\
               \"went_well\": [\"what went well\"],\n\
               \"problems\": [\"problems\"],\n\
               \"suggestions\": [\"improvement suggestions\"]\n\
             }}",
            role.name(),
            crate::utils::trunc(&board_summary, 1500),
        );

        let system = format!("Model: {}\n\n{}", client.model(), role.system_prompt(""));
        let output = run_agent_simple(client, &system, &prompt, 3, &on_progress).await;

        let perspective = parse_perspective(&output, role.name());
        all_feedback.push_str(&format!(
            "\n=== {} feedback ===\nWent well: {}\nProblems: {}\nSuggestions: {}\n",
            role.name(),
            perspective.went_well.join(", "),
            perspective.problems.join(", "),
            perspective.suggestions.join(", "),
        ));
        perspectives.push(perspective);
    }

    // ── ScrumMaster — synthesis ──────────────────────────────────────────────────
    on_progress("🏃 ScrumMaster: Synthesizing retrospective and deriving action items...");

    let sm_prompt = format!(
        "You are the ScrumMaster synthesizing the team retrospective.\n\n\
         Sprint info:\n{}\n\n\
         Team feedback:\n{}\n\n\
         Synthesize and create an improvement plan for the next sprint.\n\
         JSON output:\n\
         {{\n\
           \"went_well\": [\"synthesized went well\"],\n\
           \"problems\": [\"synthesized problems\"],\n\
           \"action_items\": [\"SMART action items\"],\n\
           \"next_sprint_focus\": \"next sprint core objective\",\n\
           \"velocity_trend\": \"rising|stable|declining|unmeasurable\",\n\
           \"team_health_score\": 8\n\
         }}",
        crate::utils::trunc(&board_summary, 1000),
        crate::utils::trunc(&all_feedback, 2000),
    );

    let sm_system = format!("Model: {}\n\n{}", client.model(),
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
                .unwrap_or("Focus on quality improvement").to_string(),
            velocity_trend: v["velocity_trend"].as_str()
                .unwrap_or("unmeasurable").to_string(),
            team_health_score: v["team_health_score"].as_u64()
                .unwrap_or(7).min(10) as u8,
            perspectives,
        }
    } else {
        RetroResult {
            sprint_id: sprint_id.to_string(),
            went_well: vec!["Sprint completed".to_string()],
            problems: vec!["Retrospective parsing failed".to_string()],
            action_items: vec!["Manual retrospective required".to_string()],
            next_sprint_focus: "Improvement needed".to_string(),
            velocity_trend: "unmeasurable".to_string(),
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
        "Completed stories: {}/{}\nTotal bugs: {}\nRecent activity:\n{}",
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
            went_well: vec!["Fast deployment".to_string()],
            problems: vec!["Insufficient testing".to_string()],
            action_items: vec!["Achieve 80% test coverage".to_string()],
            next_sprint_focus: "Quality improvement".to_string(),
            velocity_trend: "rising".to_string(),
            team_health_score: 8,
            perspectives: vec![],
        };
        let rendered = result.render();
        assert!(rendered.contains("S5"));
        assert!(rendered.contains("Fast deployment"));
        assert!(rendered.contains("8/10"));
    }

    #[test]
    fn test_team_health_score_clamp() {
        // parse_retro_result should clamp score to max 10
        let json_text = r#"{"went_well":[],"problems":[],"action_items":[],
            "next_sprint_focus":"focus","velocity_trend":"rising","team_health_score":99}"#;
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
        let json = r#"{"went_well":["Deployment succeeded"],"problems":["Many bugs"],"suggestions":["Automated testing"]}"#;
        let perspective = parse_perspective(json, "QAEngineer");
        assert_eq!(perspective.went_well, vec!["Deployment succeeded"]);
        assert_eq!(perspective.problems, vec!["Many bugs"]);
    }
}
