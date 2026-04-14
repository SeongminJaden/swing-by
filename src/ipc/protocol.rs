//! AI-to-AI communication protocol
//!
//! Other AIs (Claude Code, Gemini, Grok, ChatGPT, etc.) or automation tools can
//! call this agent via stdio or HTTP using this standard protocol.
//!
//! Protocol:
//!   - JSON-RPC 2.0 based
//!   - MCP (Model Context Protocol) compatible
//!   - stdin/stdout (stdio mode) or HTTP POST (server mode)
//!
//! Supported methods:
//!   initialize        — handshake and capability negotiation
//!   capabilities      — list supported features
//!   chat              — general conversation request
//!   run_tool          — direct tool execution
//!   agile_sprint      — agile sprint execution
//!   board_status      — agile board status query
//!   ping              — connection check

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Capability declarations ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub version: String,
}

pub fn declare_capabilities() -> Vec<AgentCapability> {
    vec![
        AgentCapability { name: "chat".into(), description: "General conversation and coding support".into(), version: "1.0".into() },
        AgentCapability { name: "tools".into(), description: "File system, shell, git and other tool execution".into(), version: "1.0".into() },
        AgentCapability { name: "agile".into(), description: "Agile sprint and board management".into(), version: "1.0".into() },
        AgentCapability { name: "multi_agent".into(), description: "Multi-agent pipeline".into(), version: "1.0".into() },
        AgentCapability { name: "mcp_proxy".into(), description: "MCP server proxy".into(), version: "1.0".into() },
        AgentCapability { name: "memory".into(), description: "Persistent memory store/query".into(), version: "1.0".into() },
    ]
}

// ─── JSON-RPC messages ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }

    pub fn err(id: serde_json::Value, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".into(), id,
            result: None,
            error: Some(JsonRpcError { code, message: message.to_string(), data: None }),
        }
    }
}

// ─── High-level request/response types ───────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRequest {
    pub method: String,
    pub params: HashMap<String, serde_json::Value>,
    /// Caller AI info (for identification)
    pub caller_id: Option<String>,
    pub caller_type: Option<String>,  // "claude-code", "gemini", "grok", "custom"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentResponse {
    pub success: bool,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
}

impl AgentResponse {
    pub fn success(content: &str) -> Self {
        Self { success: true, content: content.to_string(), metadata: HashMap::new(), error: None }
    }

    pub fn failure(error: &str) -> Self {
        Self { success: false, content: String::new(), metadata: HashMap::new(), error: Some(error.to_string()) }
    }

    pub fn with_meta(mut self, key: &str, val: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), val);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonrpc_ok_response_serializes() {
        let resp = JsonRpcResponse::ok(
            serde_json::json!(1),
            serde_json::json!({"status": "ok"}),
        );
        let j = serde_json::to_string(&resp).unwrap();
        assert!(j.contains("\"jsonrpc\":\"2.0\""));
        assert!(j.contains("\"result\""));
        assert!(!j.contains("\"error\""), "ok response must not include error field");
    }

    #[test]
    fn jsonrpc_err_response_serializes() {
        let resp = JsonRpcResponse::err(serde_json::json!(42), -32600, "Invalid Request");
        let j = serde_json::to_string(&resp).unwrap();
        assert!(j.contains("\"error\""));
        assert!(j.contains("-32600"));
        assert!(!j.contains("\"result\""), "error response must not include result field");
    }

    #[test]
    fn jsonrpc_request_roundtrip() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "ping");
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn agent_response_success_failure() {
        let ok = AgentResponse::success("done");
        assert!(ok.success);
        assert_eq!(ok.content, "done");
        assert!(ok.error.is_none());

        let fail = AgentResponse::failure("oops");
        assert!(!fail.success);
        assert!(fail.error.as_deref() == Some("oops"));
    }

    #[test]
    fn capabilities_not_empty() {
        let caps = declare_capabilities();
        assert!(!caps.is_empty());
        assert!(caps.iter().any(|c| c.name == "chat"));
        assert!(caps.iter().any(|c| c.name == "agile"));
    }
}
