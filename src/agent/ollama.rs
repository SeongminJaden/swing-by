use anyhow::{Context, Result};
use reqwest::Client;
use tracing::{debug, instrument};

use crate::models::{ChatOptions, ChatRequest, ChatResponse, Message};

/// Ollama API 클라이언트
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("HTTP 클라이언트 생성 실패"),
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    /// 환경변수에서 설정을 읽어 클라이언트 생성
    pub fn from_env() -> Self {
        let base_url = std::env::var("OLLAMA_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "gemma4:e4b".to_string());
        Self::new(base_url, model)
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Ollama 서버 헬스체크
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await;
        Ok(resp.is_ok())
    }

    /// 사용 가능한 모델 목록 조회
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Ollama 서버 연결 실패")?;

        let body: serde_json::Value = resp.json().await.context("응답 파싱 실패")?;
        let models = body["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    /// 채팅 완성 요청 (non-streaming)
    #[instrument(skip(self, messages), fields(model = %self.model, msg_count = messages.len()))]
    pub async fn chat(&self, messages: Vec<Message>) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.base_url);

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature: Some(0.7),
                num_predict: Some(2048),
                ..Default::default()
            }),
        };

        debug!("Ollama 요청 전송");

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Ollama API 요청 실패")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API 오류 {}: {}", status, body);
        }

        let chat_resp: ChatResponse = resp.json().await.context("응답 JSON 파싱 실패")?;
        debug!("응답 수신 완료");

        Ok(chat_resp)
    }
}
