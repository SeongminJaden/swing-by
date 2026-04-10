# Rust AI Agent - Ollama + Gemma

Docker에서 실행되는 Ollama와 Gemma 모델을 기반으로 한 다목적 Rust AI 에이전트입니다.

## 기능

- 🤖 **AI 채팅**: Ollama + Gemma를 통한 자연어 처리
- 💻 **코드 분석 및 실행**: Python, JavaScript, Go, Rust 등 다양한 언어 지원
- 📁 **파일 I/O**: 파일 생성, 읽기, 수정
- 🐛 **디버깅**: 코드 디버깅 지원
- 🖥️ **시스템 접근**: 터미널 명령어 실행

## 빠른 시작

### 1단계: 필수 설치

- **Docker Desktop**: https://www.docker.com/products/docker-desktop
- **Rust**: https://www.rust-lang.org/tools/install

### 2단계: Ollama + Gemma 설정

Windows:
```powershell
.\setup-ollama.bat
```

Linux/macOS:
```bash
chmod +x setup-ollama.sh
./setup-ollama.sh
```

또는 수동으로:
```bash
docker-compose up -d
docker exec ai_agent_ollama ollama pull gemma:latest
```

### 3단계: Rust 프로젝트 빌드

```bash
cargo build --release
```

### 4단계: 에이전트 실행

```bash
cargo run
```

## 구조

```
.
├── src/
│   ├── main.rs                 # 진입점
│   ├── agent/                  # Agent 핵심 로직
│   │   ├── mod.rs
│   │   ├── chat.rs            # 채팅 루프
│   │   ├── ollama.rs          # Ollama API
│   │   └── tools.rs           # Tool 관리
│   ├── tools/                  # 각 Tool 구현
│   │   ├── mod.rs
│   │   ├── code_executor.rs
│   │   ├── file_handler.rs
│   │   ├── debugger.rs
│   │   └── system.rs
│   └── models/                 # 데이터 구조
│       ├── mod.rs
│       └── message.rs
├── Cargo.toml                  # 의존성
├── docker-compose.yml          # Ollama 설정
├── setup-ollama.sh            # 설정 스크립트 (Linux/macOS)
├── setup-ollama.bat           # 설정 스크립트 (Windows)
└── README.md                  # 이 파일

```

## Ollama API

- **주소**: `http://localhost:11434`
- **모델**: `gemma:latest`

## 개발 진행 상황

- [ ] Agent 핵심 구조
- [ ] Ollama 클라이언트 구현
- [ ] 채팅 루프
- [ ] Code Executor
- [ ] File Handler
- [ ] System Commands
- [ ] Debugger Interface
- [ ] 통합 테스트

## 환경 변수

```bash
OLLAMA_API_URL=http://localhost:11434  # Ollama API 주소
OLLAMA_MODEL=gemma:latest               # 사용할 모델
LOG_LEVEL=info                          # 로그 레벨
```

## Docker 관리

```bash
# 시작
docker-compose up -d

# 중지
docker-compose down

# 로그 확인
docker-compose logs -f

# 모델 확인
docker exec ai_agent_ollama ollama list
```

## 문제 해결

### 메모리 부족
Gemma 모델 실행에는 최소 8GB 메모리가 필요합니다.
Docker Desktop 설정에서 메모리를 증가시키세요.

### Ollama 연결 불가
```bash
# Ollama 서비스 상태 확인
docker ps | grep ollama

# API 테스트
curl http://localhost:11434/api/tags
```

## 라이선스

MIT

## 참고

- [Ollama 공식](https://ollama.ai)
- [Gemma 모델](https://ai.google.dev/gemma)
- [Rust Documentation](https://doc.rust-lang.org/)
