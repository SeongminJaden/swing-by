//! AI-to-AI communication server
//!
//! Two modes:
//!   stdio mode: reads JSON-RPC requests from stdin and responds to stdout
//!               other AIs run this process as a subprocess to communicate
//!               (same as MCP protocol)
//!
//!   HTTP mode: receives JSON-RPC requests on TCP socket
//!               external AIs call via HTTP POST
//!
//! Usage examples (from Claude Code):
//!   `ai_agent --ipc-stdio`   → Claude Code runs as subprocess
//!   `ai_agent --ipc-server 8765` → run as HTTP server
//!
//! Calling from another AI:
//!   echo '{"jsonrpc":"2.0","id":1,"method":"chat","params":{"prompt":"hello"}}' | ai_agent --ipc-stdio
//!   curl -X POST http://localhost:8765 -H 'Content-Type: application/json' \
//!        -d '{"jsonrpc":"2.0","id":1,"method":"capabilities","params":{}}'

use anyhow::Result;
use serde_json::json;

use super::protocol::{
    JsonRpcRequest, JsonRpcResponse, declare_capabilities,
};
use crate::agent::ollama::OllamaClient;

pub struct AgentServer {
    client: OllamaClient,
}

impl AgentServer {
    pub fn new(client: OllamaClient) -> Self {
        Self { client }
    }

    // ─── stdio mode ──────────────────────────────────────────────────────────

    /// stdin/stdout JSON-RPC loop (MCP compatible)
    pub async fn run_stdio(&self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        eprintln!("[IPC] stdio mode started (JSON-RPC 2.0)");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 { break; }

            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }

