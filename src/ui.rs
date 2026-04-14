//! CLI color and rendering utilities
//!
//! Provides a colorful terminal UI in the style of Claude Code / GitHub Copilot.

use colored::*;

// ─── Color theme constants ────────────────────────────────────────────────────

pub const BANNER_VERSION: &str = env!("CARGO_PKG_VERSION");

// ─── Banner ───────────────────────────────────────────────────────────────────

pub fn print_banner(model: &str, session_label: &str) {
    let top    = "╔══════════════════════════════════════════════════╗";
    let mid    = "║                                                  ║";
    let bot    = "╚══════════════════════════════════════════════════╝";

    println!("{}", top.bright_cyan().bold());
    println!("{}", mid.bright_cyan().bold());

    let title = "  ✦  AI Agent  ─  Swing-by                       ";
    println!("{}{}{}",
        "║".bright_cyan().bold(),
        title.white().bold(),
        "║".bright_cyan().bold()
    );

    let model_line = format!("  ✦  Ollama + {:<34}", model);
    println!("{}{}{}",
        "║".bright_cyan().bold(),
        model_line.bright_purple(),
        "║".bright_cyan().bold()
    );

    println!("{}", mid.bright_cyan().bold());
    println!("{}", bot.bright_cyan().bold());

    if !session_label.is_empty() {
        println!("  {} {}", "Session:".bright_yellow().bold(), session_label.trim().white());
    }

    println!("  {} {}  {}  {}",
        "Slash commands:".dimmed(),
        "/help".bright_cyan(),
        "|".dimmed(),
        "quit: exit".dimmed()
    );
    println!();
}

// ─── Prompt prefix ────────────────────────────────────────────────────────────

pub fn prompt_prefix(plan_mode: bool, think_mode: bool) -> String {
    let label = match (plan_mode, think_mode) {
        (true, true)  => format!("[{}+{}] ", "PLAN".bright_magenta().bold(), "THINK".bright_yellow().bold()),
        (true, false) => format!("[{}] ", "PLAN".bright_magenta().bold()),
        (false, true) => format!("[{}] ", "THINK".bright_yellow().bold()),
        (false, false) => String::new(),
    };
    format!("{}{} ", label, "You".bright_green().bold())
}

pub fn agent_prefix() -> String {
    format!("{} ", "Agent".bright_magenta().bold())
}

// ─── Tool call display ───────────────────────────────────────────────────────

pub fn print_tool_start(name: &str, args_preview: &str) {
    println!("\n{}{}{}{}",
        "┌─[".bright_yellow(),
        "Tool".yellow().bold(),
        format!("] {} ", name).bright_yellow(),
        args_preview.dimmed()
    );
}

pub fn print_tool_result(success: bool, output: &str) {
    let (icon, color_fn): (&str, fn(&str) -> ColoredString) = if success {
        ("✓", |s: &str| s.bright_green())
    } else {
        ("✗", |s: &str| s.bright_red())
    };
    println!("{}{}{}",
        "└─[".dimmed(),
        color_fn(icon),
        format!("] {}", output).dimmed()
    );
}

pub fn print_multi_tool_header(count: usize) {
    println!("\n{}", format!("▶ Running {} tools in parallel", count).bright_yellow().bold());
}

// ─── Status messages ──────────────────────────────────────────────────────────

pub fn print_ok(msg: &str) {
    println!("{} {}", "✓".bright_green().bold(), msg);
}

pub fn print_warn(msg: &str) {
    println!("{} {}", "⚠".bright_yellow().bold(), msg.yellow());
}

pub fn print_err(msg: &str) {
    println!("{} {}", "✗".bright_red().bold(), msg.bright_red());
}

pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ".bright_cyan().bold(), msg.cyan());
}

pub fn print_connecting(msg: &str) {
    print!("{} {} ", "◎".bright_cyan(), msg.dimmed());
}

pub fn print_section(title: &str) {
    println!("\n{}", format!("── {} ", title).bright_cyan().bold());
}

// ─── Slash command help ───────────────────────────────────────────────────────

