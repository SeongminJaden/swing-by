//! Discord bot implementation

use anyhow::Result;
use serenity::{
    async_trait,
    model::{channel::Message as DMsg, gateway::Ready, id::ChannelId},
    prelude::*,
};
use std::sync::Arc;

use crate::agent::{
    ollama::OllamaClient,
    tools::dispatch_tool,
    orchestrator::{run_pipeline, AgentRole},
};
use crate::models::Message;
use crate::discord::session::SessionStore;

const DISCORD_MAX_LEN: usize = 1900;  // Discord message max 2000 chars, safety margin

// ─── Bot data keys ───────────────────────────────────────────────────────────

struct ClientKey;
impl TypeMapKey for ClientKey {
    type Value = Arc<OllamaClient>;
}

struct SessionKey;
impl TypeMapKey for SessionKey {
    type Value = SessionStore;
}

struct PrefixKey;
impl TypeMapKey for PrefixKey {
    type Value = String;
}

struct AllowedChannelKey;
impl TypeMapKey for AllowedChannelKey {
    type Value = Option<u64>;
}

// ─── Event handler ───────────────────────────────────────────────────────────

struct BotHandler;

#[async_trait]
impl EventHandler for BotHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("Discord bot connected: {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: DMsg) {
        // Ignore messages from the bot itself
        if msg.author.bot { return; }

        let data = ctx.data.read().await;
        let prefix = data.get::<PrefixKey>().cloned().unwrap_or_else(|| "!".to_string());
        let allowed_channel = *data.get::<AllowedChannelKey>().unwrap();
        let client = Arc::clone(data.get::<ClientKey>().unwrap());
        let sessions = data.get::<SessionKey>().cloned().unwrap();
        drop(data);

        // Channel filter
        if let Some(ch) = allowed_channel {
            if msg.channel_id.get() != ch { return; }
        }

        // Check prefix
        if !msg.content.starts_with(&prefix) { return; }

        let content = msg.content[prefix.len()..].trim();
        let (cmd, rest) = content.split_once(|c: char| c.is_whitespace())
            .unwrap_or((content, ""));
        let rest = rest.trim();

        match cmd.to_lowercase().as_str() {
            "ask" | "q" => {
                handle_ask(&ctx, &msg, &client, &sessions, rest, AgentRole::General).await;
            }
            "code" | "dev" => {
                handle_ask(&ctx, &msg, &client, &sessions, rest, AgentRole::Developer).await;
            }
            "plan" => {
                handle_ask(&ctx, &msg, &client, &sessions, rest, AgentRole::Planner).await;
            }
            "debug" => {
                handle_ask(&ctx, &msg, &client, &sessions, rest, AgentRole::Debugger).await;
            }
            "pipeline" | "pipe" => {
                handle_pipeline(&ctx, &msg, &client, rest).await;
            }
            "status" => {
                let (msg_count, elapsed) = sessions.stats(msg.channel_id.get())
                    .unwrap_or((0, "New session".to_string()));
                let text = format!(
                    "**AI Agent Status**\n\
                     Model: `{}`\n\
                     Channel history: {} messages\n\
                     Session time: {}",
                    client.model(), msg_count, elapsed
                );
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            "clear" => {
                let system = build_system_prompt(AgentRole::General, client.model());
                sessions.clear(msg.channel_id.get(), &system);
                send_chunked(&ctx, msg.channel_id, "Session cleared.").await;
            }
            "history" => {
                let n: usize = rest.parse().unwrap_or(5);
                let history = sessions.get_or_create(
                    msg.channel_id.get(),
                    &build_system_prompt(AgentRole::General, client.model())
                );
                let skip = history.len().saturating_sub(n);
                let lines: Vec<String> = history.iter().skip(skip)
                    .filter(|m| !matches!(m.role, crate::models::Role::System))
                    .map(|m| format!("[{:?}] {}", m.role, crate::utils::trunc(&m.content, 100)))
                    .collect();
                let text = if lines.is_empty() {
                    "No history".to_string()
                } else {
                    format!("**Recent {} messages**\n{}", lines.len(), lines.join("\n"))
                };
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            "help" | "h" => {
                let text = format!(
                    "**AI Agent commands** (prefix: `{}`)\n\
                     `{}ask <question>` — general question\n\
                     `{}code <request>` — code generation\n\
                     `{}plan <task>` — task planning\n\
                     `{}debug <problem>` — debugging\n\
                     `{}pipeline <task>` — full plan→dev→debug pipeline\n\
                     `{}status` — agent status\n\
                     `{}clear` — clear channel session\n\
                     `{}history [n]` — last n history entries",
                    prefix, prefix, prefix, prefix, prefix, prefix, prefix, prefix, prefix
                );
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            _ => {
                // If only prefix with no command, treat as ask
                if !content.is_empty() {
                    handle_ask(&ctx, &msg, &client, &sessions, content, AgentRole::General).await;
                }
            }
        }
    }
}

// ─── Handler functions ───────────────────────────────────────────────────────────

async fn handle_ask(
    ctx: &Context,
    msg: &DMsg,
    client: &OllamaClient,
    sessions: &SessionStore,
    prompt: &str,
    role: AgentRole,
) {
    if prompt.is_empty() {
        send_chunked(ctx, msg.channel_id, "Please enter a question.").await;
        return;
    }

    // Show typing indicator
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let system = build_system_prompt(role, client.model());
    let mut history = sessions.get_or_create(msg.channel_id.get(), &system);

    // Handle attachments (inject text file context)
    let mut user_content = prompt.to_string();
    for attachment in &msg.attachments {
        if attachment.size < 500_000 {  // only under 500KB
            if let Ok(bytes) = attachment.download().await {
                if let Ok(text) = String::from_utf8(bytes) {
                    user_content.push_str(&format!(
                        "\n\n## Attachment: {}\n```\n{}\n```",
                        attachment.filename,
                        crate::utils::trunc(&text, 3000)
                    ));
                }
            }
        }
    }

    history.push(Message::user(&user_content));

    // Max 10-turn tool call loop
    let mut response_text = String::new();
    for _turn in 0..10 {
        match client.chat(history.clone()).await {
            Ok(resp) => {
                let ai_text = resp.message.content.clone();

                match crate::agent::chat::parse_response_pub(&ai_text) {
                    crate::models::AgentResponse::Exit => break,
                    crate::models::AgentResponse::Text(_) => {
                        response_text = ai_text.clone();
                        history.push(Message::assistant(&ai_text));
                        break;
                    }
                    crate::models::AgentResponse::ToolCall(tc) if tc.name == "__multi__" => {
                        let mut results = Vec::new();
                        for raw in &tc.args {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
                                let name = val["name"].as_str().unwrap_or("").to_string();
                                let args: Vec<String> = val["args"].as_array()
                                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                    .unwrap_or_default();
                                let result = dispatch_tool(&crate::models::ToolCall { name: name.clone(), args }).await;
                                results.push(format!("Tool '{}' result:\n{}", name, result.output));
                            }
                        }
                        history.push(Message::assistant(&ai_text));
                        history.push(Message::tool(results.join("\n\n")));
                        response_text = format!("[Multi-tool execution complete]\n{}", results.join("\n"));
                    }
                    crate::models::AgentResponse::ToolCall(tc) => {
                        let result = dispatch_tool(&tc).await;
                        history.push(Message::assistant(&ai_text));
                        history.push(Message::tool(format!("Tool '{}' result:\n{}", tc.name, result.output)));
                        response_text = format!("🔧 Running `{}`...", tc.name);
                    }
                }
            }
            Err(e) => {
                response_text = format!("Error: {}", e);
                break;
            }
        }
    }

    sessions.update(msg.channel_id.get(), history);

    if response_text.is_empty() {
        response_text = "No response".to_string();
    }

    // Role emoji header
    let header = match role {
        AgentRole::Planner   => "📋 **Planner Agent**\n",
        AgentRole::Developer => "💻 **Developer Agent**\n",
        AgentRole::Debugger  => "🔍 **Debugger Agent**\n",
        AgentRole::Reviewer  => "👁️ **Reviewer Agent**\n",
        AgentRole::General   => "",
    };

    send_chunked(ctx, msg.channel_id, &format!("{}{}", header, response_text)).await;
}

async fn handle_pipeline(
    ctx: &Context,
    msg: &DMsg,
    client: &OllamaClient,
    task: &str,
) {
    if task.is_empty() {
        send_chunked(ctx, msg.channel_id, "Usage: `!pipeline <task description>`").await;
        return;
    }

    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;
    send_chunked(ctx, msg.channel_id, &format!("🚀 **Pipeline started**: {}", task)).await;

    match run_pipeline(client, task).await {
        Ok(result) => {
            let text = format!(
                "✅ **Pipeline complete**\n\n\
                 📋 **Plan**\n{}\n\n\
                 💻 **Implementation**\n{}\n\n\
                 🔍 **Verification**\n{}",
                crate::utils::trunc(&result.plan, 500),
                crate::utils::trunc(&result.implementation, 800),
                crate::utils::trunc(&result.verification, 400),
            );
            send_chunked(ctx, msg.channel_id, &text).await;
        }
        Err(e) => {
            send_chunked(ctx, msg.channel_id, &format!("❌ Pipeline failed: {}", e)).await;
        }
    }
}

// ─── Utilities ────────────────────────────────────────────────────────────────

/// Split and send message within 2000-char Discord limit
async fn send_chunked(ctx: &Context, channel: ChannelId, text: &str) {
    if text.len() <= DISCORD_MAX_LEN {
        let _ = channel.say(&ctx.http, text).await;
        return;
    }

    let mut remaining = text;
    let mut part = 1;
    while !remaining.is_empty() {
        // Cut at character boundary
        let cut = if remaining.len() <= DISCORD_MAX_LEN {
            remaining.len()
        } else {
            let mut end = DISCORD_MAX_LEN;
            while end > 0 && !remaining.is_char_boundary(end) { end -= 1; }
            // Prefer cutting at newline
            if let Some(nl) = remaining[..end].rfind('\n') {
                nl + 1
            } else {
                end
            }
        };

        let chunk = &remaining[..cut];
        remaining = &remaining[cut..];

        let header = if part > 1 { format!("*(continued {})* ", part) } else { String::new() };
        let _ = channel.say(&ctx.http, format!("{}{}", header, chunk)).await;
        part += 1;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

fn build_system_prompt(role: AgentRole, model: &str) -> String {
    let role_prompt = match role {
        AgentRole::General => crate::agent::tools::tool_descriptions().to_string(),
        AgentRole::Planner => format!(
            "{}\n\n=== Role: Planner Agent ===\n\
             You are a software planning expert.\n\
             Analyze user requests and systematically write:\n\
             1. Clear requirements definition\n\
             2. Tech stack selection\n\
             3. Step-by-step implementation plan\n\
             4. Risk factors and alternatives.",
            crate::agent::tools::tool_descriptions()
        ),
        AgentRole::Developer => format!(
            "{}\n\n=== Role: Developer Agent ===\n\
             You are a senior software engineer.\n\
             When writing code:\n\
             1. Thorough error handling\n\
             2. Include test code\n\
             3. Comment key logic\n\
             4. Consider performance and security\n\
             Write code that actually works.",
            crate::agent::tools::tool_descriptions()
        ),
        AgentRole::Debugger => format!(
            "{}\n\n=== Role: Debugger Agent ===\n\
             You are a debugging expert.\n\
             When analyzing issues:\n\
             1. Distinguish symptoms from root cause\n\
             2. Verify reproduction steps\n\
             3. Trace cause step by step\n\
             4. Fix and verify\n\
             Analyze based on evidence.",
            crate::agent::tools::tool_descriptions()
        ),
        AgentRole::Reviewer => format!(
            "{}\n\n=== Role: Code Reviewer Agent ===\n\
             You are a code review expert.\n\
             Review checklist:\n\
             1. Bugs and edge cases\n\
             2. Security vulnerabilities\n\
             3. Performance bottlenecks\n\
             4. Code quality and maintainability\n\
             5. Concrete improvement suggestions",
            crate::agent::tools::tool_descriptions()
        ),
    };
    format!("Model: {}\n\n{}", model, role_prompt)
}

// ─── Bot entry point ──────────────────────────────────────────────────────────

pub async fn run_discord_bot(client: Arc<OllamaClient>) -> Result<()> {
    let token = std::env::var("DISCORD_TOKEN")
        .map_err(|_| anyhow::anyhow!(
            "DISCORD_TOKEN environment variable is not set.\n\
             Obtain a bot token from the Discord Developer Portal and set it."
        ))?;

    let prefix = std::env::var("DISCORD_PREFIX").unwrap_or_else(|_| "!".to_string());
    let allowed_channel: Option<u64> = std::env::var("DISCORD_CHANNEL_ID")
        .ok()
        .and_then(|s| s.parse().ok());

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut discord_client = serenity::Client::builder(&token, intents)
        .event_handler(BotHandler)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create Discord client: {}", e))?;

    {
        let mut data = discord_client.data.write().await;
        data.insert::<ClientKey>(client);
        data.insert::<SessionKey>(SessionStore::new());
        data.insert::<PrefixKey>(prefix.clone());
        data.insert::<AllowedChannelKey>(allowed_channel);
    }

    println!("Discord bot started (prefix: '{}')", prefix);
    if let Some(ch) = allowed_channel {
        println!("Allowed channel: {}", ch);
    }

    discord_client.start().await
        .map_err(|e| anyhow::anyhow!("Discord bot execution failed: {}", e))
}
