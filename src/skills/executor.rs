//! Skill 실행기 — 스킬 프롬프트를 확장하여 AI에 전달

use anyhow::Result;
use crate::agent::ollama::OllamaClient;
use crate::models::Message;
use crate::skills::loader::SkillRegistry;

/// 스킬을 이름과 인자로 실행하고 AI 응답 반환
pub async fn execute_skill(
    registry: &SkillRegistry,
    client: &OllamaClient,
    skill_name: &str,
    args: &[&str],
    on_token: impl Fn(&str),
) -> Result<String> {
    let skill = registry.get(skill_name)
        .ok_or_else(|| anyhow::anyhow!("스킬을 찾을 수 없음: '{}'\n사용 가능: {}",
            skill_name,
            registry.all().iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
        ))?;

    let prompt = skill.expand(args);
    let history = vec![
        Message::system("당신은 도움이 되는 AI 에이전트입니다."),
        Message::user(&prompt),
    ];

    let result = client.chat_stream(history, on_token).await?;
    Ok(result)
}
