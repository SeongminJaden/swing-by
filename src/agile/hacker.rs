//! 해커 에이전트 — 자체 프로젝트 보안 감사 전문가
//!
//! ⚠️  이 에이전트는 사용자가 직접 만든 프로젝트에만 사용합니다.
//!     외부 시스템에 대한 공격 시도는 절대 하지 않습니다.
//!
//! 역할:
//!   - OWASP Top 10 기준으로 코드를 정적 분석
//!   - cargo audit, semgrep, bandit 등 보안 스캐너 실행
//!   - 취약한 코드 패턴 탐지 (SQL injection, XSS, SSRF 등)
//!   - 의존성 취약점 (CVE) 확인
//!   - PoC(Proof of Concept) 작성으로 실제 영향도 검증
//!   - 상세 보안 리포트 생성
//!   - Developer에게 수정 지시서 전달
//!
//! 루프:
//!   Developer 구현 완료
//!   → HackerAgent 취약점 분석
//!   → 취약점 발견 → Developer 수정
//!   → QAAgent 재검증
//!   → 모두 OK → Done ✅

use crate::agent::ollama::OllamaClient;
use crate::agent::node::{NodeHub, NodeMessage, MsgType};
use crate::agile::board::AgileBoard;
use crate::agile::security::{OwaspCategory, SecurityReport, Severity, Vulnerability};
use crate::agile::story::UserStory;

// ─── 보안 스캐너 정의 ────────────────────────────────────────────────────────

struct ScanResult {
    tool: String,
    output: String,
    #[allow(dead_code)]
    found_issues: bool,
}

/// 사용 가능한 보안 스캐너를 순서대로 실행
async fn run_static_scanners(project_path: &str) -> Vec<ScanResult> {
    let mut results = Vec::new();

    // 1. cargo audit (Rust 의존성 CVE)
    let cargo_result = crate::tools::system::run_shell(
        &format!("cd {} && cargo audit --json 2>/dev/null || cargo audit 2>&1", project_path)
    );
    if let Ok(r) = cargo_result {
        let found = r.stdout.contains("error") || r.stdout.contains("vulnerability")
            || r.stdout.contains("warning");
        results.push(ScanResult {
            tool: "cargo-audit".to_string(),
            output: crate::utils::trunc(&r.stdout, 3000).to_string(),
            found_issues: found,
        });
    }

    // 2. grep으로 위험 패턴 탐지 (빠른 휴리스틱)
    let patterns = [
        ("unsafe", "Unsafe Rust 블록"),
        ("unwrap()", "unwrap() 패닉 위험"),
        ("expect(", "expect() 패닉 위험"),
        ("eval(", "코드 인젝션 위험"),
        ("exec(", "명령어 인젝션 위험"),
        ("sql", "SQL 쿼리 패턴"),
        ("password", "패스워드 하드코딩 의심"),
        ("secret", "시크릿 하드코딩 의심"),
        ("TODO.*security", "보안 관련 TODO 미완성"),
    ];
    for (pat, desc) in &patterns {
        if let Ok(found) = crate::tools::grep_files(pat, project_path) {
            if !found.is_empty() {
                let out = found.iter().take(10)
                    .map(|l| format!("  {}", l))
                    .collect::<Vec<_>>().join("\n");
                results.push(ScanResult {
                    tool: format!("grep:{}", pat),
                    output: format!("패턴 '{}' ({}) — {} 곳 발견:\n{}", pat, desc, found.len(), out),
                    found_issues: true,
                });
            }
        }
    }

    // 3. semgrep (설치된 경우)
    let semgrep = crate::tools::system::run_shell(
        &format!("cd {} && semgrep --config=auto --json . 2>/dev/null | head -c 4096", project_path)
    );
    if let Ok(r) = semgrep {
        if !r.stdout.is_empty() && !r.stdout.contains("command not found") {
            let found = r.stdout.contains("\"results\":[{");
            results.push(ScanResult {
                tool: "semgrep".to_string(),
                output: crate::utils::trunc(&r.stdout, 2000).to_string(),
                found_issues: found,
            });
        }
    }

    // 4. bandit (Python 프로젝트용)
    let bandit = crate::tools::system::run_shell(
        &format!("cd {} && bandit -r . -f txt 2>/dev/null | head -c 3000", project_path)
    );
    if let Ok(r) = bandit {
        if !r.stdout.is_empty() && !r.stdout.contains("command not found") {
            let found = r.stdout.contains("Issue:");
            results.push(ScanResult {
                tool: "bandit".to_string(),
                output: crate::utils::trunc(&r.stdout, 2000).to_string(),
                found_issues: found,
            });
        }
    }

    // 5. npm audit (Node.js 프로젝트용)
    let npm_audit = crate::tools::system::run_shell(
        &format!("cd {} && npm audit --json 2>/dev/null | head -c 3000", project_path)
    );
    if let Ok(r) = npm_audit {
        if !r.stdout.is_empty() && !r.stdout.contains("ENOENT") {
            let found = r.stdout.contains("\"vulnerabilities\":{") && !r.stdout.contains("\"total\":0");
            results.push(ScanResult {
                tool: "npm-audit".to_string(),
                output: crate::utils::trunc(&r.stdout, 2000).to_string(),
                found_issues: found,
            });
        }
    }

    // 6. 파일 권한 / 환경변수 하드코딩 확인
    let env_check = crate::tools::system::run_shell(
        &format!(
            r#"cd {} && grep -rn --include="*.rs" --include="*.py" --include="*.js" \
               -E '(password|secret|api_key|token)\s*=\s*"[^"]+"|API_KEY\s*=\s*"' . 2>/dev/null | head -20"#,
            project_path
        )
    );
    if let Ok(r) = env_check {
        if !r.stdout.trim().is_empty() {
            results.push(ScanResult {
                tool: "secret-scan".to_string(),
                output: format!("하드코딩된 시크릿 의심:\n{}", r.stdout),
                found_issues: true,
            });
        }
    }

    results
}

