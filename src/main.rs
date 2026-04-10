mod agent;
mod models;
mod tools;

use anyhow::Result;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // 로깅 초기화 (RUST_LOG 환경변수로 레벨 제어)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .init();

    info!("AI Agent 시작");

    let client = agent::OllamaClient::from_env();

    // Ollama 서버 연결 확인
    print!("Ollama 서버 연결 확인 중... ");
    match client.health_check().await {
        Ok(true) => {
            println!("연결됨");

            // 사용 가능한 모델 목록 출력
            match client.list_models().await {
                Ok(models) if !models.is_empty() => {
                    info!("사용 가능한 모델: {}", models.join(", "));
                }
                Ok(_) => {
                    warn!("설치된 모델 없음. `ollama pull {}` 실행 필요", client.model());
                }
                Err(e) => {
                    warn!("모델 목록 조회 실패: {}", e);
                }
            }
        }
        _ => {
            error!(
                "Ollama 서버에 연결할 수 없습니다.\n\
                 해결 방법:\n\
                 1. Docker: docker-compose up -d\n\
                 2. 또는 Ollama 직접 실행: ollama serve\n\
                 \n\
                 OLLAMA_API_URL 환경변수로 서버 주소 변경 가능 (기본: http://localhost:11434)"
            );
            std::process::exit(1);
        }
    }

    // 채팅 루프 실행
    if let Err(e) = agent::run_chat_loop(&client).await {
        error!("채팅 루프 오류: {}", e);
        return Err(e);
    }

    Ok(())
}
