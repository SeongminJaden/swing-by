//! Technical Debt analysis pipeline
//!
//! Systematically evaluates and prioritizes technical debt in the codebase.
//!
//! 흐름:
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
    pub impact: String,          // 방치 시 영향
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebtCategory {
    Architecture,    // 잘못된 설계, 과도한 결합
    Code,            // 복잡도, 중복, 긴 함수
    Testing,         // 테스트 부재, 낮은 커버리지
    Documentation,   // 문서 없음, 오래된 주석
    Dependencies,    // 오래된 의존성, 취약한 패키지
    Security,        // 보안 설정 부채
    Performance,     // 알려진 성능 문제
    Infrastructure,  // CI/CD, 배포 프로세스 부채
}

impl std::fmt::Display for DebtCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DebtCategory::Architecture   => "🏛️  아키텍처",
            DebtCategory::Code           => "💻 코드 품질",
            DebtCategory::Testing        => "🔬 테스트",
            DebtCategory::Documentation  => "📝 문서화",
            DebtCategory::Dependencies   => "📦 의존성",
            DebtCategory::Security       => "🔒 보안",
            DebtCategory::Performance    => "⚡ 성능",
            DebtCategory::Infrastructure => "🚀 인프라",
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
    pub debt_ratio: String,           // 추정 부채 비율 (기술 부채 / 전체 개발 비용)
}

