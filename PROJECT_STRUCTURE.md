# Rust AI Agent 프로젝트 구조

프로젝트 배치:
```
ai_agent/                    # Rust 프로젝트
├── src/
│   ├── main.rs             # 메인 엔트리 포인트
│   ├── agent/              # Agent 핵심 로직
│   │   ├── mod.rs
│   │   ├── chat.rs         # 채팅 루프
│   │   ├── ollama.rs       # Ollama API 클라이언트
│   │   └── tools.rs        # Tool 실행 레이어
│   ├── tools/              # 각 Tool의 구현
│   │   ├── mod.rs
│   │   ├── code_executor.rs   # 코드 실행
│   │   ├── file_handler.rs    # 파일 I/O
│   │   ├── debugger.rs        # 디버깅
│   │   └── system.rs          # 시스템 명령
│   └── models/             # 데이터 구조
│       ├── mod.rs
│       └── message.rs      # 메시지 타입
├── Cargo.toml              # 의존성 정의
├── Cargo.lock              # 의존성 잠금
└── README.md               # 프로젝트 설명

docker/                      # Docker 설정
├── docker-compose.yml      # Ollama 컨테이너 정의
└── Dockerfile              # (선택) Agent 컨테이너

scripts/                     # 유틸리티 스크립트
├── setup-ollama.sh         # 설정 자동화 (Linux/Mac)
├── setup-ollama.bat        # 설정 자동화 (Windows)
└── test-api.sh             # Ollama API 테스트
```

## 다음 단계

1. Docker와 Ollama 시작: `setup-ollama.bat` 또는 `setup-ollama.sh` 실행
2. Rust 프로젝트 생성: `cargo new ai_agent`
3. 의존성 추가: Cargo.toml 수정
4. Agent 구현 시작

## Cargo.toml 기본 의존성

```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"

# 코드 실행 관련
shlex = "1.1"          # 명령어 파싱
tempfile = "3.8"       # 임시 파일
```
