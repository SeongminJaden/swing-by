use serde::{Deserialize, Serialize};

/// Ollama API 메시지 역할
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// 채팅 메시지
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into() }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self { role: Role::Tool, content: content.into() }
    }
}

/// Ollama /api/chat 요청 바디
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ChatOptions>,
}

/// 모델 파라미터 옵션
#[derive(Debug, Serialize, Default)]
pub struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// Ollama /api/chat 응답 (stream=false)
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: Message,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u32>,
}

/// 툴 호출 요청 (AI 응답에서 파싱)
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub args: Vec<String>,
}

/// 에이전트 대화 결과
#[derive(Debug)]
pub enum AgentResponse {
    /// 일반 텍스트 응답
    Text(String),
    /// 툴 호출 요청
    ToolCall(ToolCall),
    /// 대화 종료 요청
    Exit,
}
