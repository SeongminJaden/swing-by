use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, instrument, warn};

use crate::models::{ChatOptions, ChatRequest, ChatResponse, Message};

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_MS: u64 = 1000; // 지수 백오프 기본값

/// Streaming 청크 (stream=true 응답)
#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub message: Message,
    pub done: bool,
}

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
                .timeout(std::time::Duration::from_secs(600)) // 10분 (긴 응답 허용)
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
        Ok(self.client.get(&url).send().await.is_ok())
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

    /// 채팅 완성 요청 (non-streaming, 재시도 포함)
    #[instrument(skip(self, messages), fields(model = %self.model, msg_count = messages.len()))]
    pub async fn chat(&self, messages: Vec<Message>) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature: Some(0.7),
                num_predict: Some(4096),
                ..Default::default()
            }),
        };

        let mut last_err = anyhow::anyhow!("재시도 한도 초과");
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = RETRY_BASE_MS * (1 << attempt);
                warn!("Ollama 재시도 {}/{} ({}ms 후)", attempt, MAX_RETRIES, delay);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }

            debug!("Ollama 요청 전송 (시도 {})", attempt + 1);
            let resp = match self.client.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => { last_err = anyhow::anyhow!("연결 실패: {}", e); continue; }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                // 5xx 오류는 재시도, 4xx는 즉시 실패
                if status.is_server_error() {
                    last_err = anyhow::anyhow!("Ollama 서버 오류 {}: {}", status, body);
                    continue;
                }
                anyhow::bail!("Ollama API 오류 {}: {}", status, body);
            }

            let chat_resp: ChatResponse = resp.json().await.context("응답 JSON 파싱 실패")?;
            debug!("응답 수신 완료");
            return Ok(chat_resp);
        }
        Err(last_err)
    }

    /// 단일 턴 non-streaming 요청 (Helpers)
    #[allow(dead_code)]
    pub async fn chat_simple(&self, prompt: &str) -> Result<String> {
        let msgs = vec![Message::user(prompt)];
        let resp = self.chat(msgs).await?;
        Ok(resp.message.content)
    }

    /// 스트리밍 채팅 요청 — 토큰이 도착할 때마다 콜백 호출 (재시도 포함)
    /// 반환값: 전체 응답 텍스트
    pub async fn chat_stream<F>(
        &self,
        messages: Vec<Message>,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            options: Some(ChatOptions {
                temperature: Some(0.7),
                num_predict: Some(4096),
                ..Default::default()
            }),
        };

        let mut last_err = anyhow::anyhow!("재시도 한도 초과");
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = RETRY_BASE_MS * (1 << attempt);
                warn!("스트리밍 재시도 {}/{} ({}ms 후)", attempt, MAX_RETRIES, delay);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }

            let resp = match self.client.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => { last_err = anyhow::anyhow!("연결 실패: {}", e); continue; }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if status.is_server_error() {
                    last_err = anyhow::anyhow!("서버 오류 {}: {}", status, body);
                    continue;
                }
                anyhow::bail!("Ollama API 오류 {}: {}", status, body);
            }

            let mut stream = resp.bytes_stream();
            let mut full_content = String::new();
            let mut line_buf = String::new();

            while let Some(item) = stream.next().await {
                let bytes = item.context("스트림 읽기 오류")?;
                line_buf.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(nl) = line_buf.find('\n') {
                    let line = line_buf[..nl].trim().to_string();
                    line_buf = line_buf[nl + 1..].to_string();

                    if line.is_empty() { continue; }

                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(&line) {
                        let tok = &chunk.message.content;
                        if !tok.is_empty() {
                            on_token(tok);
                            full_content.push_str(tok);
                        }
                        if chunk.done { break; }
                    }
                }
            }

            return Ok(full_content);
        }
        Err(last_err)
    }
}
