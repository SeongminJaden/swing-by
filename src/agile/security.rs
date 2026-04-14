//! 보안 취약점 Data structures
//!
//! OWASP Top 10 기반 취약점 분류와
//! 해커 에이전트가 생성하는 보안 리포트를 정의합니다.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ─── 취약점 심각도 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn icon(&self) -> &'static str {
        match self {
            Severity::Info     => "ℹ️ ",
            Severity::Low      => "🟢",
            Severity::Medium   => "🟡",
            Severity::High     => "🟠",
            Severity::Critical => "🔴",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info     => "Info",
            Severity::Low      => "Low",
            Severity::Medium   => "Medium",
            Severity::High     => "High",
            Severity::Critical => "Critical",
        }
    }
    pub fn cvss_range(&self) -> &'static str {
        match self {
            Severity::Info     => "0.0",
            Severity::Low      => "0.1-3.9",
            Severity::Medium   => "4.0-6.9",
            Severity::High     => "7.0-8.9",
            Severity::Critical => "9.0-10.0",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.label())
    }
}

// ─── OWASP Top 10 카테고리 ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OwaspCategory {
    A01BrokenAccessControl,
    A02CryptographicFailures,
    A03Injection,
    A04InsecureDesign,
    A05SecurityMisconfiguration,
    A06VulnerableComponents,
    A07AuthenticationFailures,
    A08IntegrityFailures,
    A09LoggingFailures,
    A10ServerSideRequestForgery,
    Other(String),
}

impl OwaspCategory {
    pub fn label(&self) -> String {
        match self {
            Self::A01BrokenAccessControl      => "A01: Broken Access Control".to_string(),
            Self::A02CryptographicFailures     => "A02: Cryptographic Failures".to_string(),
            Self::A03Injection                 => "A03: Injection".to_string(),
            Self::A04InsecureDesign            => "A04: Insecure Design".to_string(),
            Self::A05SecurityMisconfiguration  => "A05: Security Misconfiguration".to_string(),
            Self::A06VulnerableComponents      => "A06: Vulnerable & Outdated Components".to_string(),
            Self::A07AuthenticationFailures    => "A07: Auth & Session Failures".to_string(),
            Self::A08IntegrityFailures         => "A08: Software & Data Integrity Failures".to_string(),
            Self::A09LoggingFailures           => "A09: Security Logging & Monitoring Failures".to_string(),
            Self::A10ServerSideRequestForgery  => "A10: SSRF".to_string(),
            Self::Other(s)                     => s.clone(),
        }
    }
}

// ─── 개별 취약점 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub owasp: OwaspCategory,
    pub cve: Option<String>,          // CVE-YYYY-XXXXX
    pub cvss_score: Option<f32>,
    // 위치
    pub file: Option<String>,
    pub line: Option<u32>,
    pub code_snippet: Option<String>,
    // 설명
    pub description: String,
    pub impact: String,               // 악용 시 영향
    pub attack_vector: String,        // 어떻게 공격하는가
    pub proof_of_concept: Option<String>, // PoC 코드/명령
    // 수정
    pub fix_suggestion: String,
    pub fix_priority: u8,             // 1(즉시) ~ 5(낮음)
    pub fixed: bool,
    pub fix_commit: Option<String>,
}

impl Vulnerability {
    pub fn new(
        id: &str,
        title: &str,
        severity: Severity,
        owasp: OwaspCategory,
        description: &str,
        fix: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            severity,
            owasp,
            cve: None,
            cvss_score: None,
            file: None,
            line: None,
            code_snippet: None,
            description: description.to_string(),
            impact: String::new(),
            attack_vector: String::new(),
            proof_of_concept: None,
            fix_suggestion: fix.to_string(),
            fix_priority: 3,
            fixed: false,
            fix_commit: None,
        }
    }
}

// ─── 보안 리포트 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub id: String,
    pub story_id: String,
    pub round: usize,               // 몇 번째 해킹 시도
    pub created_at: u64,
    pub vulnerabilities: Vec<Vulnerability>,
    pub scan_tools_used: Vec<String>,
    pub scope: String,              // 스캔 대상 (파일, URL 등)
    pub overall_risk: Severity,
    pub passed: bool,               // 취약점 없음 = true
    pub summary: String,
    pub executive_summary: String,  // 경영진용 요약
}

impl SecurityReport {
    pub fn new(id: &str, story_id: &str, round: usize, scope: &str) -> Self {
        Self {
            id: id.to_string(),
            story_id: story_id.to_string(),
            round,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            vulnerabilities: Vec::new(),
            scan_tools_used: Vec::new(),
            scope: scope.to_string(),
            overall_risk: Severity::Info,
            passed: true,
            summary: String::new(),
            executive_summary: String::new(),
        }
    }

    pub fn add_vuln(&mut self, v: Vulnerability) {
        if v.severity > self.overall_risk {
            self.overall_risk = v.severity.clone();
        }
        self.passed = false;
        self.vulnerabilities.push(v);
    }