pub fn print_help_table() {
    let divider = "─────────────────────────────────────────────────────────────";

    println!();
    println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_cyan().bold());
    println!("{}{}{}",
        "║ ".bright_cyan().bold(),
        "                  Slash Command List                      ".white().bold(),
        " ║".bright_cyan().bold()
    );
    println!("{}", "╠═══════════════════════════════════════════════════════════╣".bright_cyan().bold());

    let commands: &[(&str, &str)] = &[
        // Basic
        ("/help",               "This help"),
        ("/clear",              "Clear history"),
        ("/resume",             "Load saved session"),
        ("/history [n]",        "View history (last n entries)"),
        ("/save",               "Save session"),
        ("/compact",            "Compress history with AI summary"),
        // Model
        ("/model <name>",       "Change model"),
        ("/models",             "List available models"),
        // Info
        ("/cost",               "Session token usage"),
        ("/context",            "Context window utilization"),
        ("/status",             "Session status summary"),
        ("/doctor",             "Environment diagnostics"),
        // File/context
        ("/init",               "Auto-generate CLAUDE.md"),
        ("/add <file>",         "Add file to context"),
        // Modes
        ("/think",              "Toggle extended reasoning mode"),
        ("/plan",               "Toggle plan mode"),
        // Export/git
        ("/export [filename]",  "Export conversation as markdown"),
        ("/commit [path]",      "Auto-generate AI commit message"),
        ("/review [path]",      "AI code review"),
        // Memory/config
        ("/memory save <note>", "Save a note"),
        ("/memory list",        "List saved notes"),
        ("/memory clear",       "Delete all notes"),
        ("/config [key] [val]", "View/change configuration"),
        // Agent
        ("/agile <task>",               "Agile sprint (PO→Dev→QA)"),
        ("/agile --fast <task>",         "Fast sprint (skip BA/UX)"),
        ("/board [project]",             "Agile board status"),
        ("/retro [sprint_id]",           "Sprint retrospective (KPT)"),
        ("/postmortem <desc>",           "Incident postmortem analysis"),
        ("/techdebt [path]",             "Technical debt analysis report"),
        ("/ba <task>",                   "Business analysis standalone"),
        ("/ux <task>",                   "UX design standalone"),
        ("/devops [path]",               "DevOps CI/CD configuration"),
        ("/docs [path]",                 "Auto-generate technical docs"),
        ("/sre [path]",                  "SRE monitoring + runbook"),
        ("/security [path]",             "Security audit (HackerAgent)"),
        ("/coordinator <task>",          "Parallel multi-agent coordinator"),
        ("/rag index [path]",            "Index codebase for RAG"),
        ("/rag query <question>",        "RAG-based code Q&A"),
        ("/rag status",                  "RAG index status"),
        ("/pr [create|list]",            "GitHub PR management"),
        ("/pipeline <task>",             "Multi-agent pipeline"),
        ("/nodes <task>",                "Node pipeline"),
        ("/ipc [port]",                  "Start AI-to-AI HTTP server"),
        ("/skills",                      "Detailed tool list"),
        ("/skill <name> [args]",         "Run a user skill"),
        ("/skill-new <name> <desc>",     "Create skill file"),
        ("/mcp",                         "MCP server/tool list"),
        ("/mcp-call <srv> <tool> <json>","MCP tool call"),
        ("/monitor",                     "Toggle system status display"),
        ("/sysinfo",                     "Print current system/GPU status"),
        ("exit / quit",                  "Exit"),
    ];

    // Category divider indices
    let dividers_after = [5, 7, 11, 14, 16, 21, 25];

    for (i, (cmd, desc)) in commands.iter().enumerate() {
        let cmd_str = format!("{:<30}", cmd);
        println!("{}  {}  {}",
            "║".bright_cyan().bold(),
            cmd_str.bright_yellow(),
            format!("{:<28}", desc).white()
        );
        if dividers_after.contains(&i) {
            println!("{}", format!("╟{}╢", divider).bright_cyan());
        }
    }

    println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_cyan().bold());
    println!();
}

// ─── AI response markdown rendering ──────────────────────────────────────────
//
// During streaming, raw text is printed.
// When reprinting a completed response, render_markdown is used.
// Currently only used for syntax highlighting code fences with color.

