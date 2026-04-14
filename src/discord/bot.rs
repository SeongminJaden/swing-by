//! Discord 봇 구현

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

const DISCORD_MAX_LEN: usize = 1900;  // Discord 메시지 최대 2000자, 안전 마진

// ─── 봇 데이터 키 ────────────────────────────────────────────────────────────

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

// ─── 이벤트 핸들러 ───────────────────────────────────────────────────────────

struct BotHandler;

#[async_trait]
impl EventHandler for BotHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("Discord 봇 연결됨: {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: DMsg) {
        // 봇 자신의 메시지 무시
        if msg.author.bot { return; }

        let data = ctx.data.read().await;
        let prefix = data.get::<PrefixKey>().cloned().unwrap_or_else(|| "!".to_string());
        let allowed_channel = *data.get::<AllowedChannelKey>().unwrap();
        let client = Arc::clone(data.get::<ClientKey>().unwrap());
        let sessions = data.get::<SessionKey>().cloned().unwrap();
        drop(data);

        // 채널 필터
        if let Some(ch) = allowed_channel {
            if msg.channel_id.get() != ch { return; }
        }

        // 접두사 확인
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
                    .unwrap_or((0, "새 세션".to_string()));
                let text = format!(
                    "**AI Agent 상태**\n\
                     모델: `{}`\n\
                     채널 히스토리: {} 메시지\n\
                     세션 시간: {}",
                    client.model(), msg_count, elapsed
                );
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            "clear" => {
                let system = build_system_prompt(AgentRole::General, client.model());
                sessions.clear(msg.channel_id.get(), &system);
                send_chunked(&ctx, msg.channel_id, "세션이 초기화되었습니다.").await;
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
                    "히스토리 없음".to_string()
                } else {
                    format!("**최근 {} 메시지**\n{}", lines.len(), lines.join("\n"))
                };
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            "help" | "h" => {
                let text = format!(
                    "**AI Agent 명령어** (접두사: `{}`)\n\
                     `{}ask <질문>` — 일반 질문\n\
                     `{}code <요청>` — 코드 생성\n\
                     `{}plan <작업>` — 작업 기획\n\
                     `{}debug <문제>` — 디버깅\n\
                     `{}pipeline <작업>` — 기획→개발→디버깅 전체 실행\n\
                     `{}status` — 에이전트 상태\n\
                     `{}clear` — 채널 세션 초기화\n\
                     `{}history [n]` — 최근 n개 히스토리",
                    prefix, prefix, prefix, prefix, prefix, prefix, prefix, prefix, prefix
                );
                send_chunked(&ctx, msg.channel_id, &text).await;
            }
            _ => {
                // 접두사만 있고 명령어 없으면 ask로 처리
                if !content.is_empty() {
                    handle_ask(&ctx, &msg, &client, &sessions, content, AgentRole::General).await;
                }
            }
        }
    }
}

// ─── 핸들러 함수들 ───────────────────────────────────────────────────────────

async fn handle_ask(
    ctx: &Context,
    msg: &DMsg,
    client: &OllamaClient,
    sessions: &SessionStore,
    prompt: &str,
    role: AgentRole,
) {
    if prompt.is_empty() {
        send_chunked(ctx, msg.channel_id, "질문을 입력해주세요.").await;
        return;
    }

    // 타이핑 표시
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let system = build_system_prompt(role, client.model());
    let mut history = sessions.get_or_create(msg.channel_id.get(), &system);

    // 첨부 파일 처리 (텍스트 파일 컨텍스트 주입)
    let mut user_content = prompt.to_string();
    for attachment in &msg.attachments {
        if attachment.size < 500_000 {  // 500KB 이하만
            if let Ok(bytes) = attachment.download().await {
                if let Ok(text) = String::from_utf8(bytes) {
                    user_content.push_str(&format!(
                        "\n\n## 첨부 파일: {}\n```\n{}\n```",
                        attachment.filename,
                        crate::utils::trunc(&text, 3000)
                    ));
                }
            }
        }
    }

    history.push(Message::user(&user_content));

    // 최대 10턴 툴 호출 루프
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
                                results.push(format!("툴 '{}' 결과:\n{}", name, result.output));
                            }
                        }
                        history.push(Message::assistant(&ai_text));
                        history.push(Message::tool(results.join("\n\n")));
                        response_text = format!("[다중 툴 실행 완료]\n{}", results.join("\n"));
                    }
                    crate::models::AgentResponse::ToolCall(tc) => {
                        let result = dispatch_tool(&tc).await;
                        history.push(Message::assistant(&ai_text));
                        history.push(Message::tool(format!("툴 '{}' 결과:\n{}", tc.name, result.output)));
                        response_text = format!("🔧 `{}` 실행 중...", tc.name);
                    }
                }
            }
            Err(e) => {
                response_text = format!("오류: {}", e);
                break;
            }
        }
    }

    sessions.update(msg.channel_id.get(), history);

    if response_text.is_empty() {
        response_text = "응답 없음".to_string();
    }

    // 역할 이모지 헤더
    let header = match role {
        AgentRole::Planner   => "📋 **기획 에이전트**\n",
        AgentRole::Developer => "💻 **개발 에이전트**\n",
        AgentRole::Debugger  => "🔍 **디버그 에이전트**\n",
        AgentRole::Reviewer  => "👁️ **리뷰 에이전트**\n",
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
        send_chunked(ctx, msg.channel_id, "사용법: `!pipeline <작업 설명>`").await;
        return;
    }

    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;
    send_chunked(ctx, msg.channel_id, &format!("🚀 **파이프라인 시작**: {}", task)).await;

    match run_pipeline(client, task).await {
        Ok(result) => {
            let text = format!(
                "✅ **파이프라인 완료**\n\n\
                 📋 **기획**\n{}\n\n\
                 💻 **구현**\n{}\n\n\
                 🔍 **검증**\n{}",
                crate::utils::trunc(&result.plan, 500),
                crate::utils::trunc(&result.implementation, 800),
                crate::utils::trunc(&result.verification, 400),
            );
            send_chunked(ctx, msg.channel_id, &text).await;
        }
        Err(e) => {
            send_chunked(ctx, msg.channel_id, &format!("❌ 파이프라인 실패: {}", e)).await;
        }
    }
}