    pub fn critical_count(&self) -> usize {
        self.vulnerabilities.iter().filter(|v| v.severity == Severity::Critical).count()
    }

    pub fn high_count(&self) -> usize {
        self.vulnerabilities.iter().filter(|v| v.severity == Severity::High).count()
    }

    pub fn unfixed_count(&self) -> usize {
        self.vulnerabilities.iter().filter(|v| !v.fixed).count()
    }

    /// 터미널 출력용 렌더링
    pub fn render(&self) -> String {
        let mut out = Vec::new();
        out.push(format!(
            "\n╔══════════════════════════════════════════════════╗\n\
             ║  🔒 보안 리포트 #{} — 스토리 [{}] (라운드 {})    \n\
             ╚══════════════════════════════════════════════════╝",
            self.id, self.story_id, self.round
        ));
        out.push(format!(
            "전체 위험도: {}  |  취약점: {}개  |  {}",
            self.overall_risk,
            self.vulnerabilities.len(),
            if self.passed { "✅ 통과" } else { "❌ 보안 이슈 발견" }
        ));

        if !self.vulnerabilities.is_empty() {
            out.push("\n── 취약점 목록 ──".to_string());
            for v in &self.vulnerabilities {
                let fixed_mark = if v.fixed { " [수정완료]" } else { "" };
                out.push(format!(
                    "  {} [{}] {}{}\n     위치: {}\n     영향: {}\n     수정: {}",
                    v.severity,
                    v.id,
                    v.title,
                    fixed_mark,
                    v.file.as_deref().unwrap_or("N/A"),
                    crate::utils::trunc(&v.impact, 100),
                    crate::utils::trunc(&v.fix_suggestion, 100),
                ));
            }
        }

        if !self.summary.is_empty() {
            out.push(format!("\n── 요약 ──\n{}", self.summary));
        }
        out.push(String::new());
        out.join("\n")
    }

    /// Developer에게 전달할 수정 지시서 생성
    pub fn fix_instructions(&self) -> String {
        if self.passed { return "보안 취약점 없음 — 추가 수정 불필요".to_string(); }

        let mut lines = vec![
            format!("## 🔒 보안 수정 지시서 (라운드 {})", self.round),
            format!("전체 위험도: {}  |  미수정 취약점: {}개\n", self.overall_risk, self.unfixed_count()),
        ];

        // 우선순위 순으로 정렬
        let mut vulns: Vec<&Vulnerability> = self.vulnerabilities.iter()
            .filter(|v| !v.fixed).collect();
        vulns.sort_by(|a, b| a.fix_priority.cmp(&b.fix_priority)
            .then(b.severity.cmp(&a.severity)));

        for (i, v) in vulns.iter().enumerate() {
            lines.push(format!(
                "### {}. {} [{}] {}",
                i + 1, v.severity, v.id, v.title
            ));
            lines.push(format!("**OWASP**: {}", v.owasp.label()));
            if let Some(file) = &v.file {
                let line_str = v.line.map(|l| format!(":{}", l)).unwrap_or_default();
                lines.push(format!("**위치**: `{}{}`", file, line_str));
            }
            if let Some(poc) = &v.proof_of_concept {
                lines.push(format!("**PoC**: `{}`", crate::utils::trunc(poc, 200)));
            }
            lines.push(format!("**영향**: {}", v.impact));
            lines.push(format!("**수정 방법**: {}", v.fix_suggestion));
            if let Some(code) = &v.code_snippet {
                lines.push(format!("**취약 코드**:\n```\n{}\n```", crate::utils::trunc(code, 300)));
            }
            lines.push(String::new());
        }

        lines.push("⚠️ 모든 취약점을 수정한 후 빌드 및 테스트를 다시 실행하세요.".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn report_overall_risk_updates() {
        let mut report = SecurityReport::new("SR-1", "US-1", 1, ".");
        assert_eq!(report.overall_risk, Severity::Info);
        assert!(report.passed);

        report.add_vuln(Vulnerability::new(
            "V-1", "SQL Injection", Severity::Critical,
            OwaspCategory::A03Injection, "desc", "fix"
        ));
        assert_eq!(report.overall_risk, Severity::Critical);
        assert!(!report.passed);
    }

    #[test]
    fn fix_instructions_filters_unfixed() {
        let mut report = SecurityReport::new("SR-2", "US-2", 1, ".");
        let mut v = Vulnerability::new("V-1", "XSS", Severity::High, OwaspCategory::A03Injection, "d", "f");
        v.fixed = true;
        report.add_vuln(v);
        report.add_vuln(Vulnerability::new("V-2", "SSRF", Severity::Medium,
            OwaspCategory::A10ServerSideRequestForgery, "d", "f"));
        let instructions = report.fix_instructions();
        assert!(instructions.contains("V-2"));
        assert!(!instructions.contains("V-1"));
    }
}
