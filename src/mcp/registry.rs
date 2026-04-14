//! MCP server registry — manages multiple MCP servers and provides unified tool access

use anyhow::Result;
use std::collections::HashMap;
use super::client::{load_mcp_configs, McpClient, McpTool, McpToolResult};

pub struct McpRegistry {
    clients: HashMap<String, McpClient>,
    tools: Vec<McpTool>,  // unified tool list from all servers
}

impl McpRegistry {
    /// Automatically load and initialize MCP servers from config file
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

    /// Collect tool list from all servers (async, skips failed servers)
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
                    eprintln!("[MCP] Server '{}' tool discovery failed: {}", client.config.name, e);
                }
            }
        }
        self.tools.len()
    }

    /// Return list of all registered tools
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// Find and execute a tool by name
    pub async fn call_tool(&self, tool_name: &str, arguments: serde_json::Value) -> Result<McpToolResult> {
        // Find which server it belongs to
        let tool = self.tools.iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| anyhow::anyhow!("MCP tool not found: {}", tool_name))?;

        let client = self.clients.get(&tool.server)
            .ok_or_else(|| anyhow::anyhow!("MCP server not found: {}", tool.server))?;

        client.call_tool(tool_name, arguments).await
    }

    /// Generate MCP tool descriptions to add to AI system prompt
    pub fn tool_descriptions_for_prompt(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let mut lines = vec![
            "\n## MCP tools (external servers)".to_string(),
            "MCP tool call format: TOOL: mcp_call <server> <tool_name> <JSON arguments>".to_string(),
            String::new(),
        ];

        for tool in &self.tools {
            lines.push(format!("- **{}** (server: {}): {}", tool.name, tool.server, tool.description));
        }

        lines.join("\n")
    }

    /// Number of connected servers
    pub fn server_count(&self) -> usize {
        self.clients.len()
    }

    /// List of server names
    pub fn server_names(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }
}
