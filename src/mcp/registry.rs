//! MCP 서버 레지스트리 — 여러 MCP 서버를 관리하고 툴을 통합 제공

use anyhow::Result;
use std::collections::HashMap;
use super::client::{load_mcp_configs, McpClient, McpTool, McpToolResult};

pub struct McpRegistry {
    clients: HashMap<String, McpClient>,
    tools: Vec<McpTool>,  // 모든 서버의 툴 통합 목록
}

impl McpRegistry {
    /// 설정 파일에서 MCP 서버를 자동으로 로드하고 초기화
    pub fn from_config() -> Self {
        let configs = load_mcp_configs();
        let mut clients = HashMap::new();
        for cfg in configs {
            let name = cfg.name.clone();
            clients.insert(name, McpClient::new(cfg));
        }
        Self { clients, tools: Vec::new() }
    }

    pub fn with_clients(clients: HashMap<String, McpClient>) -> Self {
        Self { clients, tools: Vec::new() }
    }

    /// 모든 서버에서 툴 목록 수집 (비동기, 실패 서버는 건너뜀)
    pub async fn discover_tools(&mut self) -> usize {
        self.tools.clear();
        for client in self.clients.values() {
            match client.list_tools().await {
                Ok(tools) => {
                    for t in tools {
                        self.tools.push(t);
                    }
                }
                Err(e) => {
                    eprintln!("[MCP] 서버 '{}' 툴 조회 실패: {}", client.config.name, e);
                }
            }
        }
        self.tools.len()
    }

    /// 등록된 모든 툴 목록 반환
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// 툴을 이름으로 찾아 실행
    pub async fn call_tool(&self, tool_name: &str, arguments: serde_json::Value) -> Result<McpToolResult> {
        // 어느 서버에 있는지 찾기
        let tool = self.tools.iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| anyhow::anyhow!("MCP 툴을 찾을 수 없음: {}", tool_name))?;

        let client = self.clients.get(&tool.server)
            .ok_or_else(|| anyhow::anyhow!("MCP 서버를 찾을 수 없음: {}", tool.server))?;

        client.call_tool(tool_name, arguments).await
    }

    /// AI 시스템 프롬프트에 추가할 MCP 툴 설명 생성
    pub fn tool_descriptions_for_prompt(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let mut lines = vec![
            "\n## MCP 툴 (외부 서버)".to_string(),
            "MCP 툴 호출 형식: TOOL: mcp_call <server> <tool_name> <JSON arguments>".to_string(),
            String::new(),
        ];

        for tool in &self.tools {
            lines.push(format!("- **{}** (서버: {}): {}", tool.name, tool.server, tool.description));
        }

        lines.join("\n")
    }

    /// 연결된 서버 수
    pub fn server_count(&self) -> usize {
        self.clients.len()
    }

    /// 서버 이름 목록
    pub fn server_names(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }
}