            let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(req) => self.handle_request(req).await,
                Err(e) => JsonRpcResponse::err(
                    serde_json::Value::Null,
                    -32700,
                    &format!("Parse error: {}", e),
                ),
            };

            let resp_str = serde_json::to_string(&response)?;
            stdout.write_all(format!("{}\n", resp_str).as_bytes()).await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    // ─── HTTP server mode ──────────────────────────────────────────────────────

    /// TCP HTTP server (tokio-based minimal HTTP)
    pub async fn run_http_server(&self, port: u16) -> Result<()> {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        eprintln!("[IPC] HTTP server started — http://0.0.0.0:{}", port);
        eprintln!("[IPC] Other AIs can send JSON-RPC requests to POST / or POST /rpc");

        loop {
            let (mut stream, addr) = listener.accept().await?;
            eprintln!("[IPC] Connection: {}", addr);

            // Simple HTTP parsing
            let mut buf = vec![0u8; 16384];
            let n = stream.read(&mut buf).await?;
            let raw = String::from_utf8_lossy(&buf[..n]);

            // Extract body from HTTP request
            let body = if let Some(sep) = raw.find("\r\n\r\n") {
                raw[sep + 4..].trim().to_string()
            } else {
                String::new()
            };

            // CORS + JSON-RPC handling
            let (status, resp_body) = if body.is_empty() {
                ("200 OK", json!({"status": "AI Agent IPC Server running", "version": "0.1.0"}).to_string())
            } else {
                match serde_json::from_str::<JsonRpcRequest>(&body) {
                    Ok(req) => {
                        let caller_info = extract_caller_header(&raw);
                        eprintln!("[IPC] Request: {} (caller: {})", req.method, caller_info);
                        let resp = self.handle_request(req).await;
                        ("200 OK", serde_json::to_string(&resp).unwrap_or_default())
                    }
                    Err(e) => {
                        let resp = JsonRpcResponse::err(
                            serde_json::Value::Null, -32700,
                            &format!("Parse error: {}", e)
                        );
                        ("400 Bad Request", serde_json::to_string(&resp).unwrap_or_default())
                    }
                }
            };

            let http_resp = format!(
                "HTTP/1.1 {}\r\n\
                 Content-Type: application/json\r\n\
                 Access-Control-Allow-Origin: *\r\n\
                 Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
                 Access-Control-Allow-Headers: Content-Type, X-Caller-ID, X-Caller-Type\r\n\
                 Content-Length: {}\r\n\
                 \r\n{}",
                status, resp_body.len(), resp_body
            );

            let _ = stream.write_all(http_resp.as_bytes()).await;
        }
    }

    // ─── Request handling ───────────────────────────────────────────────────────────

    async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        let params = req.params.unwrap_or(json!({}));

        match req.method.as_str() {
            // Initialization handshake (MCP compatible)
            "initialize" => {
                JsonRpcResponse::ok(id, json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "ai_agent",
                        "version": env!("CARGO_PKG_VERSION"),
                        "description": "Rust Ollama-based AI agent with Agile pipeline"
                    },
                    "capabilities": {
                        "tools": true,
                        "agile": true,
                        "chat": true,
                        "multi_agent": true
                    }
                }))
            }

            // Capability list
            "capabilities" | "tools/list" => {
                let caps = declare_capabilities();
                JsonRpcResponse::ok(id, json!({ "capabilities": caps }))
            }

            // Connection check
            "ping" => {
                JsonRpcResponse::ok(id, json!({ "pong": true, "model": self.client.model() }))
            }

            // General conversation
            "chat" => {
                let prompt = params["prompt"].as_str().unwrap_or("");
                let caller = params["caller_id"].as_str().unwrap_or("unknown");
                if prompt.is_empty() {
                    return JsonRpcResponse::err(id, -32602, "prompt parameter required");
                }

                eprintln!("[IPC] chat request (from: {}) — {}", caller,
                    crate::utils::trunc(prompt, 60));

                let result = self.run_chat(prompt, caller).await;
                match result {
                    Ok(content) => JsonRpcResponse::ok(id, json!({ "content": content })),
                    Err(e) => JsonRpcResponse::err(id, -32603, &e.to_string()),
                }
            }

            // Direct tool execution
            "run_tool" | "tools/call" => {
                let tool_name = params["name"].as_str()
                    .or_else(|| params["tool"].as_str())
                    .unwrap_or("");
                if tool_name.is_empty() {
                    return JsonRpcResponse::err(id, -32602, "name parameter required");
                }

                let args: Vec<String> = params["args"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();

                eprintln!("[IPC] Tool execution: {} {:?}", tool_name, args);

                let tc = crate::models::ToolCall { name: tool_name.to_string(), args };
                let result = crate::agent::tools::dispatch_tool(&tc).await;
                JsonRpcResponse::ok(id, json!({
                    "success": result.success,
                    "output": result.output
                }))
            }

            // Agile sprint
            "agile_sprint" => {
                let project = params["project"].as_str().unwrap_or("ipc_project");
                let request = params["request"].as_str().unwrap_or("");
                if request.is_empty() {
                    return JsonRpcResponse::err(id, -32602, "request parameter required");
                }

                eprintln!("[IPC] Agile sprint: {} — {}", project,
                    crate::utils::trunc(request, 60));

                match crate::agile::run_agile_sprint(
                    &self.client, project, request,
                    |msg| eprintln!("[IPC/Sprint] {}", msg)
                ).await {
                    Ok(result) => JsonRpcResponse::ok(id, json!({
                        "sprint_id": result.sprint_id,
                        "completed": result.completed_stories,
                        "failed": result.failed_stories,
                        "velocity": result.velocity,
                        "total_bugs": result.total_bugs,
                    })),
                    Err(e) => JsonRpcResponse::err(id, -32603, &e.to_string()),
                }
            }

            // Board status
            "board_status" => {
                let project = params["project"].as_str().unwrap_or("default");
                let board = crate::agile::AgileBoard::load_or_new(project);
                let rendered = board.render();
                JsonRpcResponse::ok(id, json!({ "board": rendered }))
            }

            // Unknown method
            unknown => {
                JsonRpcResponse::err(id, -32601,
                    &format!("Method not found: {}", unknown))
            }
        }
    }

    // ─── Conversation execution ───────────────────────────────────────────────────────────

    async fn run_chat(&self, prompt: &str, caller: &str) -> Result<String> {
        use crate::models::Message;
        use crate::agent::tools::tool_descriptions;
        use crate::agent::chat::load_claude_md;

        let system = format!(
            "Model: {}\n\n{}{}\n\n[IPC mode] caller: {}",
            self.client.model(), tool_descriptions(), load_claude_md(), caller
        );

        let mut history = vec![
            Message::system(&system),
            Message::user(prompt),
        ];

        for _ in 0..10 {
            let text = self.client.chat_stream(history.clone(), |_| {}).await?;
            match crate::agent::chat::parse_response_pub(&text) {
                crate::models::AgentResponse::Exit | crate::models::AgentResponse::Text(_) => {
                    return Ok(text);
                }
                crate::models::AgentResponse::ToolCall(tc) => {
                    let result = crate::agent::tools::dispatch_tool(&tc).await;
                    history.push(Message::assistant(&text));
                    history.push(Message::tool(format!("Tool '{}' result:\n{}", tc.name, result.output)));
                }
            }
        }

        Ok("Maximum turns exceeded".to_string())
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────────

fn extract_caller_header(raw_http: &str) -> String {
    for line in raw_http.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("x-caller-id:") {
            return line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        }
        if lower.starts_with("x-caller-type:") {
            return line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        }
    }
    "unknown".to_string()
}
