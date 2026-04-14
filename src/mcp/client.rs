//! MCP (Model Context Protocol) 클라이언트
//!
//! MCP 서버에 연결하여 툴을 호출합니다.
//! 지원 트랜스포트:
//!   - stdio: 로컬 프로세스 (가장 일반적)
//!   - http: HTTP/SSE 서버
//!
//! MCP 서버 설정은 ~/.claude/mcp_servers.json 또는
//! 프로젝트의 .mcp_servers.json 에서 자동으로 로드합니다.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── MCP 툴 정의 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub server: String,  // 이 툴을 제공하는 서버 이름
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct McpToolResult {
    pub success: bool,
    pub output: String,
    pub is_error: bool,
}

// ─── MCP 서버 설정 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub transport: String,  // "stdio" | "http"
    // stdio 용
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    // http 용
    pub url: Option<String>,
}

// ─── MCP 메시지 타입 ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

// ─── MCP 클라이언트 ──────────────────────────────────────────────────────────

pub struct McpClient {
    pub config: McpServerConfig,
    request_id: std::sync::atomic::AtomicU64,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            request_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// 서버에서 툴 목록을 가져옴
    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let result = self.call_method("tools/list", serde_json::json!({})).await?;

        let tools = result["tools"].as_array()
            .ok_or_else(|| anyhow::anyhow!("tools/list 응답에 tools 배열 없음"))?;

        let mut out = Vec::new();
        for t in tools {
            let name = t["name"].as_str().unwrap_or("").to_string();
            let description = t["description"].as_str().unwrap_or("").to_string();
            let input_schema = t["inputSchema"].clone();
            if !name.is_empty() {
                out.push(McpTool {
                    name,
                    description,
                    server: self.config.name.clone(),
                    input_schema,
                });
            }
        }
        Ok(out)
    }

    /// 툴 호출
    pub async fn call_tool(&self, tool_name: &str, arguments: serde_json::Value) -> Result<McpToolResult> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        let result = self.call_method("tools/call", params).await?;

        // MCP 툴 결과 Parsing
        let is_error = result["isError"].as_bool().unwrap_or(false);
        let content = result["content"].as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        if c["type"].as_str() == Some("text") {
                            c["text"].as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| result.to_string());

        Ok(McpToolResult {
            success: !is_error,
            output: content,
            is_error,
        })
    }

    /// JSON-RPC 메서드 호출 (트랜스포트별 분기)
    async fn call_method(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match self.config.transport.as_str() {
            "stdio" => self.call_stdio(method, params).await,
            "http" | "sse" => self.call_http(method, params).await,
            t => anyhow::bail!("지원하지 않는 MCP 트랜스포트: {}", t),
        }
    }

    /// stdio 트랜스포트: 자식 프로세스를 생성하여 stdin/stdout으로 통신
    async fn call_stdio(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::process::Command;

        let cmd = self.config.command.as_deref()
            .ok_or_else(|| anyhow::anyhow!("stdio MCP 서버에 command 필드 필요: {}", self.config.name))?;
        let args = self.config.args.as_deref().unwrap_or(&[]);

        let mut child = Command::new(cmd)
            .args(args)
            .envs(self.config.env.as_ref().unwrap_or(&HashMap::new()))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| format!("MCP 서버 실행 실패: {} {}", cmd, args.join(" ")))?;

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // MCP는 초기화 핸드쉐이크가 필요
        let init_req = serde_json::to_string(&JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: self.next_id(),
            method: "initialize".into(),
            params: serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "ai_agent", "version": "0.1.0" }
            }),
        })?;
        stdin.write_all(format!("{}\n", init_req).as_bytes()).await?;

        // 실제 요청
        let req = serde_json::to_string(&JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: self.next_id(),
            method: method.to_string(),
            params,
        })?;
        stdin.write_all(format!("{}\n", req).as_bytes()).await?;
        stdin.shutdown().await?;

        // 응답 읽기 (두 번째 줄이 실제 응답)
        let mut reader = BufReader::new(stdout);
        let mut _init_line = String::new();
        reader.read_line(&mut _init_line).await?;  // initialize 응답 스킵

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let _ = child.kill().await;

        let resp: JsonRpcResponse = serde_json::from_str(response_line.trim())
            .with_context(|| format!("MCP 응답 파싱 실패: {}", crate::utils::trunc(&response_line, 200)))?;

        if let Some(err) = resp.error {
            anyhow::bail!("MCP 오류 {}: {}", err.code, err.message);
        }

        resp.result.ok_or_else(|| anyhow::anyhow!("MCP 응답에 result 없음"))
    }

    /// HTTP 트랜스포트: SSE 또는 REST 엔드포인트
    async fn call_http(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let base_url = self.config.url.as_deref()
            .ok_or_else(|| anyhow::anyhow!("http MCP 서버에 url 필드 필요: {}", self.config.name))?;

        let client = reqwest::Client::new();
        let req_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": method,
            "params": params
        });

        let resp = client.post(base_url)
            .header("Content-Type", "application/json")
            .json(&req_body)
            .send()
            .await
            .with_context(|| format!("MCP HTTP 요청 실패: {}", base_url))?;

        let resp_json: JsonRpcResponse = resp.json().await
            .context("MCP HTTP 응답 파싱 실패")?;

        if let Some(err) = resp_json.error {
            anyhow::bail!("MCP 오류 {}: {}", err.code, err.message);
        }

        resp_json.result.ok_or_else(|| anyhow::anyhow!("MCP HTTP 응답에 result 없음"))
    }
}

// ─── 설정 파일 로드 ──────────────────────────────────────────────────────────

/// ~/.claude/mcp_servers.json 또는 ./.mcp_servers.json 에서 서버 설정 로드
pub fn load_mcp_configs() -> Vec<McpServerConfig> {
    let mut configs = Vec::new();

    // 전역 설정
    if let Ok(home) = std::env::var("HOME") {
        let path = std::path::PathBuf::from(home).join(".claude").join("mcp_servers.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<McpServerConfig>>(&content) {
                configs.extend(list);
            }
        }
    }

    // 프로젝트 설정
    if let Ok(content) = std::fs::read_to_string(".mcp_servers.json") {
        if let Ok(list) = serde_json::from_str::<Vec<McpServerConfig>>(&content) {
            // 이름 충돌 시 프로젝트 설정이 우선
            for cfg in list {
                configs.retain(|c: &McpServerConfig| c.name != cfg.name);
                configs.push(cfg);
            }
        }
    }

    configs
}
