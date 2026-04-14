//! TOML configuration file management
//!
//! Priority (higher overrides lower):
//!   1. Environment variables (OLLAMA_MODEL, OLLAMA_API_URL, etc.)
//!   2. Project local: ./ai-agent.toml
//!   3. Global: ~/.config/ai-agent/config.toml
//!   4. Defaults
//!
//! Example config.toml:
//! ```toml
//! [ollama]
//! model = "gemma4:e4b"
//! api_url = "http://localhost:11434"
//! timeout_secs = 600
//!
//! [agent]
//! max_turns = 20
//! history_enabled = true
//! history_max_context = 30
//!
//! [agile]
//! project = "myproject"
//! max_qa_retries = 3
//! max_security_rounds = 5
//!
//! [discord]
//! token = ""       # DISCORD_TOKEN environment variable recommended
//!
//! [mcp]
//! config_path = "~/.claude/mcp_servers.json"
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Configuration sections ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub model: String,
    pub api_url: String,
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: "gemma4:e4b".to_string(),
            api_url: "http://localhost:11434".to_string(),
            timeout_secs: 600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_turns: usize,
    pub history_enabled: bool,
    pub history_max_context: usize,  // Maximum number of messages to load from previous session
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 20,
            history_enabled: true,
            history_max_context: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgileConfig {
    pub project: String,
    pub max_qa_retries: usize,
    pub max_security_rounds: usize,
}

impl Default for AgileConfig {
    fn default() -> Self {
        Self {
            project: "project".to_string(),
            max_qa_retries: 3,
            max_security_rounds: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscordConfig {
    pub token: String,
    pub prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub config_path: String,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            config_path: "~/.claude/mcp_servers.json".to_string(),
        }
    }
}

// ─── Full configuration ───────────────────────────────────────────────────────

/// Per-role model assignment (falls back to default ollama.model if absent)
///
/// Example (ai-agent.toml):
/// ```toml
/// [roles]
/// architect = "llama3:70b"
/// developer = "codellama:34b"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RolesConfig {
    pub product_owner: Option<String>,
    pub scrum_master: Option<String>,
    pub business_analyst: Option<String>,
    pub ux_designer: Option<String>,
    pub architect: Option<String>,
    pub developer: Option<String>,
    pub reviewer: Option<String>,
    pub qa_engineer: Option<String>,
    pub tech_lead: Option<String>,
    pub devops_engineer: Option<String>,
    pub technical_writer: Option<String>,
    pub sre: Option<String>,
    pub release_manager: Option<String>,
}

impl RolesConfig {
    /// Look up model by role name (lowercase English)
    pub fn model_for(&self, role_name: &str) -> Option<String> {
        match role_name.to_lowercase().replace(' ', "_").as_str() {
            "productowner"    | "product_owner"    => self.product_owner.clone(),
            "scrummaster"     | "scrum_master"     => self.scrum_master.clone(),
            "businessanalyst" | "business_analyst" => self.business_analyst.clone(),
            "uxdesigner"      | "ux_designer"      => self.ux_designer.clone(),
            "architect"                            => self.architect.clone(),
            "developer"                            => self.developer.clone(),
            "reviewer"                             => self.reviewer.clone(),
            "qaengineer"      | "qa_engineer"      => self.qa_engineer.clone(),
            "techlead"        | "tech_lead"        => self.tech_lead.clone(),
            "devopsengineer"  | "devops_engineer"  => self.devops_engineer.clone(),
            "technicalwriter" | "technical_writer" => self.technical_writer.clone(),
            "sre"                                  => self.sre.clone(),
            "releasemanager"  | "release_manager"  => self.release_manager.clone(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub agile: AgileConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub roles: RolesConfig,
}

impl AppConfig {
    /// Load from config file (default if missing)
    pub fn load() -> Self {
        let mut config = Self::default();

        // Load global config first
        if let Some(global) = Self::global_config_path() {
            if let Ok(c) = Self::from_file(&global) {
                config.merge(c);
            }
        }

        // Override with project local config
        let local = PathBuf::from("ai-agent.toml");
        if let Ok(c) = Self::from_file(&local) {
            config.merge(c);
        }

        // Final override with environment variables
        config.apply_env_vars();
        config
    }

    fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn global_config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let path = PathBuf::from(home).join(".config").join("ai-agent").join("config.toml");
        Some(path)
    }

    /// Override with non-default values from other config
    fn merge(&mut self, other: AppConfig) {
        if other.ollama.model != OllamaConfig::default().model {
            self.ollama.model = other.ollama.model;
        }
        if other.ollama.api_url != OllamaConfig::default().api_url {
            self.ollama.api_url = other.ollama.api_url;
        }
        if other.ollama.timeout_secs != OllamaConfig::default().timeout_secs {
            self.ollama.timeout_secs = other.ollama.timeout_secs;
        }
        if other.agent.max_turns != AgentConfig::default().max_turns {
            self.agent.max_turns = other.agent.max_turns;
        }
        if !other.agent.history_enabled {
            self.agent.history_enabled = false;
        }
        if other.agile.project != AgileConfig::default().project {
            self.agile.project = other.agile.project;
        }
        if !other.discord.token.is_empty() {
            self.discord.token = other.discord.token;
        }
        if !other.discord.prefix.is_empty() {
            self.discord.prefix = other.discord.prefix;
        }
    }

    fn apply_env_vars(&mut self) {
        if let Ok(v) = std::env::var("OLLAMA_MODEL") {
            self.ollama.model = v;
        }
        if let Ok(v) = std::env::var("OLLAMA_API_URL") {
            self.ollama.api_url = v;
        }
        if let Ok(v) = std::env::var("DISCORD_TOKEN") {
            self.discord.token = v;
        }
        if let Ok(v) = std::env::var("AI_PROJECT") {
            self.agile.project = v;
        }
    }

    /// Save configuration to file (for template generation)
    pub fn save_default(path: &PathBuf) -> Result<()> {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config)?;
        let header = "# ai-agent configuration file\n# Environment variables (OLLAMA_MODEL, etc.) override settings in this file.\n\n";
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, format!("{}{}", header, toml_str))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_config_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.ollama.model, "gemma4:e4b");
        assert_eq!(cfg.ollama.api_url, "http://localhost:11434");
        assert!(cfg.agent.history_enabled);
        assert_eq!(cfg.agile.max_qa_retries, 3);
    }

    #[test]
    fn load_from_toml_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let content = r#"
[ollama]
model = "llama3:8b"
api_url = "http://localhost:11434"
timeout_secs = 300

[agent]
max_turns = 15
history_enabled = false
history_max_context = 10
"#;
        std::fs::write(&path, content).unwrap();
        let cfg = AppConfig::from_file(&path).unwrap();
        assert_eq!(cfg.ollama.model, "llama3:8b");
        assert_eq!(cfg.ollama.timeout_secs, 300);
        assert!(!cfg.agent.history_enabled);
        assert_eq!(cfg.agent.max_turns, 15);
    }

    #[test]
    fn save_default_creates_valid_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        AppConfig::save_default(&path).unwrap();
        let loaded = AppConfig::from_file(&path).unwrap();
        assert_eq!(loaded.ollama.model, "gemma4:e4b");
    }

    #[test]
    fn merge_overwrites_only_changed() {
        let mut base = AppConfig::default();
        let mut other = AppConfig::default();
        other.ollama.model = "new-model".to_string();
        base.merge(other);
        assert_eq!(base.ollama.model, "new-model");
        // api_url should remain unchanged
        assert_eq!(base.ollama.api_url, "http://localhost:11434");
    }
}