// ─── 해커 에이전트 시스템 프롬프트 ──────────────────────────────────────────

fn hacker_system_prompt(story: &UserStory, scan_results: &str) -> String {
    format!(
        "당신은 화이트햇 보안 전문가(Penetration Tester)입니다.\n\
         ⚠️  당신의 임무는 사용자가 직접 만든 프로젝트의 보안을 검사하는 것입니다.\n\
         외부 시스템, 타인의 시스템에 대한 공격 시도는 절대 수행하지 않습니다.\n\n\
         분석 대상: 스토리 [{}] — {}\n\n\
         수행할 작업:\n\
         1. 🔍 정적 분석 결과를 검토하고 추가 코드 패턴 분석\n\
         2. 🧪 OWASP Top 10 기준으로 각 카테고리 체크\n\
         3. 💉 인젝션 취약점 (SQL, Command, LDAP, XSS, XXE)\n\
         4. 🔐 인증/세션 관리 취약점\n\
         5. 🔓 접근 제어 문제\n\
         6. 📦 의존성 CVE 확인\n\
         7. 🔑 하드코딩된 시크릿, 키, 패스워드\n\
         8. 🌐 SSRF, CSRF, Open Redirect\n\
         9. 📝 민감 정보 노출 (로그, 에러 메시지)\n\
         10. ⚡ Race Condition, Buffer Overflow, Use-after-free\n\n\
         정적 분석 결과:\n{}\n\n\
         출력 형식: JSON\n\
         {{\n\
           \"vulnerabilities\": [\n\
             {{\n\
               \"id\": \"V-1\",\n\
               \"title\": \"취약점 이름\",\n\
               \"severity\": \"Critical|High|Medium|Low|Info\",\n\
               \"owasp\": \"A03: Injection\",\n\
               \"file\": \"src/main.rs\",\n\
               \"line\": 42,\n\
               \"code_snippet\": \"취약한 코드\",\n\
               \"description\": \"상세 설명\",\n\
               \"impact\": \"악용 시 영향\",\n\
               \"attack_vector\": \"공격 방법\",\n\
               \"proof_of_concept\": \"PoC 코드 또는 명령어\",\n\
               \"fix_suggestion\": \"구체적인 수정 방법\",\n\
               \"fix_priority\": 1\n\
             }}\n\
           ],\n\
           \"scan_summary\": \"전체 요약\",\n\
           \"overall_risk\": \"Critical|High|Medium|Low|Info\",\n\
           \"passed\": false,\n\
           \"executive_summary\": \"경영진용 한 줄 요약\"\n\
         }}",
        story.id, story.title,
        crate::utils::trunc(scan_results, 4000)
    )
}

