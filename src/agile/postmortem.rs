//! Post-Mortem pipeline
//!
//! Post-incident: root cause analysis → fix → prevention
//!
//! 흐름:
//!   SRE          → Incident timeline + initial RCA
//!   Developer    → Bug fix implementation
//!   TechLead     → Fix code review + deploy approval
//!   SRE          → Final runbook update + prevention measures

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::agent::node::NodeHub;
use crate::agent::ollama::OllamaClient;
use crate::agile::runner::{run_agile_agent, run_agent_simple};
use crate::agile::story::{Priority, UserStory};
use crate::agile::team::AgileRole;

// ─── Post-mortem result ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub title: String,
    pub owner: String,
    pub due_date: String,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMortemResult {
    pub incident_id: String,
    pub severity: String,
    pub duration_minutes: u32,
    pub root_cause: String,
    pub timeline: String,
    pub impact: String,
    pub fix_summary: String,
    pub action_items: Vec<ActionItem>,
    pub runbook_updated: bool,
    pub lessons_learned: Vec<String>,
}

impl PostMortemResult {
    pub fn render(&self) -> String {
        let items: Vec<String> = self.action_items.iter()
            .map(|a| format!("  - [{}] {} (담당: {}, 기한: {})", a.priority, a.title, a.owner, a.due_date))
            .collect();

        format!(
            "\n╔══════════════════════════════════════════════════╗\n\
             ║  🚨 포스트모템 보고서 — {}                        \n\
             ╚══════════════════════════════════════════════════╝\n\
             심각도: {}  |  장애 시간: {}분\n\n\
             📋 근본 원인:\n{}\n\n\
             📊 영향 범위:\n{}\n\n\
             🔧 수정 내용:\n{}\n\n\
             ✅ 액션 아이템 ({}):\n{}\n\n\
             📚 교훈:\n{}\n",
            self.incident_id,
            self.severity,
            self.duration_minutes,
            self.root_cause,
            self.impact,
            self.fix_summary,
            self.action_items.len(),
            if items.is_empty() { "  없음".to_string() } else { items.join("\n") },
            self.lessons_learned.iter()
                .map(|l| format!("  - {}", l))
                .collect::<Vec<_>>().join("\n"),
        )
    }
}

// ─── Pipeline ─────────────────────────────────────────────────────────────

