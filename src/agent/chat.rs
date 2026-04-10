use anyhow::Result;
use tracing::{debug, info};

use crate::agent::{ollama::OllamaClient, tools::{dispatch_tool, tool_descriptions}};
use crate::models::{AgentResponse, Message, ToolCall};

/// AI 응답 텍스트에서 툴 호출 또는 종료 파싱
fn parse_response(text: &str) -> AgentResponse {
    let trimmed = text.trim();

    if trimmed == "EXIT" {
        return AgentResponse::Exit;
    }

    // "TOOL: <name> <args...>" 형식 파싱
    if let Some(rest) = trimmed.strip_prefix("TOOL:") {
        let rest = rest.trim();
        let parts = shlex::split(rest).unwrap_or_else(|| {
            rest.split_whitespace().map(|s| s.to_string()).collect()
        });

        if let Some((name, args)) = parts.split_first() {
            return AgentResponse::ToolCall(ToolCall {
                name: name.clone(),
                args: args.to_vec(),
            });
        }
    }

    AgentResponse::Text(text.to_string())
}

/// 대화형 채팅 루프
pub async fn run_chat_loop(client: &OllamaClient) -> Result<()> {
    use std::io::{self, BufRead, Write};

    let system_prompt = format!(
        "당신은 Rust로 구현된 AI 에이전트입니다. 모델: {}\n\n{}",
        client.model(),
        tool_descriptions()
    );

    let mut history: Vec<Message> = vec![Message::system(&system_prompt)];

    println!("╔══════════════════════════════════════════╗");
    println!("║       AI Agent (Ollama + {:<12})  ║", client.model());
    println!("╚══════════════════════════════════════════╝");
    println!("'exit' 또는 'quit' 입력으로 종료\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("You> ");
        stdout.flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }
        if matches!(input.to_lowercase().as_str(), "exit" | "quit" | "종료") {
            println!("에이전트를 종료합니다.");
            break;
        }

        history.push(Message::user(&input));

        // 최대 10턴 툴 호출 루프
        for turn in 0..10 {
            debug!("AI 응답 요청 (turn={})", turn);

            let resp = client.chat(history.clone()).await?;
            let ai_text = resp.message.content.clone();

            info!("AI 응답: {}", ai_text);

            match parse_response(&ai_text) {
                AgentResponse::Exit => {
                    println!("에이전트가 종료를 요청했습니다.");
                    return Ok(());
                }
                AgentResponse::Text(text) => {
                    println!("\nAgent> {}\n", text);
                    history.push(Message::assistant(&ai_text));
                    break;
                }
                AgentResponse::ToolCall(tool_call) => {
                    println!("\n[툴 호출] {} {:?}", tool_call.name, tool_call.args);
                    let result = dispatch_tool(&tool_call).await;

                    let status = if result.success { "✓" } else { "✗" };
                    println!("[결과 {}]\n{}\n", status, result.output);

                    // 툴 결과를 히스토리에 추가
                    history.push(Message::assistant(&ai_text));
                    history.push(Message::tool(format!(
                        "툴 '{}'의 실행 결과:\n{}",
                        tool_call.name, result.output
                    )));
                }
            }
        }
    }

    Ok(())
}
