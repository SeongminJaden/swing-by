//! Technical Debt analysis pipeline
//!
//! Systematically evaluates and prioritizes technical debt in the codebase.
//!
//! Flow:
//!   Architect    → Architecture/design level debt analysis
//!   Developer    → Code level debt analysis (complexity, duplication, missing tests)
//!   Reviewer     → Quality/coverage debt
//!   TechLead     → Synthesis + prioritization + repayment plan

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::agent::node::NodeHub;
use crate::agent::ollama::OllamaClient;
use crate::agile::runner::run_agile_agent;
use crate::agile::story::{Priority, UserStory};
use crate::agile::team::AgileRole;

// ─── Data structures ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtItem {
    pub id: String,
    pub category: DebtCategory,
    pub title: String,
    pub description: String,
    pub file: Option<String>,
    pub estimated_days: f32,
    pub priority: String,        // High/Medium/Low
    pub impact: String,          // impact if ignored
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebtCategory {
    Architecture,    // bad design, excessive coupling
    Code,            // complexity, duplication, long functions
    Testing,         // missing tests, low coverage
    Documentation,   // no documentation, stale comments
    Dependencies,    // outdated dependencies, vulnerable packages
    Security,        // security configuration debt
    Performance,     // known performance issues
    Infrastructure,  // CI/CD, deployment process debt
}

impl std::fmt::Display for DebtCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DebtCategory::Architecture   => "🏛️  Architecture",
            DebtCategory::Code           => "💻 Code Quality",
            DebtCategory::Testing        => "🔬 Testing",
            DebtCategory::Documentation  => "📝 Documentation",
            DebtCategory::Dependencies   => "📦 Dependencies",
            DebtCategory::Security       => "🔒 Security",
            DebtCategory::Performance    => "⚡ Performance",
            DebtCategory::Infrastructure => "🚀 Infrastructure",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechDebtReport {
    pub project_path: String,
    pub analyzed_at: u64,
    pub debt_items: Vec<DebtItem>,
    pub total_estimated_days: f32,
    pub repayment_plan: Vec<String>,
    pub recommended_priority: Vec<String>,
    pub debt_ratio: String,           // estimated debt ratio (tech debt / total dev cost)
}

impl TechDebtReport {
    pub fn render(&self) -> String {
        let by_category = self.grouped_by_category();
        let mut out = vec![
            format!("\n╔══════════════════════════════════════════════════╗"),
            format!("║  📊 Technical Debt Analysis Report                ║"),
            format!("╚══════════════════════════════════════════════════╝"),
            format!("Path: {}  |  Total estimate: {} day(s)  |  Debt ratio: {}",
                self.project_path, self.total_estimated_days, self.debt_ratio),
        ];

        for (cat, items) in &by_category {
            let days: f32 = items.iter().map(|i| i.estimated_days).sum();
            out.push(format!("\n── {} ({} item(s), {} day(s)) ──", cat, items.len(), days));
            for item in items {
                out.push(format!("  [{:>4}] [{}] {}{}",
                    item.priority,
                    format!("{:.1}d", item.estimated_days),
                    item.title,
                    item.file.as_deref().map(|f| format!(" ({})", f)).unwrap_or_default(),
                ));
            }
        }

        if !self.recommended_priority.is_empty() {
            out.push("\n── Priority resolution order ──".to_string());
            for (i, p) in self.recommended_priority.iter().enumerate() {
                out.push(format!("  {}. {}", i+1, p));
            }
        }

        if !self.repayment_plan.is_empty() {
            out.push("\n── Repayment plan ──".to_string());
            for step in &self.repayment_plan {
                out.push(format!("  • {}", step));
            }
        }

        out.push(String::new());
        out.join("\n")
    }

    fn grouped_by_category(&self) -> Vec<(String, Vec<&DebtItem>)> {
        let cats = [
            DebtCategory::Architecture,
            DebtCategory::Code,
            DebtCategory::Testing,
            DebtCategory::Security,
            DebtCategory::Performance,
            DebtCategory::Dependencies,
            DebtCategory::Documentation,
            DebtCategory::Infrastructure,
        ];
        cats.iter().filter_map(|cat| {
            let items: Vec<&DebtItem> = self.debt_items.iter()
                .filter(|i| &i.category == cat).collect();
            if items.is_empty() { None }
            else { Some((cat.to_string(), items)) }
        }).collect()
    }
}

// ─── Pipeline ─────────────────────────────────────────────────────────────