/// Render a completed AI response as markdown and print it
#[allow(dead_code)]
pub fn render_markdown(text: &str) {
    let mut in_code_block = false;
    let mut code_lang = String::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // Close code block
                println!("{}", "```".dimmed());
                in_code_block = false;
                code_lang.clear();
            } else {
                // Open code block
                code_lang = line[3..].trim().to_string();
                let lang_display = if code_lang.is_empty() {
                    String::new()
                } else {
                    format!(" {}", code_lang.bright_cyan())
                };
                println!("{}{}", "```".dimmed(), lang_display);
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            println!("{}", highlight_code_line(line, &code_lang));
            continue;
        }

        // Headers
        if line.starts_with("### ") {
            println!("{}", line[4..].bright_cyan().bold());
        } else if line.starts_with("## ") {
            println!("{}", line[3..].bright_cyan().bold().underline());
        } else if line.starts_with("# ") {
            println!("{}", line[2..].bright_white().bold().underline());
        }
        // Dividers
        else if line == "---" || line == "===" {
            println!("{}", "─────────────────────────────────".dimmed());
        }
        // Bullet points
        else if line.starts_with("- ") || line.starts_with("* ") {
            println!("  {} {}", "•".bright_cyan(), inline_format(&line[2..]));
        } else if let Some(rest) = line.strip_prefix("  - ").or_else(|| line.strip_prefix("  * ")) {
            println!("    {} {}", "◦".cyan(), inline_format(rest));
        }
        // Numbered lists
        else if line.len() > 2 && line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && line.contains(". ") {
            if let Some(pos) = line.find(". ") {
                let num = &line[..pos];
                let rest = &line[pos + 2..];
                println!("  {} {}", format!("{}.", num).bright_yellow(), inline_format(rest));
            } else {
                println!("{}", inline_format(line));
            }
        }
        // Plain text
        else {
            println!("{}", inline_format(line));
        }
    }
}

/// Inline markdown formatting (`code`, **bold**, _italic_)
fn inline_format(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    let mut buf = String::new();

    while let Some(c) = chars.next() {
        match c {
            '`' => {
                result.push_str(&buf);
                buf.clear();
                let mut code = String::new();
                for ch in chars.by_ref() {
                    if ch == '`' { break; }
                    code.push(ch);
                }
                result.push_str(&code.bright_cyan().to_string());
            }
            '*' if chars.peek() == Some(&'*') => {
                chars.next(); // consume second '*'
                result.push_str(&buf);
                buf.clear();
                let mut bold = String::new();
                loop {
                    match chars.next() {
                        Some('*') if chars.peek() == Some(&'*') => { chars.next(); break; }
                        Some(ch) => bold.push(ch),
                        None => break,
                    }
                }
                result.push_str(&bold.bold().to_string());
            }
            _ => buf.push(c),
        }
    }
    result.push_str(&buf);
    result
}

