# 🚀 시작하기 - Rust AI Agent with Ollama + Gemma

## ⚡ 5분 안에 시작하기

### Windows
```powershell
.\quick-start.bat
```

### Linux / macOS
```bash
chmod +x quick-start.sh
./quick-start.sh
```

이것만 실행하면 전체 환경이 자동으로 설정됩니다! ✨

---

## 📋 전체 설정 (수동)

### 1️⃣ Docker + Ollama 설정

```bash
# 컨테이너 시작
docker-compose up -d

# Ollama 준비 대기
docker logs -f ai_agent_ollama

# 모델 다운로드 (다른 터미널)
docker exec ai_agent_ollama ollama pull gemma:latest
```

**소요 시간**: 10-30분 (인터넷 속도에 따라 다름)

### 2️⃣ 설치 확인

```bash
# 설치된 모델 확인
docker exec ai_agent_ollama ollama list

# API 테스트
curl http://localhost:11434/api/tags

# Ollama 채팅 테스트
curl -X POST http://localhost:11434/api/chat \
  -d '{
    "model": "gemma:latest",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false
  }'
```

### 3️⃣ Rust 프로젝트 빌드

```bash
# 의존성 다운로드 및 빌드
cargo build

# 또는 최적화 빌드
cargo build --release

# 실행
cargo run
```

---

## 📁 프로젝트 구조

```
C:\git\
├── 🐳 Docker 설정
│   ├── docker-compose.yml      # Ollama 컨테이너
│   ├── setup-ollama.bat        # Windows 설정 스크립트
│   └── setup-ollama.sh         # Linux/macOS 설정 스크립트
│
├── 🦀 Rust 프로젝트
│   ├── Cargo.toml              # 의존성 정의
│   ├── main.rs                 # 진입점 (임시)
│   └── src/                    # 소스 코드 (개발 예정)
│
├── 📚 문서
│   ├── README.md               # 프로젝트 개요
│   ├── OLLAMA_SETUP.md         # 상세 Ollama 가이드
│   ├── PROJECT_STRUCTURE.md    # 프로젝트 구조
│   ├── SETUP_CHECKLIST.md      # 설정 체크리스트
│   ├── QUICK_START.md          # 이 파일
│   ├── quick-start.sh          # 자동 설정 스크립트 (Linux/macOS)
│   └── quick-start.bat         # 자동 설정 스크립트 (Windows)
│
├── 🔧 기타
├── .gitignore                  # Git 무시 파일
└── .git/                       # Git 저장소
```

---

## 🎯 개발 단계

### Phase 1: 기초 구축 ✅
- ✅ Docker + Ollama 설정
- ✅ Rust 프로젝트 구조
- ✅ Cargo.toml 의존성 정의

### Phase 2: Agent 핵심 (다음)
- [ ] Ollama API 클라이언트 (`src/agent/ollama.rs`)
- [ ] 메시지 모델 (`src/models.rs`)
- [ ] 채팅 루프 (`src/agent/chat.rs`)

### Phase 3: Tool 구현
- [ ] Code Executor (`src/tools/code_executor.rs`)
- [ ] File Handler (`src/tools/file_handler.rs`)
- [ ] System Commands (`src/tools/system.rs`)
- [ ] Debugger (`src/tools/debugger.rs`)

### Phase 4: 통합 및 테스트
- [ ] 모든 Tool 통합
- [ ] 단위 테스트
- [ ] 통합 테스트

---

## 🔗 유용한 링크

| 주제 | 링크 |
|------|------|
| Docker | https://www.docker.com/products/docker-desktop |
| Rust | https://www.rust-lang.org/tools/install |
| Ollama | https://ollama.ai |
| Gemma | https://ai.google.dev/gemma |
| Reqwest | https://docs.rs/reqwest |
| Tokio | https://tokio.rs |

---

## 🆘 문제 해결

### "Docker 연결 실패"
```powershell
# Windows: Docker Desktop 재시작
# 또는
wsl.exe --shutdown
```

### "포트 11434 이미 사용 중"
```yaml
# docker-compose.yml에서 변경:
ports:
  - "11435:11434"  # 외부 11435로 변경
```

### "Ollama 서비스 시작 안 됨"
```bash
# 로그 확인
docker logs -f ai_agent_ollama

# 컨테이너 재시작
docker-compose down
docker-compose up -d
```

### "Cargo 빌드 실패"
```bash
# 캐시 정리
cargo clean
cargo build
```

---

## 💻 다음 명령어

```bash
# 🚀 에이전트 실행
cargo run

# 📊 최적화 빌드 및 실행
cargo run --release

# 🧪 테스트
cargo test

# 📋 코드 포맷
cargo fmt

# 🔍 린트
cargo clippy

# 📖 문서 생성
cargo doc --open

# 🧹 정리
cargo clean
```

---

## 🎓 학습 리소스

- **Rust 기초**: https://doc.rust-lang.org/book/
- **비동기 프로그래밍**: https://tokio.rs/tokio/tutorial
- **HTTP 클라이언트**: https://docs.rs/reqwest
- **AI 모델링**: https://ai.google.dev/gemma

---

## 📝 라이선스

MIT

---

**모든 준비가 되었습니다! `quick-start.bat` 또는 `quick-start.sh`를 실행하세요!** 🎉