pub async fn run_techdebt_analysis(
    client: &OllamaClient,
    project_path: &str,
    on_progress: impl Fn(&str) + Clone,
) -> Result<TechDebtReport> {
    let hub = NodeHub::new();
    on_progress(&format!("\n📊 ══ Tech debt analysis starting: {} ══", project_path));

    // Temporary story for analysis
    let mut story = UserStory::new(
        "TD-ANALYSIS",
        &format!("[Tech Debt Analysis] {}", project_path),
        &format!("Analyzing technical debt of project {}.", project_path),
        Priority::High, 5,
    );
    story.add_acceptance_criteria("Identify debt across all categories");
    story.add_acceptance_criteria("Establish priority and repayment plan");

    let base_ctx = format!(
        "## Analysis Target\nPath: {}\n\n\
         Read project files directly (read_file, glob_files, grep_files) and analyze the actual code.\n\
         Use web_search to compare against current best practices for the technology.",
        project_path
    );

    // ── Step 1: Architect — architecture/design debt ───────────────────────────
    print_td_divider("1/4 · Architect — Architecture/Design Debt");
    on_progress("🏛️  Architect: Analyzing architecture debt...");

    let arch_ctx = format!("{}\n\nAnalyze the following:\n\
        - Module coupling (circular dependencies, excessive coupling)\n\
        - SOLID principle violations\n\
        - Domain logic leakage\n\
        - Incorrect abstraction levels\n\
        JSON: {{\"debt_items\": [{{\"category\": \"Architecture\", \"title\": \"...\", \
        \"description\": \"...\", \"file\": \"...\", \"estimated_days\": 2.0, \"priority\": \"High\", \"impact\": \"...\"}}]}}",
        base_ctx);
    let arch_output = run_agile_agent(client, AgileRole::Architect, &story, &arch_ctx, &hub, &on_progress).await;

    // ── Step 2: Developer — code quality debt ───────────────────────────────────
    print_td_divider("2/4 · Developer — Code Quality Debt");
    on_progress("💻 Developer: Analyzing code quality debt...");

    let dev_ctx = format!("{}\n\nAnalyze the following:\n\
        - High-complexity functions (McCabe complexity > 10)\n\
        - Code duplication (DRY violations)\n\
        - Magic numbers/strings\n\
        - Missing error handling\n\
        - Core logic without tests\n\
        - Outdated dependencies (cargo audit, etc.)\n\
        JSON: {{\"debt_items\": [...]}}",
        base_ctx);
    let dev_output = run_agile_agent(client, AgileRole::Developer, &story, &dev_ctx, &hub, &on_progress).await;

    // ── Step 3: Reviewer — quality/test debt ────────────────────────────────────
    print_td_divider("3/4 · Reviewer — Quality/Test Debt");
    on_progress("👁️  Reviewer: Analyzing test/documentation debt...");

    let rev_ctx = format!("{}\n\nAnalyze the following:\n\
        - Test coverage (areas below 80%)\n\
        - Missing integration tests\n\
        - Missing documentation (README, API docs)\n\
        - Missing performance regression tests\n\
        - CI/CD pipeline gaps\n\
        JSON: {{\"debt_items\": [...]}}",
        base_ctx);
    let rev_output = run_agile_agent(client, AgileRole::Reviewer, &story, &rev_ctx, &hub, &on_progress).await;

    // ── Step 4: TechLead — synthesis + prioritization ────────────────────────────
    print_td_divider("4/4 · TechLead — Synthesis + Repayment Plan");
    on_progress("🎯 TechLead: Synthesizing tech debt and creating repayment plan...");

    let tl_ctx = format!(
        "Synthesize the following three analysis results into a prioritized repayment plan.\n\n\
         Architecture analysis:\n{}\n\n\
         Code analysis:\n{}\n\n\
         Quality analysis:\n{}\n\n\
         JSON output:\n\
         {{\n\
           \"debt_items\": [...all items merged...],\n\
           \"total_estimated_days\": 15.5,\n\
           \"debt_ratio\": \"tech debt ratio description\",\n\
           \"recommended_priority\": [\"top priority\", \"second priority\"],\n\
           \"repayment_plan\": [\"Sprint 1: ...\", \"Sprint 2: ...\"]\n\
         }}",
        crate::utils::trunc(&arch_output, 1000),
        crate::utils::trunc(&dev_output, 1000),
        crate::utils::trunc(&rev_output, 1000),
    );
    let tl_output = run_agile_agent(client, AgileRole::TechLead, &story, &tl_ctx, &hub, &on_progress).await;

    let report = parse_techdebt_report(&tl_output, project_path);
    on_progress(&report.render());

    Ok(report)
}

// ─── Parsing ────────────────────────────────────────────────────────────────────