// ─── 보안 리포트 Parsing ────────────────────────────────────────────────────────

fn parse_security_report(
    json_text: &str,
    story_id: &str,
    round: usize,
    report_id: &str,
) -> SecurityReport {
    let mut report = SecurityReport::new(report_id, story_id, round, "프로젝트 소스코드");

    let v = match extract_json(json_text) {
        Some(v) => v,
        None => {
            // JSON Parsing 실패 시 텍스트에서 패스 여부 추론
            let passed = json_text.to_uppercase().contains("NO VULNERABILITY")
                || json_text.contains("\"passed\": true")
                || json_text.contains("취약점 없음");
            report.passed = passed;
            report.summary = crate::utils::trunc(json_text, 500).to_string();
            return report;
        }
    };

    // 취약점 Parsing
    if let Some(vulns) = v["vulnerabilities"].as_array() {
        for (i, vj) in vulns.iter().enumerate() {
            let severity = parse_severity(vj["severity"].as_str().unwrap_or("Medium"));
            let owasp = parse_owasp(vj["owasp"].as_str().unwrap_or("Other"));
            let vid = vj["id"].as_str().unwrap_or(&format!("V-{}", i+1)).to_string();
            let title = vj["title"].as_str().unwrap_or("Unknown").to_string();
            let desc = vj["description"].as_str().unwrap_or("").to_string();
            let fix = vj["fix_suggestion"].as_str().unwrap_or("코드 검토 필요").to_string();

            let mut vuln = Vulnerability::new(&vid, &title, severity, owasp, &desc, &fix);
            vuln.file = vj["file"].as_str().map(|s| s.to_string());
            vuln.line = vj["line"].as_u64().map(|n| n as u32);
            vuln.code_snippet = vj["code_snippet"].as_str().map(|s| s.to_string());
            vuln.impact = vj["impact"].as_str().unwrap_or("").to_string();
            vuln.attack_vector = vj["attack_vector"].as_str().unwrap_or("").to_string();
            vuln.proof_of_concept = vj["proof_of_concept"].as_str().map(|s| s.to_string());
            vuln.fix_priority = vj["fix_priority"].as_u64().unwrap_or(3).min(5) as u8;
            report.add_vuln(vuln);
        }
    }

    report.summary = v["scan_summary"].as_str().unwrap_or("").to_string();
    report.executive_summary = v["executive_summary"].as_str().unwrap_or("").to_string();
    report.passed = v["passed"].as_bool().unwrap_or(false) || report.vulnerabilities.is_empty();

    report
}

// ─── 해커 에이전트 메인 ──────────────────────────────────────────────────────

pub struct HackerAgentOutput {
    pub report: SecurityReport,
    pub fix_instructions: String,
}