// ─── 유틸리티 ────────────────────────────────────────────────────────────────

/// 2000자 제한에 맞게 메시지 분할 전송
async fn send_chunked(ctx: &Context, channel: ChannelId, text: &str) {
    if text.len() <= DISCORD_MAX_LEN {
        let _ = channel.say(&ctx.http, text).await;
        return;
    }

    let mut remaining = text;
    let mut part = 1;
    while !remaining.is_empty() {
        // 문자 경계에서 자르기
        let cut = if remaining.len() <= DISCORD_MAX_LEN {
            remaining.len()
        } else {
            let mut end = DISCORD_MAX_LEN;
            while end > 0 && !remaining.is_char_boundary(end) { end -= 1; }
            // 가능하면 줄바꿈에서 자르기
            if let Some(nl) = remaining[..end].rfind('\n') {
                nl + 1
            } else {
                end
            }
        };

        let chunk = &remaining[..cut];
        remaining = &remaining[cut..];

        let header = if part > 1 { format!("*(계속 {})* ", part) } else { String::new() };
        let _ = channel.say(&ctx.http, format!("{}{}", header, chunk)).await;
        part += 1;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

fn build_system_prompt(role: AgentRole, model: &str) -> String {
    let role_prompt = match role {
        AgentRole::General => crate::agent::tools::tool_descriptions().to_string(),
        AgentRole::Planner => format!(
            "{}\n\n=== 역할: 기획 에이전트 ===\n\
             당신은 소프트웨어 기획 전문가입니다.\n\
             사용자의 요청을 분석하여:\n\
             1. 명확한 요구사항 정의\n\
             2. 기술 스택 선정\n\
             3. 단계별 구현 계획\n\
             4. 위험 요소 및 대안\n\
             을 체계적으로 작성하세요.",
            crate::agent::tools::tool_descriptions()
        ),
        AgentRole::Developer => format!(
            "{}\n\n=== 역할: 개발 에이전트 ===\n\
             당신은 시니어 소프트웨어 엔지니어입니다.\n\
             코드를 작성할 때:\n\
             1. 에러 처리 철저히\n\
             2. 테스트 코드 포함\n\
             3. 주요 로직에 주석\n\
             4. 성능과 보안 고려\n\
             실제 동작하는 코드를 작성하세요.",
            crate::agent::tools::tool_descriptions()
        ),
        AgentRole::Debugger => format!(
            "{}\n\n=== 역할: 디버그 에이전트 ===\n\
             당신은 디버깅 전문가입니다.\n\
             문제를 분석할 때:\n\
             1. 증상과 근본 원인 구분\n\
             2. 재현 방법 확인\n\
             3. 단계적 원인 추적\n\
             4. 수정 및 검증\n\
             근거 기반으로 분석하세요.",
            crate::agent::tools::tool_descriptions()
        ),
        AgentRole::Reviewer => format!(
            "{}\n\n=== 역할: 코드 리뷰 에이전트 ===\n\
             당신은 코드 리뷰 전문가입니다.\n\
             리뷰 시 확인 항목:\n\
             1. 버그 및 엣지 케이스\n\
             2. 보안 취약점\n\
             3. 성능 병목\n\
             4. 코드 품질 및 유지보수성\n\
             5. 구체적인 개선 제안",
            crate::agent::tools::tool_descriptions()
        ),
    };
    format!("모델: {}\n\n{}", model, role_prompt)
}

// ─── 봇 실행 진입점 ──────────────────────────────────────────────────────────

pub async fn run_discord_bot(client: Arc<OllamaClient>) -> Result<()> {
    let token = std::env::var("DISCORD_TOKEN")
        .map_err(|_| anyhow::anyhow!(
            "DISCORD_TOKEN 환경변수가 설정되지 않았습니다.\n\
             Discord Developer Portal에서 봇 토큰을 발급받아 설정하세요."
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
        .map_err(|e| anyhow::anyhow!("Discord 클라이언트 생성 실패: {}", e))?;

    {
        let mut data = discord_client.data.write().await;
        data.insert::<ClientKey>(client);
        data.insert::<SessionKey>(SessionStore::new());
        data.insert::<PrefixKey>(prefix.clone());
        data.insert::<AllowedChannelKey>(allowed_channel);
    }

    println!("Discord 봇 시작 (접두사: '{}')", prefix);
    if let Some(ch) = allowed_channel {
        println!("허용 채널: {}", ch);
    }

    discord_client.start().await
        .map_err(|e| anyhow::anyhow!("Discord 봇 실행 실패: {}", e))
}