pub async fn run_postmortem(
    client: &OllamaClient,
    incident_description: &str,
    project_path: &str,
    on_progress: impl Fn(&str) + Clone,
) -> Result<PostMortemResult> {
    let hub = NodeHub::new();
    let incident_id = format!("INC-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() % 100000).unwrap_or(0));

    on_progress(&format!("\n🚨 ══ 포스트모템 시작: {} ══", incident_id));
    on_progress(&format!("장애 설명: {}", crate::utils::trunc(incident_description, 100)));

    // 포스트모템용 임시 스토리
    let mut story = UserStory::new(
        &incident_id,
        &format!("[장애] {}", crate::utils::trunc(incident_description, 60)),
        incident_description,
        Priority::Critical, 8,
    );
    story.add_acceptance_criteria("근본 원인 파악");
    story.add_acceptance_criteria("수정 코드 배포");
    story.add_acceptance_criteria("재발 방지책 문서화");

    // ── 1단계: SRE — 장애 분석 + 초기 RCA ──────────────────────────────────
    print_pm_divider("1/4 · SRE — 장애 분석 + RCA");
    on_progress("📡 SRE: 장애 타임라인 및 근본 원인 분석 중...");

    let sre_ctx = format!(
        "## 장애 정보\n설명: {}\n프로젝트 경로: {}\n\n\
         다음을 수행하세요:\n\
         1. 로그 및 코드를 분석하여 장애 원인 파악\n\
         2. 영향 범위 평가\n\
         3. 타임라인 재구성\n\
         4. 초기 RCA 작성",
        incident_description, project_path
    );
    let sre_output = run_agile_agent(client, AgileRole::SRE, &story, &sre_ctx, &hub, &on_progress).await;

    // ── 2단계: Developer — 수정 구현 ────────────────────────────────────────
    print_pm_divider("2/4 · Developer — 버그 수정");
    on_progress("💻 Developer: 근본 원인 수정 중...");

    let dev_ctx = format!(
        "## SRE 분석 결과\n{}\n\n\
         위 분석을 바탕으로:\n\
         1. 근본 원인을 수정하는 코드 작성\n\
         2. 회귀 테스트 추가\n\
         3. 수정 내용 설명",
        crate::utils::trunc(&sre_output, 2000)
    );
    story.implementation = Some(sre_output.clone());
    let dev_output = run_agile_agent(client, AgileRole::Developer, &story, &dev_ctx, &hub, &on_progress).await;

    // ── 3단계: TechLead — 수정 검토 + 배포 승인 ────────────────────────────
    print_pm_divider("3/4 · TechLead — 수정 검토 + 승인");
    on_progress("🎯 TechLead: 수정 코드 리뷰 및 배포 승인 중...");

    let tl_ctx = format!(
        "## 장애 수정 코드 리뷰\n\
         SRE 분석: {}\n\n\
         수정 코드: {}\n\n\
         리뷰 항목:\n\
         1. 수정이 근본 원인을 완전히 해결하는가?\n\
         2. 사이드 이펙트 위험\n\
         3. 즉시 배포 가능한가?",
        crate::utils::trunc(&sre_output, 1000),
        crate::utils::trunc(&dev_output, 1500),
    );
    story.implementation = Some(dev_output.clone());
    let tl_output = run_agile_agent(client, AgileRole::TechLead, &story, &tl_ctx, &hub, &on_progress).await;

    // ── 4단계: SRE — 최종 런북 업데이트 ────────────────────────────────────
    print_pm_divider("4/4 · SRE — 런북 + 재발 방지책");
    on_progress("📡 SRE: 최종 런북 업데이트 및 재발 방지책 작성 중...");

    let final_ctx = format!(
        "## 포스트모템 최종 단계\n\
         초기 분석: {}\n수정 완료: {}\nTechLead 승인: {}\n\n\
         다음을 완성하세요:\n\
         1. 최종 포스트모템 보고서\n\
         2. 런북 업데이트\n\
         3. 재발 방지 액션 아이템 (담당자 + 기한 포함)\n\
         4. 교훈 정리",
        crate::utils::trunc(&sre_output, 800),
        crate::utils::trunc(&dev_output, 600),
        crate::utils::trunc(&tl_output, 400),
    );
    let final_output = run_agile_agent(client, AgileRole::SRE, &story, &final_ctx, &hub, &on_progress).await;

    // ── 결과 Parsing ────────────────────────────────────────────────────────────
    let result = parse_postmortem_result(&final_output, &incident_id);
    on_progress(&result.render());

    Ok(result)
}

pub fn parse_postmortem_result(text: &str, incident_id: &str) -> PostMortemResult {
    if let Some(v) = extract_json(text) {
        let action_items: Vec<ActionItem> = v["action_items"].as_array()
            .map(|arr| arr.iter().map(|a| ActionItem {
                title: a["title"].as_str().unwrap_or("").to_string(),
                owner: a["owner"].as_str().unwrap_or("Team").to_string(),
                due_date: a["due_date"].as_str().unwrap_or("TBD").to_string(),
                priority: a["priority"].as_str().unwrap_or("Medium").to_string(),
            }).collect())
            .unwrap_or_default();

        let lessons: Vec<String> = v["lessons_learned"].as_array()
            .map(|arr| arr.iter().filter_map(|l| l.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["분석 결과를 검토하세요.".to_string()]);

        PostMortemResult {
            incident_id: incident_id.to_string(),
            severity: v["severity"].as_str().unwrap_or("High").to_string(),
            duration_minutes: v["duration_minutes"].as_u64().unwrap_or(0) as u32,
            root_cause: v["root_cause"].as_str().unwrap_or(text).to_string(),
            timeline: v["timeline"].as_str().unwrap_or("").to_string(),
            impact: v["impact"].as_str().unwrap_or("").to_string(),
            fix_summary: v["fix_summary"].as_str().unwrap_or("").to_string(),
            action_items,
            runbook_updated: v["runbook_updated"].as_bool().unwrap_or(false),
            lessons_learned: lessons,
        }
    } else {
        PostMortemResult {
            incident_id: incident_id.to_string(),
            severity: "Unknown".to_string(),
            duration_minutes: 0,
            root_cause: crate::utils::trunc(text, 300).to_string(),
            timeline: String::new(),
            impact: String::new(),
            fix_summary: String::new(),
            action_items: Vec::new(),
            runbook_updated: false,
            lessons_learned: vec!["분석 완료. 상세 내용 검토 필요.".to_string()],
        }
    }
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

fn print_pm_divider(title: &str) {
    let pad = 48usize.saturating_sub(title.len()) / 2;
    let sp = " ".repeat(pad);
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║{}{}{}║", sp, title, sp);
    println!("╚══════════════════════════════════════════════════╝");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result() -> PostMortemResult {
        PostMortemResult {
            incident_id: "INC-001".to_string(),
            severity: "High".to_string(),
            duration_minutes: 45,
            root_cause: "메모리 누수로 인한 OOM".to_string(),
            timeline: "14:00 이상 감지, 14:05 롤백".to_string(),
            impact: "API 응답 불가 5분".to_string(),
            fix_summary: "메모리 풀 설정 수정".to_string(),
            action_items: vec![ActionItem {
                title: "메모리 알람 추가".to_string(),
                owner: "SRE".to_string(),
                due_date: "2026-04-30".to_string(),
                priority: "High".to_string(),
            }],
            runbook_updated: true,
            lessons_learned: vec!["메모리 모니터링 중요".to_string()],
        }
    }

    #[test]
    fn test_render_contains_incident_id() {
        let result = make_result();
        let rendered = result.render();
        assert!(rendered.contains("INC-001"));
        assert!(rendered.contains("High"));
        assert!(rendered.contains("45"));
    }

    #[test]
    fn test_render_contains_action_items() {
        let result = make_result();
        let rendered = result.render();
        assert!(rendered.contains("메모리 알람 추가"));
        assert!(rendered.contains("SRE"));
    }

    #[test]
    fn test_parse_postmortem_valid_json() {
        let json = r#"{
            "severity": "Critical",
            "duration_minutes": 120,
            "root_cause": "DB 연결 풀 고갈",
            "timeline": "타임라인",
            "impact": "전체 서비스 다운",
            "fix_summary": "연결 풀 크기 증가",
            "action_items": [{"title":"알람","owner":"SRE","due_date":"2026-05-01","priority":"High"}],
            "runbook_updated": true,
            "lessons_learned": ["연결 풀 모니터링 필요"]
        }"#;
        let result = parse_postmortem_result(json, "INC-TEST");
        assert_eq!(result.severity, "Critical");
        assert_eq!(result.duration_minutes, 120);
        assert_eq!(result.action_items.len(), 1);
        assert!(result.runbook_updated);
    }

    #[test]
    fn test_parse_postmortem_invalid_json_fallback() {
        let result = parse_postmortem_result("파싱 불가 텍스트", "INC-FALLBACK");
        assert_eq!(result.incident_id, "INC-FALLBACK");
        assert_eq!(result.severity, "Unknown");
        assert!(!result.runbook_updated);
    }
}

// ── 단독 실행용 시스템 프롬프트 ─────────────────────────────────────────────
pub async fn run_sre_standalone(
    client: &OllamaClient,
    task: &str,
    on_progress: impl Fn(&str),
) -> String {
    let _hub = NodeHub::new();
    run_agent_simple(
        client,
        &format!("모델: {}\n\n{}\n\n{}",
            client.model(),
            crate::agent::tools::tool_descriptions(),
            AgileRole::SRE.system_prompt("")),
        task, 10, &on_progress,
    ).await
}
