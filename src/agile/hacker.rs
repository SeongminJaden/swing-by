//! Hacker agent — security auditor for your own project
//!
//! ⚠️  This agent is used only for projects you built yourself.
//!     It never attempts attacks on external systems.
//!
//! Role:
//!   - Static analysis of code against OWASP Top 10
//!   - Run security scanners: cargo audit, semgrep, bandit, etc.
//!   - Detect vulnerable code patterns (SQL injection, XSS, SSRF, etc.)
//!   - Check dependency vulnerabilities (CVE)
//!   - Write PoC (Proof of Concept) to verify real-world impact
//!   - Generate detailed security reports
//!   - Deliver fix instructions to Developer
//!
//! Loop:
//!   Developer implementation complete
//!   → HackerAgent vulnerability analysis
//!   → Vulnerabilities found → Developer fixes
//!   → QAAgent re-verification
//!   → All OK → Done ✅

use crate::agent::ollama::OllamaClient;
use crate::agent::node::{NodeHub, NodeMessage, MsgType};
use crate::agile::board::AgileBoard;
use crate::agile::security::{OwaspCategory, SecurityReport, Severity, Vulnerability};
use crate::agile::story::UserStory;

// ─── Security scanner definitions ─────────────────────────────────────────────

struct ScanResult {
    tool: String,
    output: String,
    #[allow(dead_code)]
    found_issues: bool,
}

/// Run available security scanners in sequence
async fn run_static_scanners(project_path: &str) -> Vec<ScanResult> {
    let mut results = Vec::new();

    // 1. cargo audit (Rust dependency CVEs)
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

    // 2. grep for dangerous patterns (quick heuristics)
    let patterns = [
        ("unsafe", "Unsafe Rust block"),
        ("unwrap()", "unwrap() panic risk"),
        ("expect(", "expect() panic risk"),
        ("eval(", "code injection risk"),
        ("exec(", "command injection risk"),
        ("sql", "SQL query pattern"),
        ("password", "suspected hardcoded password"),
        ("secret", "suspected hardcoded secret"),
        ("TODO.*security", "incomplete security TODO"),
    ];
    for (pat, desc) in &patterns {
        if let Ok(found) = crate::tools::grep_files(pat, project_path) {
            if !found.is_empty() {
                let out = found.iter().take(10)
                    .map(|l| format!("  {}", l))
                    .collect::<Vec<_>>().join("\n");
                results.push(ScanResult {
                    tool: format!("grep:{}", pat),
                    output: format!("Pattern '{}' ({}) — {} occurrence(s) found:\n{}", pat, desc, found.len(), out),
                    found_issues: true,
                });
            }
        }
    }

    // 3. semgrep (if installed)
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

    // 4. bandit (for Python projects)
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

    // 5. npm audit (for Node.js projects)
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

    // 6. File permissions / hardcoded env var check
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
                output: format!("Suspected hardcoded secrets:\n{}", r.stdout),
                found_issues: true,
            });
        }
    }

    results
}

// ─── Hacker agent system prompt ────────────────────────────────────────────