/// 단일 보안 스캔 라운드 실행
pub async fn run_hacker_agent(
    client: &OllamaClient,
    story: &UserStory,
    project_path: &str,
    round: usize,
    hub: &NodeHub,
    on_progress: &impl Fn(&str),
) -> HackerAgentOutput {
    use crate::models::Message;

    on_progress(&format!("🔒 HackerAgent [{}] 보안 스캔 시작 (라운드 {})...", story.id, round));

    // 1단계: 정적 스캐너 실행
    on_progress("  🔍 정적 분석 도구 실행 중...");
    let scan_results = run_static_scanners(project_path).await;
    on_progress(&format!("  📊 스캐너 {}개 실행 완료", scan_results.len()));

    let scan_text = scan_results.iter()
        .map(|r| format!("=== {} ===\n{}", r.tool, r.output))
        .collect::<Vec<_>>().join("\n\n");

    // 2단계: AI 보안 분석
    on_progress("  🧠 AI 보안 분석 중...");
    let system = hacker_system_prompt(story, &scan_text);
    let user_msg = format!(
        "다음 프로젝트의 보안 취약점을 분석하세요.\n\
         🔍 먼저 web_search로 이 유형의 프로젝트에 알려진 취약점 패턴을 검색하세요.\n\
         📂 프로젝트 경로: {}\n\n\
         구현 내용:\n{}\n\n\
         정적 분석 결과:\n{}",
        project_path,
        crate::utils::trunc(story.implementation.as_deref().unwrap_or("구현 없음"), 3000),
        crate::utils::trunc(&scan_text, 2000),
    );

    let mut history = vec![
        Message::system(&format!("모델: {}\n\n{}\n\n{}", client.model(),
            crate::agent::tools::tool_descriptions(), system)),
        Message::user(&user_msg),
    ];

    // HackerAgent 시작을 노드 허브에 알림
    let _ = hub.send(NodeMessage {
        from: "HackerAgent".to_string(),
        to: String::new(),
        msg_type: MsgType::Status,
        content: format!("[{}] 보안 스캔 라운드 {} 시작", story.id, round),
        metadata: Default::default(),
    }).await;

    let mut ai_output = String::new();
    let max_turns = 12usize;

    for turn in 0..max_turns {
        let ai_text = match client.chat_stream(history.clone(), |_| {}).await {
            Ok(t) => t,
            Err(e) => { ai_output = format!("AI 오류: {}", e); break; }
        };

        match crate::agent::chat::parse_response_pub(&ai_text) {
            crate::models::AgentResponse::Exit | crate::models::AgentResponse::Text(_) => {
                history.push(Message::assistant(&ai_text));
                ai_output = ai_text;
                break;
            }
            crate::models::AgentResponse::ToolCall(tc) if tc.name == "__multi__" => {
                history.push(Message::assistant(&ai_text));
                let mut results = Vec::new();
                for raw in &tc.args {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
                    let name = val["name"].as_str().unwrap_or("").to_string();
                    let args: Vec<String> = val["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    on_progress(&format!("  🔧 [HackerAgent] {}...", name));
                    let result = crate::agent::tools::dispatch_tool(
                        &crate::models::ToolCall { name: name.clone(), args }
                    ).await;
                    results.push(format!("툴 '{}' 결과:\n{}", name, result.output));
                }
                history.push(Message::tool(results.join("\n\n")));
                if turn == max_turns - 1 { ai_output = results.join("\n\n"); }
            }
            crate::models::AgentResponse::ToolCall(tc) => {
                on_progress(&format!("  🔧 [HackerAgent] {}...", tc.name));
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("툴 '{}' 결과:\n{}", tc.name, result.output)));
                if turn == max_turns - 1 { ai_output = result.output; }
            }
        }
    }

    // 리포트 Parsing
    let report_id = format!("SEC-{}-R{}", story.id, round);
    let mut report = parse_security_report(&ai_output, &story.id, round, &report_id);
    report.scan_tools_used = scan_results.iter().map(|r| r.tool.clone()).collect();

    let fix_instructions = report.fix_instructions();

    // 결과를 Developer 노드에 전달
    let _ = hub.send(NodeMessage {
        from: "HackerAgent".to_string(),
        to: "Developer".to_string(),
        msg_type: MsgType::Result,
        content: fix_instructions.clone(),
        metadata: Default::default(),
    }).await;

    // QA 에이전트에도 공유
    let _ = hub.send(NodeMessage {
        from: "HackerAgent".to_string(),
        to: "QAEngineer".to_string(),
        msg_type: MsgType::Status,
        content: format!("[{}] 보안 스캔 라운드 {} 완료 — 취약점 {}개",
            story.id, round, report.vulnerabilities.len()),
        metadata: Default::default(),
    }).await;

    on_progress(&format!(
        "  {} 보안 스캔 완료 — {} (취약점 {}개, Critical {}개, High {}개)",
        if report.passed { "✅" } else { "🚨" },
        report.overall_risk,
        report.vulnerabilities.len(),
        report.critical_count(),
        report.high_count(),
    ));

    HackerAgentOutput { report, fix_instructions }
}

// ─── 보안 수정 루프 ──────────────────────────────────────────────────────────

const MAX_SECURITY_ROUNDS: usize = 5;

pub struct SecurityFixResult {
    pub rounds: usize,
    pub final_report: SecurityReport,
    pub approved: bool,
}

