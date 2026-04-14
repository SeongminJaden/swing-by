//! AI-to-AI 통신 프로토콜
//!
//! 다른 AI (Claude Code, Gemini, Grok, ChatGPT 등) 또는 자동화 도구가
//! stdio 또는 HTTP로 이 에이전트를 호출할 수 있는 표준 프로토콜입니다.
//!
//! 프로토콜:
//!   - JSON-RPC 2.0 기반
//!   - MCP (Model Context Protocol) 호환
//!   - stdin/stdout (stdio 모드) 또는 HTTP POST (서버 모드)
//!
//! 지원 메서드:
//!   initialize        — 핸드쉐이크 및 능력 협상
//!   capabilities      — 지원 기능 목록
//!   chat              — 일반 대화 요청
//!   run_tool          — 툴 직접 실행
//!   agile_sprint      — 애자일 Sprint 실행
//!   board_status      — 애자일 보드 상태 조회
//!   ping              — 연결 확인

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 능력 선언 ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub version: String,
}

pub fn declare_capabilities() -> Vec<AgentCapability> {
    vec![
        AgentCapability { name: "chat".into(), description: "일반 대화 및 코딩 지원".into(), version: "1.0".into() },
        AgentCapability { name: "tools".into(), description: "파일 시스템, 쉘, git 등 툴 실행".into(), version: "1.0".into() },
        AgentCapability { name: "agile".into(), description: "애자일 스프린트 및 보드 관리".into(), version: "1.0".into() },
        AgentCapability { name: "multi_agent".into(), description: "다중 에이전트 파이프라인".into(), version: "1.0".into() },
        AgentCapability { name: "mcp_proxy".into(), description: "MCP 서버 프록시".into(), version: "1.0".into() },
        AgentCapability { name: "memory".into(), description: "영속 메모리 저장/조회".into(), version: "1.0".into() },
    ]
}

// ─── JSON-RPC 메시지 ─────────────────────────────────────────────────────────

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

// ─── 고수준 요청/응답 타입 ───────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRequest {
    pub method: String,
    pub params: HashMap<String, serde_json::Value>,
    /// 호출자 AI 정보 (식별용)
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