fn hacker_system_prompt(story: &UserStory, scan_results: &str) -> String {
    format!(
        "You are a white-hat security expert (Penetration Tester).\n\
         ⚠️  Your mission is to audit the security of a project built by the user.\n\
         You never attempt attacks on external or third-party systems.\n\n\
         Target: Story [{}] — {}\n\n\
         Tasks to perform:\n\
         1. 🔍 Review static analysis results and analyze additional code patterns\n\
         2. 🧪 Check each OWASP Top 10 category\n\
         3. 💉 Injection vulnerabilities (SQL, Command, LDAP, XSS, XXE)\n\
         4. 🔐 Authentication/session management vulnerabilities\n\
         5. 🔓 Access control issues\n\
         6. 📦 Dependency CVE checks\n\
         7. 🔑 Hardcoded secrets, keys, passwords\n\
         8. 🌐 SSRF, CSRF, Open Redirect\n\
         9. 📝 Sensitive information exposure (logs, error messages)\n\
         10. ⚡ Race Condition, Buffer Overflow, Use-after-free\n\n\
         Static analysis results:\n{}\n\n\
         Output format: JSON\n\
         {{\n\
           \"vulnerabilities\": [\n\
             {{\n\
               \"id\": \"V-1\",\n\
               \"title\": \"vulnerability name\",\n\
               \"severity\": \"Critical|High|Medium|Low|Info\",\n\
               \"owasp\": \"A03: Injection\",\n\
               \"file\": \"src/main.rs\",\n\
               \"line\": 42,\n\
               \"code_snippet\": \"vulnerable code\",\n\
               \"description\": \"detailed description\",\n\
               \"impact\": \"impact if exploited\",\n\
               \"attack_vector\": \"attack method\",\n\
               \"proof_of_concept\": \"PoC code or command\",\n\
               \"fix_suggestion\": \"specific fix instructions\",\n\
               \"fix_priority\": 1\n\
             }}\n\
           ],\n\
           \"scan_summary\": \"overall summary\",\n\
           \"overall_risk\": \"Critical|High|Medium|Low|Info\",\n\
           \"passed\": false,\n\
           \"executive_summary\": \"one-line executive summary\"\n\
         }}",
        story.id, story.title,
        crate::utils::trunc(scan_results, 4000)
    )
}

// ─── Security report parsing ───────────────────────────────────────────────────

