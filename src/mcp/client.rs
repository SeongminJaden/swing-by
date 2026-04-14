//! MCP (Model Context Protocol) client
//!
//! Connects to MCP servers and calls tools.
//! Supported transports:
//!   - stdio: local process (most common)
//!   - http: HTTP/SSE server
//!
//! MCP server config is loaded automatically from ~/.claude/mcp_servers.json or
//! the project's .mcp_servers.json.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── MCP tool definition ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub server: String,  // name of server providing this tool
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct McpToolResult {
    pub success: bool,
    pub output: String,
    pub is_error: bool,
}

// ─── MCP server configuration ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub transport: String,  // "stdio" | "http"
    // for stdio
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    // for http
    pub url: Option<String>,
}

// ─── MCP message types ─────────────────────────────────────────────────────────

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

// ─── MCP client ──────────────────────────────────────────────────────────

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

    /// Fetch tool list from server
    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let result = self.call_method("tools/list", serde_json::json!({})).await?;

        let tools = result["tools"].as_array()
            .ok_or_else(|| anyhow::anyhow!("tools/list response missing tools array"))?;

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

    /// Call a tool
    pub async fn call_tool(&self, tool_name: &str, arguments: serde_json::Value) -> Result<McpToolResult> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        let result = self.call_method("tools/call", params).await?;

        // Parse MCP tool result
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

    /// Call JSON-RPC method (dispatch by transport)
    async fn call_method(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        match self.config.transport.as_str() {
            "stdio" => self.call_stdio(method, params).await,
            "http" | "sse" => self.call_http(method, params).await,
            t => anyhow::bail!("Unsupported MCP transport: {}", t),
        }
    }

    /// stdio transport: spawn child process and communicate via stdin/stdout
    async fn call_stdio(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::process::Command;

        let cmd = self.config.command.as_deref()
            .ok_or_else(|| anyhow::anyhow!("stdio MCP server requires command field: {}", self.config.name))?;
        let args = self.config.args.as_deref().unwrap_or(&[]);

        let mut child = Command::new(cmd)
            .args(args)
            .envs(self.config.env.as_ref().unwrap_or(&HashMap::new()))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to launch MCP server: {} {}", cmd, args.join(" ")))?;

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // MCP requires initialization handshake
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

        // Actual request
        let req = serde_json::to_string(&JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: self.next_id(),
            method: method.to_string(),
            params,
        })?;
        stdin.write_all(format!("{}\n", req).as_bytes()).await?;
        stdin.shutdown().await?;

        // Read response (second line is actual response)
        let mut reader = BufReader::new(stdout);
        let mut _init_line = String::new();
        reader.read_line(&mut _init_line).await?;  // skip initialize response

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let _ = child.kill().await;

        let resp: JsonRpcResponse = serde_json::from_str(response_line.trim())
            .with_context(|| format!("Failed to parse MCP response: {}", crate::utils::trunc(&response_line, 200)))?;

        if let Some(err) = resp.error {
            anyhow::bail!("MCP error {}: {}", err.code, err.message);
        }

        resp.result.ok_or_else(|| anyhow::anyhow!("MCP response missing result"))
    }

    /// HTTP transport: SSE or REST endpoint
    async fn call_http(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let base_url = self.config.url.as_deref()
            .ok_or_else(|| anyhow::anyhow!("http MCP server requires url field: {}", self.config.name))?;

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
            .with_context(|| format!("MCP HTTP request failed: {}", base_url))?;

        let resp_json: JsonRpcResponse = resp.json().await
            .context("Failed to parse MCP HTTP response")?;

        if let Some(err) = resp_json.error {
            anyhow::bail!("MCP error {}: {}", err.code, err.message);
        }

        resp_json.result.ok_or_else(|| anyhow::anyhow!("MCP HTTP response missing result"))
    }
}

// ─── Config file loading ──────────────────────────────────────────────────────────

/// Load server config from ~/.claude/mcp_servers.json or ./.mcp_servers.json
pub fn load_mcp_configs() -> Vec<McpServerConfig> {
    let mut configs = Vec::new();

    // Global config
    if let Ok(home) = std::env::var("HOME") {
        let path = std::path::PathBuf::from(home).join(".claude").join("mcp_servers.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<McpServerConfig>>(&content) {
                configs.extend(list);
            }
        }
    }

    // Project config
    if let Ok(content) = std::fs::read_to_string(".mcp_servers.json") {
        if let Ok(list) = serde_json::from_str::<Vec<McpServerConfig>>(&content) {
            // Project config takes priority on name conflict
            for cfg in list {
                configs.retain(|c: &McpServerConfig| c.name != cfg.name);
                configs.push(cfg);
            }
        }
    }

    configs
}
