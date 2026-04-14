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
pub mod utils;

use anyhow::Result;
use tracing::{error, info, warn};

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

    // ── 플래그 Parsing ───────────────────────────────────────────────────────────
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

    // IPC stdio 모드는 연결 확인 없이 바로 시작
    if ipc_stdio {
        let server = ipc::AgentServer::new(client);
        return server.run_stdio().await;
    }

    // IPC HTTP 서버 모드
    if let Some(port) = ipc_port {
        let server = ipc::AgentServer::new(client);
        return server.run_http_server(port).await;
    }

    // ── Ollama 연결 확인 ─────────────────────────────────────────────────────
    let quiet = print_prompt.is_some();
    if !quiet {
        print!("Ollama 서버 연결 확인 중... ");
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    match client.health_check().await {
        Ok(true) => {
            if !quiet { println!("연결됨"); }
            match client.list_models().await {
                Ok(models) if !models.is_empty() => {
                    info!("사용 가능한 모델: {}", models.join(", "));
                    if !quiet && !models.iter().any(|m| m == client.model()) {
                        warn!("모델 '{}' 없음 — `ollama pull {}` 필요", client.model(), client.model());
                    }
                }
                Ok(_) => { if !quiet { warn!("설치된 모델 없음. `ollama pull {}` 실행 필요", client.model()); } }
                Err(e) => { if !quiet { warn!("모델 목록 조회 실패: {}", e); } }
            }
        }
        _ => {
            error!(
                "Ollama 서버에 연결할 수 없습니다.\n\
                 해결 방법:\n\
                 1. Ollama 직접 실행: ollama serve\n\
                 2. OLLAMA_API_URL 환경변수로 서버 주소 변경 (기본: http://localhost:11434)"
            );
            std::process::exit(1);
        }
    }

    // ── 실행 모드 분기 ────────────────────────────────────────────────────────
    if discord_mode {
        println!("Discord 봇 모드 시작...");
        let client_arc = std::sync::Arc::new(client);
        if let Err(e) = discord::run_discord_bot(client_arc).await {
            error!("Discord 봇 오류: {}", e);
            return Err(e);
        }
    } else if let Some(task) = agile_task {
        // 애자일 Sprint 모드
        let project = agile_project.as_deref().unwrap_or("project");
        println!("\n🏃 애자일 스프린트 시작: {}", project);
        match agile::run_agile_sprint(&client, project, &task, |msg| println!("{}", msg)).await {
            Ok(result) => {
                println!("\n╔══════════════════════════════════════════╗");
                println!("║  스프린트 완료                           ║");
                println!("╚══════════════════════════════════════════╝");
                println!("  완료 스토리 : {} 개", result.completed_stories.len());
                println!("  실패 스토리 : {} 개", result.failed_stories.len());
                println!("  총 버그     : {} 개", result.total_bugs);
                println!("  벨로시티    : {} pts", result.velocity);
            }
            Err(e) => { error!("애자일 스프린트 오류: {}", e); return Err(e); }
        }
    } else if let Some(task) = pipeline_task {
        match agent::orchestrator::run_pipeline(&client, &task).await {
            Ok(result) => {
                println!("\n=== 파이프라인 결과 ===");
                println!("계획:\n{}", result.plan);
                println!("\n구현:\n{}", result.implementation);
                println!("\n검증:\n{}", result.verification);
                if let Some(review) = result.review { println!("\n리뷰:\n{}", review); }
            }
            Err(e) => { error!("파이프라인 오류: {}", e); return Err(e); }
        }
    } else if let Some(prompt) = print_prompt {
        if let Err(e) = agent::run_print_mode(&client, &prompt).await {
            error!("실행 오류: {}", e);
            return Err(e);
        }
    } else {
        if let Err(e) = agent::run_chat_loop_opts(&client, resume).await {
            error!("채팅 루프 오류: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

fn print_help() {
    println!("AI Agent — Ollama 기반 풀스택 개발 에이전트\n");
    println!("사용법:");
    println!("  ai_agent [옵션]");
    println!("  ai_agent -p \"<프롬프트>\"           비대화형 단일 실행");
    println!("  ai_agent --agile \"<작업>\"           애자일 스프린트 실행\n");
    println!("옵션:");
    println!("  -p, --print <프롬프트>    비대화형 모드로 단일 프롬프트 실행 후 종료");
    println!("  -m, --model <이름>        사용할 모델 (기본: OLLAMA_MODEL 또는 gemma4:e4b)");
    println!("  -r, --resume              이전 세션 히스토리를 불러와서 계속");
    println!("  -c, --continue            --resume 과 동일");
    println!("  -s, --session <이름>      세션 파일 이름 지정");
    println!("  -V, --version             버전 출력");
    println!("  -d, --discord             Discord 봇 모드 (DISCORD_TOKEN 필요)");
    println!("  --pipeline <작업>         멀티에이전트 파이프라인 (기획→개발→디버깅)");
    println!("  --agile <작업>            애자일 스프린트 (PO→SM→Arch→Dev→QA→Review)");
    println!("  --project <이름>          애자일 프로젝트 이름 (--agile 와 함께 사용)");
    println!("  --ipc-stdio               AI-to-AI stdio 통신 모드 (JSON-RPC 2.0)");
    println!("  --ipc-server <포트>       AI-to-AI HTTP 서버 모드");
    println!("  -h, --help                이 도움말 출력\n");
    println!("환경변수:");
    println!("  OLLAMA_API_URL     Ollama 서버 주소 (기본: http://localhost:11434)");
    println!("  OLLAMA_MODEL       사용할 모델 (기본: gemma4:e4b)\n");
    println!("AI-to-AI 통신 예시:");
    println!("  # Claude Code에서 이 에이전트 호출:");
    println!("  echo '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chat\",\"params\":{{\"prompt\":\"hello\"}}}}' | ai_agent --ipc-stdio");
    println!("  # HTTP 서버 실행 후 다른 AI에서 호출:");
    println!("  ai_agent --ipc-server 8765");
    println!("  curl -X POST http://localhost:8765 -H 'Content-Type: application/json' \\");
    println!("       -H 'X-Caller-ID: claude-code' \\");
    println!("       -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"agile_sprint\",\"params\":{{\"project\":\"myapp\",\"request\":\"로그인 기능 구현\"}}}}'");
    println!("\n대화형 슬래시 명령어: /help 참고 (실행 후)");
}