fn parse_security_report(
    json_text: &str,
    story_id: &str,
    round: usize,
    report_id: &str,
) -> SecurityReport {
    let mut report = SecurityReport::new(report_id, story_id, round, "project source code");

    let v = match extract_json(json_text) {
        Some(v) => v,
        None => {
            // JSON parsing failed — infer pass/fail from text
            let passed = json_text.to_uppercase().contains("NO VULNERABILITY")
                || json_text.contains("\"passed\": true")
                || json_text.contains("no vulnerabilities");
            report.passed = passed;
            report.summary = crate::utils::trunc(json_text, 500).to_string();
            return report;
        }
    };

    // Parse vulnerabilities
    if let Some(vulns) = v["vulnerabilities"].as_array() {
        for (i, vj) in vulns.iter().enumerate() {
            let severity = parse_severity(vj["severity"].as_str().unwrap_or("Medium"));
            let owasp = parse_owasp(vj["owasp"].as_str().unwrap_or("Other"));
            let vid = vj["id"].as_str().unwrap_or(&format!("V-{}", i+1)).to_string();
            let title = vj["title"].as_str().unwrap_or("Unknown").to_string();
            let desc = vj["description"].as_str().unwrap_or("").to_string();
            let fix = vj["fix_suggestion"].as_str().unwrap_or("Code review required").to_string();

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

// ─── Hacker agent main ──────────────────────────────────────────────────────

pub struct HackerAgentOutput {
    pub report: SecurityReport,
    pub fix_instructions: String,
}

/// Run a single security scan round
pub async fn run_hacker_agent(
    client: &OllamaClient,
    story: &UserStory,
    project_path: &str,
    round: usize,
    hub: &NodeHub,
    on_progress: &impl Fn(&str),
) -> HackerAgentOutput {
    use crate::models::Message;

    on_progress(&format!("🔒 HackerAgent [{}] security scan starting (round {})...", story.id, round));

    // Step 1: run static scanners
    on_progress("  🔍 Running static analysis tools...");
    let scan_results = run_static_scanners(project_path).await;
    on_progress(&format!("  📊 {} scanner(s) completed", scan_results.len()));

    let scan_text = scan_results.iter()
        .map(|r| format!("=== {} ===\n{}", r.tool, r.output))
        .collect::<Vec<_>>().join("\n\n");

    // Step 2: AI security analysis
    on_progress("  🧠 Running AI security analysis...");
    let system = hacker_system_prompt(story, &scan_text);
    let user_msg = format!(
        "Analyze the security vulnerabilities of the following project.\n\
         🔍 First use web_search to find known vulnerability patterns for this type of project.\n\
         📂 Project path: {}\n\n\
         Implementation:\n{}\n\n\
         Static analysis results:\n{}",
        project_path,
        crate::utils::trunc(story.implementation.as_deref().unwrap_or("No implementation"), 3000),
        crate::utils::trunc(&scan_text, 2000),
    );

    let mut history = vec![
        Message::system(&format!("Model: {}\n\n{}\n\n{}", client.model(),
            crate::agent::tools::tool_descriptions(), system)),
        Message::user(&user_msg),
    ];

    // Notify node hub that HackerAgent is starting
    let _ = hub.send(NodeMessage {
        from: "HackerAgent".to_string(),
        to: String::new(),
        msg_type: MsgType::Status,
        content: format!("[{}] Security scan round {} starting", story.id, round),
        metadata: Default::default(),
    }).await;

    let mut ai_output = String::new();
    let max_turns = 12usize;

    for turn in 0..max_turns {
        let ai_text = match client.chat_stream(history.clone(), |_| {}).await {
            Ok(t) => t,
            Err(e) => { ai_output = format!("AI error: {}", e); break; }
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
                    results.push(format!("Tool '{}' result:\n{}", name, result.output));
                }
                history.push(Message::tool(results.join("\n\n")));
                if turn == max_turns - 1 { ai_output = results.join("\n\n"); }
            }
            crate::models::AgentResponse::ToolCall(tc) => {
                on_progress(&format!("  🔧 [HackerAgent] {}...", tc.name));
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("Tool '{}' result:\n{}", tc.name, result.output)));
                if turn == max_turns - 1 { ai_output = result.output; }
            }
        }
    }

    // Parse report
    let report_id = format!("SEC-{}-R{}", story.id, round);
    let mut report = parse_security_report(&ai_output, &story.id, round, &report_id);
    report.scan_tools_used = scan_results.iter().map(|r| r.tool.clone()).collect();

    let fix_instructions = report.fix_instructions();

    // Forward results to Developer node
    let _ = hub.send(NodeMessage {
        from: "HackerAgent".to_string(),
        to: "Developer".to_string(),
        msg_type: MsgType::Result,
        content: fix_instructions.clone(),
        metadata: Default::default(),
    }).await;

    // Also share with QA agent
    let _ = hub.send(NodeMessage {
        from: "HackerAgent".to_string(),
        to: "QAEngineer".to_string(),
        msg_type: MsgType::Status,
        content: format!("[{}] Security scan round {} complete — {} vulnerability(ies)",
            story.id, round, report.vulnerabilities.len()),
        metadata: Default::default(),
    }).await;

    on_progress(&format!(
        "  {} Security scan complete — {} ({} vulnerability(ies), {} Critical, {} High)",
        if report.passed { "✅" } else { "🚨" },
        report.overall_risk,
        report.vulnerabilities.len(),
        report.critical_count(),
        report.high_count(),
    ));

    HackerAgentOutput { report, fix_instructions }
}

// ─── Security fix loop ─────────────────────────────────────────────────────────

const MAX_SECURITY_ROUNDS: usize = 5;

pub struct SecurityFixResult {
    pub rounds: usize,
    pub final_report: SecurityReport,
    pub approved: bool,
}

