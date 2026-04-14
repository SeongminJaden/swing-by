//! Post-Mortem pipeline
//!
//! Post-incident: root cause analysis → fix → prevention
//!
//! Flow:
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
            .map(|a| format!("  - [{}] {} (owner: {}, due: {})", a.priority, a.title, a.owner, a.due_date))
            .collect();

        format!(
            "\n╔══════════════════════════════════════════════════╗\n\
             ║  🚨 Post-Mortem Report — {}                       \n\
             ╚══════════════════════════════════════════════════╝\n\
             Severity: {}  |  Downtime: {} minutes\n\n\
             📋 Root Cause:\n{}\n\n\
             📊 Impact:\n{}\n\n\
             🔧 Fix Summary:\n{}\n\n\
             ✅ Action Items ({}):\n{}\n\n\
             📚 Lessons Learned:\n{}\n",
            self.incident_id,
            self.severity,
            self.duration_minutes,
            self.root_cause,
            self.impact,
            self.fix_summary,
            self.action_items.len(),
            if items.is_empty() { "  None".to_string() } else { items.join("\n") },
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

    on_progress(&format!("\n🚨 ══ Post-mortem starting: {} ══", incident_id));
    on_progress(&format!("Incident description: {}", crate::utils::trunc(incident_description, 100)));

    // Temporary story for post-mortem
    let mut story = UserStory::new(
        &incident_id,
        &format!("[Incident] {}", crate::utils::trunc(incident_description, 60)),
        incident_description,
        Priority::Critical, 8,
    );
    story.add_acceptance_criteria("Identify root cause");
    story.add_acceptance_criteria("Deploy fix code");
    story.add_acceptance_criteria("Document prevention measures");

    // ── Step 1: SRE — incident analysis + initial RCA ────────────────────────
    print_pm_divider("1/4 · SRE — Incident Analysis + RCA");
    on_progress("📡 SRE: Analyzing incident timeline and root cause...");

    let sre_ctx = format!(
        "## Incident Information\nDescription: {}\nProject path: {}\n\n\
         Perform the following:\n\
         1. Analyze logs and code to identify the cause\n\
         2. Assess impact scope\n\
         3. Reconstruct timeline\n\
         4. Write initial RCA",
        incident_description, project_path
    );
    let sre_output = run_agile_agent(client, AgileRole::SRE, &story, &sre_ctx, &hub, &on_progress).await;

    // ── Step 2: Developer — implement fix ───────────────────────────────────
    print_pm_divider("2/4 · Developer — Bug Fix");
    on_progress("💻 Developer: Fixing root cause...");

    let dev_ctx = format!(
        "## SRE Analysis Result\n{}\n\n\
         Based on the analysis above:\n\
         1. Write code that fixes the root cause\n\
         2. Add regression tests\n\
         3. Explain the fix",
        crate::utils::trunc(&sre_output, 2000)
    );
    story.implementation = Some(sre_output.clone());
    let dev_output = run_agile_agent(client, AgileRole::Developer, &story, &dev_ctx, &hub, &on_progress).await;

    // ── Step 3: TechLead — fix review + deploy approval ────────────────────
    print_pm_divider("3/4 · TechLead — Fix Review + Approval");
    on_progress("🎯 TechLead: Reviewing fix code and approving deployment...");

    let tl_ctx = format!(
        "## Incident Fix Code Review\n\
         SRE analysis: {}\n\n\
         Fix code: {}\n\n\
         Review checklist:\n\
         1. Does the fix fully resolve the root cause?\n\
         2. Side effect risks\n\
         3. Is immediate deployment safe?",
        crate::utils::trunc(&sre_output, 1000),
        crate::utils::trunc(&dev_output, 1500),
    );
    story.implementation = Some(dev_output.clone());
    let tl_output = run_agile_agent(client, AgileRole::TechLead, &story, &tl_ctx, &hub, &on_progress).await;

    // ── Step 4: SRE — final runbook update ──────────────────────────────────
    print_pm_divider("4/4 · SRE — Runbook + Prevention");
    on_progress("📡 SRE: Writing final runbook update and prevention measures...");

    let final_ctx = format!(
        "## Post-Mortem Final Stage\n\
         Initial analysis: {}\nFix complete: {}\nTechLead approval: {}\n\n\
         Complete the following:\n\
         1. Final post-mortem report\n\
         2. Runbook update\n\
         3. Prevention action items (with owner + due date)\n\
         4. Document lessons learned",
        crate::utils::trunc(&sre_output, 800),
        crate::utils::trunc(&dev_output, 600),
        crate::utils::trunc(&tl_output, 400),
    );
    let final_output = run_agile_agent(client, AgileRole::SRE, &story, &final_ctx, &hub, &on_progress).await;

    // ── Parse results ────────────────────────────────────────────────────────────
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
            .unwrap_or_else(|| vec!["Review the analysis results.".to_string()]);

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
            lessons_learned: vec!["Analysis complete. Detailed review required.".to_string()],
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
            root_cause: "OOM due to memory leak".to_string(),
            timeline: "Anomaly detected at 14:00, rolled back at 14:05".to_string(),
            impact: "API unavailable for 5 minutes".to_string(),
            fix_summary: "Fixed memory pool settings".to_string(),
            action_items: vec![ActionItem {
                title: "Add memory alert".to_string(),
                owner: "SRE".to_string(),
                due_date: "2026-04-30".to_string(),
                priority: "High".to_string(),
            }],
            runbook_updated: true,
            lessons_learned: vec!["Memory monitoring is important".to_string()],
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
        assert!(rendered.contains("Add memory alert"));
        assert!(rendered.contains("SRE"));
    }

    #[test]
    fn test_parse_postmortem_valid_json() {
        let json = r#"{
            "severity": "Critical",
            "duration_minutes": 120,
            "root_cause": "DB connection pool exhaustion",
            "timeline": "timeline",
            "impact": "Complete service outage",
            "fix_summary": "Increased connection pool size",
            "action_items": [{"title":"Add alert","owner":"SRE","due_date":"2026-05-01","priority":"High"}],
            "runbook_updated": true,
            "lessons_learned": ["Connection pool monitoring required"]
        }"#;
        let result = parse_postmortem_result(json, "INC-TEST");
        assert_eq!(result.severity, "Critical");
        assert_eq!(result.duration_minutes, 120);
        assert_eq!(result.action_items.len(), 1);
        assert!(result.runbook_updated);
    }

    #[test]
    fn test_parse_postmortem_invalid_json_fallback() {
        let result = parse_postmortem_result("unparseable text", "INC-FALLBACK");
        assert_eq!(result.incident_id, "INC-FALLBACK");
        assert_eq!(result.severity, "Unknown");
        assert!(!result.runbook_updated);
    }
}

// ── Standalone execution system prompt ──────────────────────────────────────
pub async fn run_sre_standalone(
    client: &OllamaClient,
    task: &str,
    on_progress: impl Fn(&str),
) -> String {
    let _hub = NodeHub::new();
    run_agent_simple(
        client,
        &format!("Model: {}\n\n{}\n\n{}",
            client.model(),
            crate::agent::tools::tool_descriptions(),
            AgileRole::SRE.system_prompt("")),
        task, 10, &on_progress,
    ).await
}
