//! Skill executor — expands skill prompts and sends them to AI

use anyhow::Result;
use crate::agent::ollama::OllamaClient;
use crate::models::Message;
use crate::skills::loader::SkillRegistry;

/// Execute a skill by name and args, returns AI response
pub async fn execute_skill(
    registry: &SkillRegistry,
    client: &OllamaClient,
    skill_name: &str,
    args: &[&str],
    on_token: impl Fn(&str),
) -> Result<String> {
    let skill = registry.get(skill_name)
        .ok_or_else(|| anyhow::anyhow!("Skill not found: '{}'\nAvailable: {}",
            skill_name,
            registry.all().iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
        ))?;

    let prompt = skill.expand(args);
    let history = vec![
        Message::system("You are a helpful AI agent."),
        Message::user(&prompt),
    ];

    let result = client.chat_stream(history, on_token).await?;
    Ok(result)
}