/// HackerAgent + Developer 보안 수정 루프
/// QA와 HackerAgent 모두 OK할 때까지 반복
pub async fn run_security_fix_loop(
    client: &OllamaClient,
    board: &AgileBoard,
    hub: &NodeHub,
    story_id: &str,
    project_path: &str,
    on_progress: impl Fn(&str) + Clone,
) -> SecurityFixResult {
    let mut round = 0usize;
    let mut last_report = SecurityReport::new("SEC-0", story_id, 0, project_path);

    loop {
        round += 1;
        on_progress(&format!("\n🔒 ═══ 보안 라운드 {}/{} ═══", round, MAX_SECURITY_ROUNDS));

        let story = match board.get_story(story_id) {
            Some(s) => s,
            None => break,
        };

        // HackerAgent 실행
        let hack_output = run_hacker_agent(
            client, &story, project_path, round, hub, &on_progress
        ).await;

        // 리포트를 보드에 저장
        board.update_story_field(story_id, "HackerAgent", |s| {
            let report_text = format!(
                "=== 보안 리포트 라운드 {} ===\n{}", round, hack_output.report.render()
            );
            // qa_report 필드에 보안 리포트 추가
            let prev = s.qa_report.take().unwrap_or_default();
            s.qa_report = Some(format!("{}\n\n{}", prev, report_text));
        }).ok();

        last_report = hack_output.report.clone();

        if hack_output.report.passed {
            on_progress("✅ HackerAgent: 취약점 없음 — 보안 승인");
            break;
        }

        if round >= MAX_SECURITY_ROUNDS {
            on_progress(&format!("⚠️ 최대 보안 라운드({}) 도달 — 미수정 취약점 {}개 존재",
                MAX_SECURITY_ROUNDS, hack_output.report.unfixed_count()));
            break;
        }

        // Developer에게 수정 지시
        on_progress(&format!("🔁 Developer에게 보안 수정 지시 ({}개 취약점)...",
            hack_output.report.unfixed_count()));
        on_progress(&format!("{}", hack_output.report.render()));

        let story = match board.get_story(story_id) { Some(s) => s, None => break };

        // Developer 보안 수정 실행
        let dev_ctx = format!(
            "## 보안 수정 지시서 (라운드 {})\n{}\n\n## 현재 구현\n{}",
            round,
            hack_output.fix_instructions,
            crate::utils::trunc(story.implementation.as_deref().unwrap_or(""), 2000),
        );

        on_progress(&format!("💻 Developer: 보안 취약점 수정 중 (라운드 {})...", round));
        let dev_output = run_security_developer(
            client, &story, &dev_ctx, hub, &on_progress
        ).await;

        board.update_story_field(story_id, "Developer", |s| {
            s.implementation = Some(dev_output.clone());
        }).ok();

        on_progress(&format!("  ✍️  Developer 수정 완료 (라운드 {})", round));
    }

    SecurityFixResult {
        rounds: round,
        final_report: last_report.clone(),
        approved: last_report.passed,
    }
}