/// Code line syntax highlighting (simple keyword-based highlighting per language)
fn highlight_code_line(line: &str, lang: &str) -> String {
    let lang = lang.to_lowercase();

    // Comment handling
    let comment_prefix = match lang.as_str() {
        "rust" | "rs" | "c" | "cpp" | "c++" | "java" | "js" | "ts" | "javascript" | "typescript"
            | "go" | "swift" | "kotlin" => Some("//"),
        "python" | "py" | "bash" | "sh" | "toml" | "yaml" | "yml" => Some("#"),
        _ => None,
    };

    let trimmed = line.trim_start();
    if let Some(prefix) = comment_prefix {
        if trimmed.starts_with(prefix) {
            return format!("{}", line.dimmed().italic());
        }
    }
    // Python/bash # comments
    if (lang == "python" || lang == "py" || lang == "bash" || lang == "sh")
        && trimmed.starts_with('#')
    {
        return format!("{}", line.dimmed().italic());
    }

    // Empty lines
    if line.trim().is_empty() {
        return line.to_string();
    }

    // Keyword-based highlighting (full line)
    let keywords_rust = [
        "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait",
        "match", "if", "else", "for", "while", "loop", "return", "async", "await",
        "self", "super", "crate", "where", "type", "const", "static", "ref", "move",
        "Box", "Vec", "Option", "Result", "Some", "None", "Ok", "Err", "true", "false",
    ];
    let keywords_python = [
        "def", "class", "if", "elif", "else", "for", "while", "return", "import",
        "from", "as", "with", "try", "except", "finally", "raise", "pass", "break",
        "continue", "lambda", "yield", "async", "await", "True", "False", "None",
        "and", "or", "not", "in", "is",
    ];
    let keywords_js = [
        "const", "let", "var", "function", "return", "if", "else", "for", "while",
        "class", "import", "export", "from", "async", "await", "new", "this",
        "true", "false", "null", "undefined", "typeof", "instanceof",
    ];

    let keywords: &[&str] = match lang.as_str() {
        "rust" | "rs" => &keywords_rust,
        "python" | "py" => &keywords_python,
        "js" | "ts" | "javascript" | "typescript" => &keywords_js,
        _ => &[],
    };

    // String highlighting (quotes in the line) — keyword first token highlighting
    let first_token = line.split_whitespace().next().unwrap_or("");
    let first_clean = first_token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');

    if keywords.contains(&first_clean) {
        // First keyword in blue, rest in default
        let rest = &line[line.find(first_clean).unwrap_or(0) + first_clean.len()..];
        return format!("{}{}", first_clean.bright_blue().bold(), rest.white());
    }

    // Numbers (lines starting with a digit after indentation)
    if trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return format!("{}", line.bright_yellow());
    }

    // Default white
    line.white().to_string()
}

// ─── Quick color output used in /help ────────────────────────────────────────

pub fn color_cmd(s: &str) -> ColoredString {
    s.bright_yellow().bold()
}

pub fn color_desc(s: &str) -> ColoredString {
    s.white()
}

// ─── Ollama connection status ─────────────────────────────────────────────────

pub fn print_connection_ok(model: &str) {
    println!("{}", "Connected".bright_green().bold());
    let _ = model; // display model if needed
}

pub fn print_connection_fail(url: &str) {
    println!("{}", "Connection failed".bright_red().bold());
    println!();
    println!("{}", "Cannot connect to Ollama server.".bright_red().bold());
    println!("{}", "How to fix:".yellow());
    println!("  {}  ollama serve", "1.".dimmed());
    println!("  {}  Change address via OLLAMA_API_URL env var (current: {})", "2.".dimmed(), url.cyan());
    println!();
}

// ─── Sprint result banner ─────────────────────────────────────────────────────

pub fn print_sprint_result(completed: usize, failed: usize, bugs: usize, velocity: u32) {
    println!();
    println!("{}", "╔══════════════════════════════════════════╗".bright_cyan().bold());
    println!("{}{}{}",
        "║ ".bright_cyan().bold(),
        "  Sprint Completed                      ".white().bold(),
        " ║".bright_cyan().bold()
    );
    println!("{}", "╠══════════════════════════════════════════╣".bright_cyan().bold());
    println!("{}  Completed stories : {}{}",
        "║".bright_cyan().bold(), completed.to_string().bright_green().bold(), "                         ║".bright_cyan().bold());
    println!("{}  Failed stories    : {}{}",
        "║".bright_cyan().bold(),
        if failed > 0 { failed.to_string().bright_red().bold() } else { failed.to_string().dimmed() },
        "                         ║".bright_cyan().bold()
    );
    println!("{}  Total bugs        : {}{}",
        "║".bright_cyan().bold(),
        if bugs > 0 { bugs.to_string().bright_yellow().bold() } else { bugs.to_string().dimmed() },
        "                         ║".bright_cyan().bold()
    );
    println!("{}  Velocity          : {} pts{}",
        "║".bright_cyan().bold(), velocity.to_string().bright_white().bold(),
        "                      ║".bright_cyan().bold()
    );
    println!("{}", "╚══════════════════════════════════════════╝".bright_cyan().bold());
    println!();
}

// ─── Pipeline section ─────────────────────────────────────────────────────────

pub fn print_pipeline_section(icon: &str, title: &str, content: &str) {
    println!("\n{} {}", icon, title.bright_cyan().bold());
    for line in content.lines() {
        println!("  {}", line.white());
    }
}