impl TechDebtReport {
    pub fn render(&self) -> String {
        let by_category = self.grouped_by_category();
        let mut out = vec![
            format!("\n╔══════════════════════════════════════════════════╗"),
            format!("║  📊 기술 부채 분석 보고서                         ║"),
            format!("╚══════════════════════════════════════════════════╝"),
            format!("경로: {}  |  총 추정: {}일  |  부채 비율: {}",
                self.project_path, self.total_estimated_days, self.debt_ratio),
        ];

        for (cat, items) in &by_category {
            let days: f32 = items.iter().map(|i| i.estimated_days).sum();
            out.push(format!("\n── {} ({} 항목, {}일) ──", cat, items.len(), days));
            for item in items {
                out.push(format!("  [{:>4}] [{}] {}{}",
                    item.priority,
                    format!("{:.1}일", item.estimated_days),
                    item.title,
                    item.file.as_deref().map(|f| format!(" ({})", f)).unwrap_or_default(),
                ));
            }
        }

        if !self.recommended_priority.is_empty() {
            out.push("\n── 우선 해결 순서 ──".to_string());
            for (i, p) in self.recommended_priority.iter().enumerate() {
                out.push(format!("  {}. {}", i+1, p));
            }
        }

        if !self.repayment_plan.is_empty() {
            out.push("\n── 상환 계획 ──".to_string());
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
    on_progress(&format!("\n📊 ══ 기술 부채 분석 시작: {} ══", project_path));

    // 분석용 임시 스토리
    let mut story = UserStory::new(
        "TD-ANALYSIS",
        &format!("[기술 부채 분석] {}", project_path),
        &format!("프로젝트 {} 의 기술 부채를 분석합니다.", project_path),
        Priority::High, 5,
    );
    story.add_acceptance_criteria("모든 카테고리 부채 식별");
    story.add_acceptance_criteria("우선순위 및 상환 계획 수립");

    let base_ctx = format!(
        "## 분석 대상\n경로: {}\n\n\
         프로젝트 파일을 직접 읽고(read_file, glob_files, grep_files) 실제 코드를 분석하세요.\n\
         web_search로 해당 기술의 현재 베스트 프랙티스와 비교하세요.",
        project_path
    );

    // ── 1단계: Architect — 구조/설계 부채 ──────────────────────────────────
    print_td_divider("1/4 · Architect — 구조/설계 부채");
    on_progress("🏛️  Architect: 아키텍처 부채 분석 중...");

    let arch_ctx = format!("{}\n\n다음을 분석하세요:\n\
        - 모듈 간 결합도 (순환 의존성, 과도한 결합)\n\
        - SOLID 원칙 위반\n\
        - 도메인 로직 누수\n\
        - 잘못된 추상화 레벨\n\
        JSON: {{\"debt_items\": [{{\"category\": \"Architecture\", \"title\": \"...\", \
        \"description\": \"...\", \"file\": \"...\", \"estimated_days\": 2.0, \"priority\": \"High\", \"impact\": \"...\"}}]}}",
        base_ctx);
    let arch_output = run_agile_agent(client, AgileRole::Architect, &story, &arch_ctx, &hub, &on_progress).await;

    // ── 2단계: Developer — 코드 수준 부채 ───────────────────────────────────
    print_td_divider("2/4 · Developer — 코드 품질 부채");
    on_progress("💻 Developer: 코드 품질 부채 분석 중...");

    let dev_ctx = format!("{}\n\n다음을 분석하세요:\n\
        - 복잡도 높은 함수 (McCabe 복잡도 > 10)\n\
        - 코드 중복 (DRY 위반)\n\
        - 매직 넘버/문자열\n\
        - 에러 처리 부재\n\
        - 테스트 없는 핵심 로직\n\
        - 오래된 의존성 (cargo audit 등)\n\
        JSON: {{\"debt_items\": [...]}}",
        base_ctx);
    let dev_output = run_agile_agent(client, AgileRole::Developer, &story, &dev_ctx, &hub, &on_progress).await;

    // ── 3단계: Reviewer — 품질/테스트 부채 ──────────────────────────────────
    print_td_divider("3/4 · Reviewer — 품질/테스트 부채");
    on_progress("👁️  Reviewer: 테스트·문서 부채 분석 중...");

    let rev_ctx = format!("{}\n\n다음을 분석하세요:\n\
        - 테스트 커버리지 (80% 미만 영역)\n\
        - 통합 테스트 부재\n\
        - 문서화 누락 (README, API 문서)\n\
        - 성능 회귀 테스트 부재\n\
        - CI/CD 파이프라인 갭\n\
        JSON: {{\"debt_items\": [...]}}",
        base_ctx);
    let rev_output = run_agile_agent(client, AgileRole::Reviewer, &story, &rev_ctx, &hub, &on_progress).await;

    // ── 4단계: TechLead — 종합 + 우선순위 ──────────────────────────────────
    print_td_divider("4/4 · TechLead — 종합 + 상환 계획");
    on_progress("🎯 TechLead: 기술 부채 종합 및 상환 계획 수립 중...");

    let tl_ctx = format!(
        "다음 세 가지 분석 결과를 종합하여 우선순위와 상환 계획을 수립하세요.\n\n\
         아키텍처 분석:\n{}\n\n\
         코드 분석:\n{}\n\n\
         품질 분석:\n{}\n\n\
         JSON 출력:\n\
         {{\n\
           \"debt_items\": [...모든 항목 통합...],\n\
           \"total_estimated_days\": 15.5,\n\
           \"debt_ratio\": \"기술 부채 비율 설명\",\n\
           \"recommended_priority\": [\"1순위 항목\", \"2순위\"],\n\
           \"repayment_plan\": [\"스프린트 1: ...\", \"스프린트 2: ...\"]\n\
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
            debt_ratio: v["debt_ratio"].as_str().unwrap_or("측정 불가").to_string(),
            debt_items,
        }
    } else {
        TechDebtReport {
            project_path: project_path.to_string(),
            analyzed_at: now,
            debt_items: Vec::new(),
            total_estimated_days: 0.0,
            repayment_plan: vec!["수동 분석 필요".to_string()],
            recommended_priority: Vec::new(),
            debt_ratio: "측정 불가".to_string(),
        }
    }
}

fn parse_category(s: &str) -> DebtCategory {
    match s.to_lowercase().as_str() {
        "architecture" | "아키텍처" => DebtCategory::Architecture,
        "code" | "코드" | "code quality" => DebtCategory::Code,
        "testing" | "테스트" => DebtCategory::Testing,
        "documentation" | "문서" => DebtCategory::Documentation,
        "dependencies" | "의존성" => DebtCategory::Dependencies,
        "security" | "보안" => DebtCategory::Security,
        "performance" | "성능" => DebtCategory::Performance,
        "infrastructure" | "인프라" => DebtCategory::Infrastructure,
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
        assert!(DebtCategory::Architecture.to_string().contains("아키텍처"));
        assert!(DebtCategory::Security.to_string().contains("보안"));
        assert!(DebtCategory::Testing.to_string().contains("테스트"));
    }

    #[test]
    fn test_extract_json_from_code_fence() {
        let text = "설명\n```json\n{\"key\": \"value\"}\n```\n끝";
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
        assert!(rendered.contains("기술 부채 분석 보고서"));
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
