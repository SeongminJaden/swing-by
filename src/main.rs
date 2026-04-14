pub mod agile;
mod agent;
pub mod config;
mod discord;
pub mod history;
pub mod ipc;
pub mod mcp;
pub mod monitor;
mod models;
pub mod skills;
mod tools;
pub mod ui;
pub mod utils;

use anyhow::Result;
use colored::*;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ai_agent {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // ── Flag Parsing ────────────────────────────────────────────────────────────
    let resume       = args.iter().any(|a| a == "--resume" || a == "-r" || a == "--continue" || a == "-c");
    let discord_mode = args.iter().any(|a| a == "--discord" || a == "-d");
    let ipc_stdio    = args.iter().any(|a| a == "--ipc-stdio");

    let model_override = args.windows(2).find(|w| w[0] == "--model" || w[0] == "-m").map(|w| w[1].clone());
    let print_prompt   = args.windows(2).find(|w| w[0] == "--print" || w[0] == "-p").map(|w| w[1].clone());
    let session_name   = args.windows(2).find(|w| w[0] == "--session" || w[0] == "-s").map(|w| w[1].clone());
    let pipeline_task  = args.windows(2).find(|w| w[0] == "--pipeline").map(|w| w[1].clone());
    let agile_task     = args.windows(2).find(|w| w[0] == "--agile").map(|w| w[1].clone());
    let agile_project  = args.windows(2).find(|w| w[0] == "--project").map(|w| w[1].clone());
    let ipc_port       = args.windows(2).find(|w| w[0] == "--ipc-server").and_then(|w| w[1].parse::<u16>().ok());

    if let Some(ref m) = model_override { std::env::set_var("OLLAMA_MODEL", m); }
    if let Some(ref s) = session_name   { std::env::set_var("AI_SESSION_NAME", s); }

    let client = agent::OllamaClient::from_env();
    if std::env::var("OLLAMA_MODEL").is_err() { std::env::set_var("OLLAMA_MODEL", client.model()); }
    if std::env::var("OLLAMA_API_URL").is_err() { std::env::set_var("OLLAMA_API_URL", "http://localhost:11434"); }

    // IPC stdio mode starts immediately without connection check
    if ipc_stdio {
        let server = ipc::AgentServer::new(client);
        return server.run_stdio().await;
    }

    // IPC HTTP server mode
    if let Some(port) = ipc_port {
        let server = ipc::AgentServer::new(client);
        return server.run_http_server(port).await;
    }

    // ── Ollama connection check ──────────────────────────────────────────────
    let quiet = print_prompt.is_some();
    if !quiet {
        use std::io::Write;
        ui::print_connecting("Checking Ollama server connection...");
        std::io::stdout().flush().ok();
    }

    match client.health_check().await {
        Ok(true) => {
            if !quiet {
                ui::print_connection_ok(client.model());
                match client.list_models().await {
                    Ok(models) if !models.is_empty() => {
                        info!("Available models: {}", models.join(", "));
                        if !models.iter().any(|m| m == client.model()) {
                            ui::print_warn(&format!(
                                "Model '{}' not found — run {}",
                                client.model(),
                                format!("ollama pull {}", client.model()).bright_cyan()
                            ));
                        }
                    }
                    Ok(_) => {
                        ui::print_warn(&format!(
                            "No models installed. Run {}",
                            format!("ollama pull {}", client.model()).bright_cyan()
                        ));
                    }
                    Err(e) => { ui::print_warn(&format!("Failed to list models: {}", e)); }
                }
            } else {
                match client.list_models().await {
                    Ok(models) if !models.is_empty() => {
                        info!("Available models: {}", models.join(", "));
                    }
                    _ => {}
                }
            }
        }
        _ => {
            let url = std::env::var("OLLAMA_API_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
            ui::print_connection_fail(&url);
            error!("Failed to connect to Ollama server");
            std::process::exit(1);
        }
    }

    // ── Execution mode branching ──────────────────────────────────────────────
    if discord_mode {
        ui::print_info("Starting Discord bot mode...");
        let client_arc = std::sync::Arc::new(client);
        if let Err(e) = discord::run_discord_bot(client_arc).await {
            error!("Discord bot error: {}", e);
            return Err(e);
        }
    } else if let Some(task) = agile_task {
        // Agile Sprint mode
        let project = agile_project.as_deref().unwrap_or("project");
        println!("\n{} Starting agile sprint: {}", "🏃".bright_yellow(), project.bright_cyan().bold());
        match agile::run_agile_sprint(&client, project, &task, |msg| println!("{}", msg)).await {
            Ok(result) => {
                ui::print_sprint_result(
                    result.completed_stories.len(),
                    result.failed_stories.len(),
                    result.total_bugs,
                    result.velocity,
                );
            }
            Err(e) => { error!("Agile sprint error: {}", e); return Err(e); }
        }
    } else if let Some(task) = pipeline_task {
        match agent::orchestrator::run_pipeline(&client, &task).await {
            Ok(result) => {
                ui::print_pipeline_section("📋", "Plan", &result.plan);
                ui::print_pipeline_section("💻", "Implementation", &result.implementation);
                ui::print_pipeline_section("🔍", "Verification", &result.verification);
                if let Some(review) = result.review {
                    ui::print_pipeline_section("👁", "Review", &review);
                }
            }
            Err(e) => { error!("Pipeline error: {}", e); return Err(e); }
        }
    } else if let Some(prompt) = print_prompt {
        if let Err(e) = agent::run_print_mode(&client, &prompt).await {
            error!("Execution error: {}", e);
            return Err(e);
        }
    } else {
        if let Err(e) = agent::run_chat_loop_opts(&client, resume).await {
            error!("Chat loop error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

fn print_help() {
    println!();
    println!("{}", "AI Agent — Ollama-based full-stack development agent".bright_white().bold());
    println!("{}", format!("Version {}", env!("CARGO_PKG_VERSION")).dimmed());
    println!();

    println!("{}", "Usage:".bright_cyan().bold());
    println!("  {} {}",         "ai_agent".bright_yellow(), "[options]".white());
    println!("  {} {} {}",      "ai_agent".bright_yellow(), "-p".bright_green(), "\"<prompt>\"           non-interactive single execution".white());
    println!("  {} {} {}",      "ai_agent".bright_yellow(), "--agile".bright_green(), "\"<task>\"             run agile sprint".white());
    println!();

    println!("{}", "Options:".bright_cyan().bold());
    let opts: &[(&str, &str)] = &[
        ("-p, --print <prompt>", "Run a single prompt in non-interactive mode and exit"),
        ("-m, --model <name>",   "Model to use (default: OLLAMA_MODEL or gemma4:e4b)"),
        ("-r, --resume",         "Load previous session history and continue"),
        ("-c, --continue",       "Same as --resume"),
        ("-s, --session <name>", "Specify session file name"),
        ("-V, --version",        "Print version"),
        ("-d, --discord",        "Discord bot mode (requires DISCORD_TOKEN)"),
        ("--pipeline <task>",    "Multi-agent pipeline (plan→dev→debug)"),
        ("--agile <task>",       "Agile sprint (PO→SM→Arch→Dev→QA→Review)"),
        ("--project <name>",     "Agile project name (used with --agile)"),
        ("--ipc-stdio",          "AI-to-AI stdio communication mode (JSON-RPC 2.0)"),
        ("--ipc-server <port>",  "AI-to-AI HTTP server mode"),
        ("-h, --help",           "Print this help"),
    ];
    for (flag, desc) in opts {
        println!("  {}  {}", format!("{:<28}", flag).bright_green(), desc.white());
    }
    println!();

    println!("{}", "Environment variables:".bright_cyan().bold());
    println!("  {}  {}", "OLLAMA_API_URL".bright_yellow(), "Ollama server address (default: http://localhost:11434)".white());
    println!("  {}  {}", "OLLAMA_MODEL  ".bright_yellow(), "Model to use (default: gemma4:e4b)".white());
    println!();

    println!("{}", "AI-to-AI communication examples:".bright_cyan().bold());
    println!("  {}", "# Calling this agent from Claude Code:".dimmed().italic());
    println!("  {}", "echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chat\",\"params\":{\"prompt\":\"hello\"}}' | ai_agent --ipc-stdio".bright_black());
    println!("  {}", "# Start HTTP server and call from another AI:".dimmed().italic());
    println!("  {}", "ai_agent --ipc-server 8765".bright_black());
    println!();

    println!("{}", "Interactive slash commands: see /help (after launch)".dimmed());
    println!();
}