fn parse_techdebt_report(text: &str, project_path: &str) -> TechDebtReport {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);

    if let Some(v) = extract_json(text) {
        let debt_items: Vec<DebtItem> = v["debt_items"].as_array()
            .map(|arr| arr.iter().enumerate().map(|(i, d)| DebtItem {
                id: format!("TD-{}", i + 1),
                category: parse_category(d["category"].as_str().unwrap_or("Code")),
                title: d["title"].as_str().unwrap_or("").to_string(),
                description: d["description"].as_str().unwrap_or("").to_string(),
                file: d["file"].as_str().map(|s| s.to_string()),
                estimated_days: d["estimated_days"].as_f64().unwrap_or(1.0) as f32,
                priority: d["priority"].as_str().unwrap_or("Medium").to_string(),
                impact: d["impact"].as_str().unwrap_or("").to_string(),
            }).collect())
            .unwrap_or_default();

        let total_days: f32 = debt_items.iter().map(|d| d.estimated_days).sum();

        TechDebtReport {
            project_path: project_path.to_string(),
            analyzed_at: now,
            total_estimated_days: v["total_estimated_days"].as_f64()
                .unwrap_or(total_days as f64) as f32,
            repayment_plan: str_array(&v["repayment_plan"]),
            recommended_priority: str_array(&v["recommended_priority"]),
            debt_ratio: v["debt_ratio"].as_str().unwrap_or("unmeasurable").to_string(),
            debt_items,
        }
    } else {
        TechDebtReport {
            project_path: project_path.to_string(),
            analyzed_at: now,
            debt_items: Vec::new(),
            total_estimated_days: 0.0,
            repayment_plan: vec!["Manual analysis required".to_string()],
            recommended_priority: Vec::new(),
            debt_ratio: "unmeasurable".to_string(),
        }
    }
}

fn parse_category(s: &str) -> DebtCategory {
    match s.to_lowercase().as_str() {
        "architecture" => DebtCategory::Architecture,
        "code" | "code quality" => DebtCategory::Code,
        "testing" => DebtCategory::Testing,
        "documentation" => DebtCategory::Documentation,
        "dependencies" => DebtCategory::Dependencies,
        "security" => DebtCategory::Security,
        "performance" => DebtCategory::Performance,
        "infrastructure" => DebtCategory::Infrastructure,
        _ => DebtCategory::Code,
    }
}

fn str_array(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
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

fn print_td_divider(title: &str) {
    let pad = 48usize.saturating_sub(title.len()) / 2;
    let sp = " ".repeat(pad);
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║{}{}{}║", sp, title, sp);
    println!("╚══════════════════════════════════════════════════╝");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_category_english() {
        assert_eq!(parse_category("Architecture"), DebtCategory::Architecture);
        assert_eq!(parse_category("code"), DebtCategory::Code);
        assert_eq!(parse_category("testing"), DebtCategory::Testing);
        assert_eq!(parse_category("security"), DebtCategory::Security);
        assert_eq!(parse_category("performance"), DebtCategory::Performance);
        assert_eq!(parse_category("infrastructure"), DebtCategory::Infrastructure);
    }

    #[test]
    fn test_parse_category_unknown_defaults_to_code() {
        assert_eq!(parse_category("unknown_xyz"), DebtCategory::Code);
    }

    #[test]
    fn test_debt_category_display() {
        assert!(DebtCategory::Architecture.to_string().contains("Architecture"));
        assert!(DebtCategory::Security.to_string().contains("Security"));
        assert!(DebtCategory::Testing.to_string().contains("Testing"));
    }

    #[test]
    fn test_extract_json_from_code_fence() {
        let text = "description\n```json\n{\"key\": \"value\"}\n```\nend";
        let val = extract_json(text);
        assert!(val.is_some());
        assert_eq!(val.unwrap()["key"], "value");
    }

    #[test]
    fn test_extract_json_raw() {
        let text = "prefix {\"total\": 42} suffix";
        let val = extract_json(text);
        assert!(val.is_some());
        assert_eq!(val.unwrap()["total"], 42);
    }

    #[test]
    fn test_extract_json_none_when_missing() {
        assert!(extract_json("no json here").is_none());
    }

    #[test]
    fn test_techdebt_report_render_empty() {
        let report = TechDebtReport {
            project_path: "/tmp".to_string(),
            analyzed_at: 0,
            debt_items: vec![],
            total_estimated_days: 0.0,
            repayment_plan: vec![],
            recommended_priority: vec![],
            debt_ratio: "0%".to_string(),
        };
        let rendered = report.render();
        assert!(rendered.contains("Technical Debt Analysis Report"));
    }

    #[test]
    fn test_str_array_parses_correctly() {
        let val = serde_json::json!(["a", "b", "c"]);
        let arr = str_array(&val);
        assert_eq!(arr, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_str_array_empty_on_non_array() {
        let val = serde_json::json!({"key": "not array"});
        let arr = str_array(&val);
        assert!(arr.is_empty());
    }
}