/// HackerAgent + Developer security fix loop
/// Repeats until both QA and HackerAgent approve
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
        on_progress(&format!("\n🔒 ═══ Security round {}/{} ═══", round, MAX_SECURITY_ROUNDS));

        let story = match board.get_story(story_id) {
            Some(s) => s,
            None => break,
        };

        // Run HackerAgent
        let hack_output = run_hacker_agent(
            client, &story, project_path, round, hub, &on_progress
        ).await;

        // Save report to board
        board.update_story_field(story_id, "HackerAgent", |s| {
            let report_text = format!(
                "=== Security report round {} ===\n{}", round, hack_output.report.render()
            );
            // Append security report to qa_report field
            let prev = s.qa_report.take().unwrap_or_default();
            s.qa_report = Some(format!("{}\n\n{}", prev, report_text));
        }).ok();

        last_report = hack_output.report.clone();

        if hack_output.report.passed {
            on_progress("✅ HackerAgent: No vulnerabilities — security approved");
            break;
        }

        if round >= MAX_SECURITY_ROUNDS {
            on_progress(&format!("⚠️ Maximum security rounds ({}) reached — {} unresolved vulnerability(ies)",
                MAX_SECURITY_ROUNDS, hack_output.report.unfixed_count()));
            break;
        }

        // Send fix instructions to Developer
        on_progress(&format!("🔁 Sending security fix instructions to Developer ({} vulnerability(ies))...",
            hack_output.report.unfixed_count()));
        on_progress(&format!("{}", hack_output.report.render()));

        let story = match board.get_story(story_id) { Some(s) => s, None => break };

        // Run Developer security fix
        let dev_ctx = format!(
            "## Security fix instructions (round {})\n{}\n\n## Current implementation\n{}",
            round,
            hack_output.fix_instructions,
            crate::utils::trunc(story.implementation.as_deref().unwrap_or(""), 2000),
        );

        on_progress(&format!("💻 Developer: Fixing security vulnerabilities (round {})...", round));
        let dev_output = run_security_developer(
            client, &story, &dev_ctx, hub, &on_progress
        ).await;

        board.update_story_field(story_id, "Developer", |s| {
            s.implementation = Some(dev_output.clone());
        }).ok();

        on_progress(&format!("  ✍️  Developer fix complete (round {})", round));
    }

    SecurityFixResult {
        rounds: round,
        final_report: last_report.clone(),
        approved: last_report.passed,
    }
}

/// Run dedicated security-fix Developer
async fn run_security_developer(
    client: &OllamaClient,
    story: &UserStory,
    security_ctx: &str,
    hub: &NodeHub,
    on_progress: &impl Fn(&str),
) -> String {
    use crate::models::Message;

    let system = format!(
        "Model: {}\n\n{}\n\n\
         You are a developer specializing in security fixes.\n\
         Fix the code based on the provided security vulnerability report.\n\n\
         Fix principles:\n\
         - Fix all vulnerabilities using OWASP-recommended methods\n\
         - Use Parameterized Query / Prepared Statement (SQL Injection)\n\
         - Validate and encode input (XSS)\n\
         - Apply principle of least privilege\n\
         - Move secrets to environment variables or Secret Manager\n\
         - Add security headers (HSTS, CSP, X-Frame-Options)\n\
         - Remove sensitive information from error messages\n\n\
         After fixing, be sure to:\n\
         1. Verify build succeeds\n\
         2. Confirm existing tests pass\n\
         3. List the vulnerabilities that were fixed",
        client.model(),
        crate::agent::tools::tool_descriptions(),
    );

    let user_msg = format!(
        "Fix the following security vulnerabilities:\n\n{}\n\n\
         Before fixing, use web_search to find the latest remediation methods for each vulnerability.",
        security_ctx
    );

    let mut history = vec![
        Message::system(&system),
        Message::user(&user_msg),
    ];

    let _ = hub.send(NodeMessage {
        from: "Developer".to_string(), to: "HackerAgent".to_string(),
        msg_type: MsgType::Status,
        content: format!("[{}] Security fix work starting", story.id),
        metadata: Default::default(),
    }).await;

    let mut final_output = String::new();

    for turn in 0..20 {
        let ai_text = match client.chat_stream(history.clone(), |_| {}).await {
            Ok(t) => t,
            Err(e) => return format!("Error: {}", e),
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
                    results.push(format!("Tool '{}' result:\n{}", name, result.output));
                }
                history.push(Message::tool(results.join("\n\n")));
                if turn == 19 { final_output = results.join("\n\n"); }
            }
            crate::models::AgentResponse::ToolCall(tc) => {
                on_progress(&format!("  🔧 [SecDev] {}...", tc.name));
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                history.push(Message::assistant(&ai_text));
                history.push(Message::tool(format!("Tool '{}' result:\n{}", tc.name, result.output)));
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
