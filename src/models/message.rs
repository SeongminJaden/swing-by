use serde::{Deserialize, Serialize};

/// Ollama API message role
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

/// Chat message
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

/// Ollama /api/chat request body
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ChatOptions>,
}

/// Model parameter options
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

/// Ollama /api/chat response (stream=false)
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: Message,
    #[allow(dead_code)]
    pub done: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub eval_count: Option<u32>,
}

/// Tool call request (parsed from AI response)
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub args: Vec<String>,
}

/// Agent conversation result
#[derive(Debug)]
pub enum AgentResponse {
    /// Plain text response
    #[allow(dead_code)]
    Text(String),
    /// Tool call request
    ToolCall(ToolCall),
    /// Request to end the conversation
    Exit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_roles_serialize_lowercase() {
        let sys = Message::system("hello");
        assert_eq!(sys.role, Role::System);
        let j = serde_json::to_string(&sys).unwrap();
        assert!(j.contains("\"system\""), "role must serialize as 'system'");
    }

    #[test]
    fn message_constructors_set_content() {
        assert_eq!(Message::user("hi").content, "hi");
        assert_eq!(Message::assistant("ok").content, "ok");
        assert_eq!(Message::tool("result").content, "result");
        assert_eq!(Message::system("sys").content, "sys");
    }

    #[test]
    fn chat_request_serializes_stream_false() {
        let req = ChatRequest {
            model: "test".into(),
            messages: vec![Message::user("ping")],
            stream: false,
            options: None,
        };
        let j = serde_json::to_string(&req).unwrap();
        assert!(j.contains("\"stream\":false"));
        assert!(j.contains("\"model\":\"test\""));
    }

    #[test]
    fn chat_options_skips_none_fields() {
        let opts = ChatOptions { temperature: Some(0.5), ..Default::default() };
        let j = serde_json::to_string(&opts).unwrap();
        assert!(j.contains("\"temperature\":0.5"));
        assert!(!j.contains("num_predict"), "None fields must be skipped");
    }

    #[test]
    fn json_rpc_response_deserialized_from_ollama_format() {
        let json = r#"{"message":{"role":"assistant","content":"hello"},"done":true}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.message.content, "hello");
        assert!(resp.done);
    }
}