/// 보안 수정 전담 Developer 실행
async fn run_security_developer(
    client: &OllamaClient,
    story: &UserStory,
    security_ctx: &str,
    hub: &NodeHub,
    on_progress: &impl Fn(&str),
) -> String {
    use crate::models::Message;

    let system = format!(
        "모델: {}\n\n{}\n\n\
         당신은 보안 수정 전문 개발자입니다.\n\
         주어진 보안 취약점 리포트를 바탕으로 코드를 수정하세요.\n\n\
         수정 원칙:\n\
         - 모든 취약점을 OWASP 권장 방법으로 수정\n\
         - Parameterized Query / Prepared Statement 사용 (SQL Injection)\n\
         - 입력값 검증 및 인코딩 (XSS)\n\
         - 최소 권한 원칙 적용\n\
         - 시크릿은 환경변수 또는 Secret Manager로 이동\n\
         - 보안 헤더 추가 (HSTS, CSP, X-Frame-Options)\n\
         - 에러 메시지에서 민감 정보 제거\n\n\
         수정 후 반드시:\n\
         1. 빌드 확인\n\
         2. 기존 테스트 통과 확인\n\
         3. 수정된 취약점 목록 정리",
        client.model(),
        crate::agent::tools::tool_descriptions(),
    );

    let user_msg = format!(
        "다음 보안 취약점을 수정하세요:\n\n{}\n\n\
         수정 전 web_search로 각 취약점의 최신 수정 방법을 검색하세요.",
        security_ctx
    );

    let mut history = vec![
        Message::system(&system),
        Message::user(&user_msg),
    ];

    let _ = hub.send(NodeMessage {
        from: "Developer".to_string(), to: "HackerAgent".to_string(),
        msg_type: MsgType::Status,
        content: format!("[{}] 보안 수정 작업 시작", story.id),
        metadata: Default::default(),
    }).await;

    let mut final_output = String::new();

    for turn in 0..20 {
        let ai_text = match client.chat_stream(history.clone(), |_| {}).await {
            Ok(t) => t,
            Err(e) => return format!("오류: {}", e),
        };
        match crate::agent::chat::parse_response_pub(&ai_text) {
            crate::models::AgentResponse::Exit | crate::models::AgentResponse::Text(_) => {
                final_output = ai_text.clone();
                history.push(Message::assistant(&ai_text));
                break;
            }
            crate::models::AgentResponse::ToolCall(tc) if tc.name == "__multi__" => {
                history.push(Message::assistant(&ai_text));
                let mut results = Vec::new();
                for raw in &tc.args {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
                    let name = val["name"].as_str().unwrap_or("").to_string();
                    let args: Vec<String> = val["args"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                    on_progress(&format!("  🔧 [SecDev] {}...", name));
                    let result = crate::agent::tools::dispatch_tool(
                        &crate::models::ToolCall { name: name.clone(), args }
                    ).await;
                    results.push(format!("툴 '{}' 결과:\n{}", name, result.output));
                }
                history.push(Message::tool(results.join("\n\n")));
                if turn == 19 { final_output = results.join("\n\n"); }
            }
            crate::models::AgentResponse::ToolCall(tc) => {
                on_progress(&format!("  🔧 [SecDev] {}...", tc.name));
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("툴 '{}' 결과:\n{}", tc.name, result.output)));
                if turn == 19 { final_output = result.output; }
            }
        }
    }

    final_output
}

// ─── Helpers ────────────────────────────────────────────────────────────────────

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high"     => Severity::High,
        "low"      => Severity::Low,
        "info"     => Severity::Info,
        _          => Severity::Medium,
    }
}

fn parse_owasp(s: &str) -> OwaspCategory {
    if s.contains("A01") || s.to_lowercase().contains("access control") {
        OwaspCategory::A01BrokenAccessControl
    } else if s.contains("A02") || s.to_lowercase().contains("crypt") {
        OwaspCategory::A02CryptographicFailures
    } else if s.contains("A03") || s.to_lowercase().contains("inject") {
        OwaspCategory::A03Injection
    } else if s.contains("A04") || s.to_lowercase().contains("insecure design") {
        OwaspCategory::A04InsecureDesign
    } else if s.contains("A05") || s.to_lowercase().contains("misconfig") {
        OwaspCategory::A05SecurityMisconfiguration
    } else if s.contains("A06") || s.to_lowercase().contains("component") {
        OwaspCategory::A06VulnerableComponents
    } else if s.contains("A07") || s.to_lowercase().contains("auth") {
        OwaspCategory::A07AuthenticationFailures
    } else if s.contains("A08") || s.to_lowercase().contains("integrit") {
        OwaspCategory::A08IntegrityFailures
    } else if s.contains("A09") || s.to_lowercase().contains("log") {
        OwaspCategory::A09LoggingFailures
    } else if s.contains("A10") || s.to_lowercase().contains("ssrf") {
        OwaspCategory::A10ServerSideRequestForgery
    } else {
        OwaspCategory::Other(s.to_string())
    }
}

fn extract_json(text: &str) -> Option<serde_json::Value> {
    let candidate = if let Some(s) = text.find("```json") {
        let after = &text[s + 7..];
        if let Some(e) = after.find("```") { &after[..e] } else { after }
    } else if let Some(s) = text.find('{') {
        if let Some(e) = text.rfind('}') { &text[s..=e] } else { return None }
    } else {
        return None;
    };
    serde_json::from_str(candidate.trim()).ok()
}
