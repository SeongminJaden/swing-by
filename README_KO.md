# AI Agent — 로컬 LLM 멀티에이전트 시스템

> **Ollama + Gemma** (또는 호환 모델) 기반의 완전 로컬 AI 에이전트. 13개 전문 역할, 병렬 툴 실행, RAG 코드베이스 인덱싱, GitHub PR 자동화, AI-to-AI 통신을 갖춘 완전한 애자일 개발 파이프라인을 제공합니다.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Ollama](https://img.shields.io/badge/ollama-compatible-blue)](https://ollama.ai)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow)](LICENSE)
[![](https://img.shields.io/badge/톡코딩에게_후원하기-☕-yellow?style=for-the-badge)](https://buymeacoffee.com/tok_coding)

> 이 프로젝트가 도움이 됐다면 커피 한 잔 후원해 주세요! ☕  
> 여러분의 후원이 프로젝트를 지속하고 새로운 기능 개발에 큰 힘이 됩니다.

---

## 기능 소개

### 핵심 에이전트
- **스트리밍 채팅** — 실시간 토큰 단위 출력
- **멀티 턴 툴 사용** — 파일 읽기/쓰기, 쉘 명령 실행, 웹 검색, 코드 실행
- **세션 유지** — 채팅 히스토리 자동 저장/복원 (`--resume`)
- **컨텍스트 압축** — 토큰 한계 도달 시 자동 히스토리 압축
- **CLAUDE.md 지원** — 프로젝트 및 전역 지시사항 자동 로딩

### 애자일 멀티에이전트 파이프라인
13개 AI 역할이 협력하여 전체 소프트웨어 개발 생명주기를 처리:

| 역할 | 아이콘 | 담당 |
|------|--------|------|
| ProductOwner | 📦 | 요구사항 분석 → 유저 스토리 작성 |
| ScrumMaster | 🏃 | 스프린트 계획 및 장애물 제거 |
| BusinessAnalyst | 📊 | 이해관계자 분석, ROI, KPI 정의 |
| UXDesigner | 🎨 | 페르소나, 와이어프레임, 접근성 |
| Architect | 🏛️ | 시스템 설계, 기술 스택, SOLID 원칙 |
| Developer | 💻 | TDD 구현, OWASP 보안 준수 |
| QAEngineer | 🔬 | 테스트 케이스, 버그 리포트, 회귀 테스트 |
| Reviewer | 👁️ | 코드 리뷰, 보안, 성능 점수 |
| TechLead | 🎯 | 게이트 리뷰, ADR, 릴리즈 승인 |
| DevOpsEngineer | 🚀 | CI/CD, Docker, Kubernetes, IaC |
| TechnicalWriter | 📝 | README, API 문서, 체인지로그 |
| SRE | 📡 | SLO/SLI, 런북, 카오스 엔지니어링 |
| ReleaseManager | 🎁 | 릴리즈 노트, SemVer, 롤백 계획 |

**파이프라인 흐름:**
```
ProductOwner → ScrumMaster → Architect → Developer → QAEngineer
→ Reviewer → TechLead → DevOpsEngineer → TechnicalWriter
→ SRE → ReleaseManager
```

### Coordinator 멀티에이전트 모드
Claude Code의 병렬 실행 아키텍처에서 영감:
- 리더 에이전트가 복잡한 태스크를 독립적 서브태스크로 분해
- 워커 에이전트들이 `tokio::spawn`으로 **병렬 실행**
- `mpsc` 채널을 통한 실시간 진행 상황 스트리밍
- 리더가 결과를 종합하여 일관성 있는 최종 출력 생성
- 최대 8개 병렬 워커

### 병렬 툴 실행
읽기 전용 툴은 동시 실행 (Claude Code 아키텍처 참조):
```
read_file, list_dir, glob, grep, git_*, web_search, docker_*, ...
```
쓰기/부작용 툴은 안전을 위해 항상 순차 실행.

### RAG 코드베이스 인덱싱
- 프로젝트 디렉토리 탐색, 파일을 800자 청크로 분할
- TF-IDF 키워드 스코어링 (외부 벡터 DB 불필요)
- `.rag_index.json`에 인덱스 영구 저장
- 관련 코드 컨텍스트를 질의에 자동 주입

### GitHub PR 자동화
- 현재 브랜치와 기본 브랜치 자동 감지
- AI가 PR 제목, 설명, 테스트 계획 생성
- 스프린트 인식: 릴리즈 노트에서 PR 자동 생성
- 드래프트 PR, 레이블, 커스텀 베이스 브랜치 지원

### 전문 파이프라인
| 명령어 | 파이프라인 |
|--------|-----------|
| `/agile <작업>` | 11개 역할 전체 스프린트 파이프라인 |
| `/retro` | 스프린트 회고 (KPT 형식) |
| `/postmortem <장애>` | 장애 분석 → 수정 → 런북 |
| `/techdebt` | 기술 부채 분석 및 우선순위화 |
| `/security <대상>` | 보안 감사 (방어적) |
| `/coordinator <작업>` | 병렬 멀티에이전트 분해 |
| `/ba <주제>` | 비즈니스 분석 단독 실행 |
| `/ux <주제>` | UX/UI 디자인 단독 실행 |
| `/devops <주제>` | DevOps & 인프라 단독 실행 |
| `/docs <주제>` | 문서화 단독 실행 |
| `/sre <주제>` | SRE 분석 단독 실행 |

### 스프린트 체크포인트
- 각 스토리 완료 후 `.sprint_checkpoint.json`에 진행 상황 자동 저장
- `/agile checkpoint resume`으로 중단된 스프린트 재개
- `sprint-report-{id}-{timestamp}.md`에 스프린트 보고서 자동 저장

### 역할별 모델 지정
`config.toml`에서 역할마다 다른 Ollama 모델 설정:
```toml
[roles]
architect = "llama3.2:latest"
developer = "codestral:latest"
qa_engineer = "gemma4:e4b"
```

### Discord 봇 모드
`--discord` 플래그로 Discord를 통한 전체 에이전트 기능 제공.

### AI-to-AI 통신
- **stdio 모드** (`--ipc-stdio`): stdin/stdout을 통한 JSON-RPC 2.0
- **HTTP 서버 모드** (`--ipc-server <포트>`): 다른 에이전트를 위한 REST 엔드포인트

---

## 설치

### 한 줄 설치

**Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/SeongminJaden/swing-by/main/install.sh | bash
```

**macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/SeongminJaden/swing-by/main/install_mac.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/SeongminJaden/swing-by/main/install.ps1 | iex
```

인스톨러가 터미널에서 대화형으로 안내합니다:
1. 선택적 툴 설치 (git, Docker, Python, Node.js) — yes/no 선택
2. Ollama 설치
3. AI 모델 메뉴에서 선택
4. 사전 빌드된 바이너리 다운로드
5. 환경 변수 자동 설정

### 모델 선택 메뉴
```
  Gemma 4 (추천)
   1) gemma4:e4b       — 8B  Q4, 가장 빠름, ~5GB  [추천]
   2) gemma4:12b       — 12B Q4, 더 좋은 품질, ~7GB
   3) gemma4:27b       — 27B Q4, 최고 품질,   ~16GB

  대안 모델
   4) llama3.2:latest  — Meta Llama 3.2 3B, ~2GB
   5) llama3.1:latest  — Meta Llama 3.1 8B, ~5GB
   6) codestral:latest — 코드 특화, ~12GB
   7) qwen2.5:7b       — 다국어, ~5GB
   8) 직접 모델명 입력
```

### 수동 설치
```bash
# 릴리즈 페이지에서 직접 바이너리 다운로드
# https://github.com/USER/ai-agent/releases/latest

chmod +x ai_agent-linux-x86_64
sudo mv ai_agent-linux-x86_64 /usr/local/bin/ai_agent
```

### 환경 변수
```bash
OLLAMA_API_URL=http://localhost:11434   # Ollama 서버 (기본값)
OLLAMA_MODEL=gemma4:e4b                 # 사용 모델 (기본값)
DISCORD_TOKEN=...                       # --discord 모드 필수
```

---

## 사용법

### 대화형 모드 (채팅)
```bash
cargo run
```

### 슬래시 명령어 (채팅 중)
```
/help                        모든 명령어 표시
/agile <작업>                전체 애자일 스프린트 파이프라인 실행
/agile checkpoint resume     중단된 스프린트 재개
/retro                       스프린트 회고 실행
/postmortem <장애 설명>      장애 포스트모템 파이프라인
/techdebt                    기술 부채 분석
/security <대상>             보안 감사
/coordinator <작업>          병렬 멀티에이전트 실행
/ba <주제>                   비즈니스 분석
/ux <주제>                   UX/UI 디자인
/devops <주제>               DevOps & 인프라
/docs <주제>                 문서화 생성
/sre <주제>                  SRE 분석
/rag index                   현재 코드베이스 RAG 인덱싱
/rag query <질문>            인덱싱된 코드베이스 질의
/rag status                  RAG 인덱스 통계 표시
/pr list                     열린 GitHub PR 목록
/pr create [제목]            AI 생성 설명으로 PR 생성
/save                        현재 세션 저장
/load                        이전 세션 불러오기
/memory list                 저장된 메모리 표시
/memory add <메모>           영구 메모리 추가
/compact                     수동 컨텍스트 압축
/config                      현재 설정 표시
/clear                       채팅 히스토리 초기화
/exit                        종료
```

### 비대화형 모드
```bash
# 단일 프롬프트 실행 후 종료
cargo run -- --print "이 코드베이스를 설명해줘"

# 전체 애자일 스프린트
cargo run -- --agile "OAuth2 로그인 추가" --project webapp

# 파이프라인 모드 (계획→구현→디버깅→리뷰)
cargo run -- --pipeline "레이트 리미터 구현"

# 이전 세션 재개
cargo run -- --resume

# 특정 모델 사용
cargo run -- --model llama3.2:latest
```

### AI-to-AI 통신
```bash
# stdio JSON-RPC (Claude Code 등 다른 에이전트에서 호출)
echo '{"jsonrpc":"2.0","id":1,"method":"chat","params":{"prompt":"hello"}}' | ./ai_agent --ipc-stdio

# 외부 에이전트를 위한 HTTP 서버
./ai_agent --ipc-server 8765
curl -X POST http://localhost:8765 \
  -H 'Content-Type: application/json' \
  -H 'X-Caller-ID: claude-code' \
  -d '{"jsonrpc":"2.0","id":1,"method":"agile_sprint","params":{"project":"myapp","request":"로그인 추가"}}'
```

---

## 프로젝트 구조

```
src/
├── main.rs                  # 진입점, CLI 인자 파싱
├── config.rs                # AppConfig + 역할별 모델 오버라이드
├── utils.rs                 # 공유 유틸리티 (trunc 등)
├── agent/
│   ├── mod.rs               # 공개 API (run_chat_loop 등)
│   ├── chat.rs              # 채팅 루프, 슬래시 명령어, 세션 관리
│   ├── ollama.rs            # Ollama 스트리밍 클라이언트
│   ├── tools.rs             # 툴 디스패처 (50개 이상)
│   ├── node.rs              # 에이전트 메시징용 NodeHub
│   ├── orchestrator.rs      # 파이프라인 오케스트레이터
│   ├── react.rs             # ReAct 추론 루프
│   ├── sub_agent.rs         # 서브에이전트 생성
│   ├── rag.rs               # RAG 코드베이스 인덱싱 (TF-IDF)
│   └── github.rs            # GitHub PR 자동화 (gh CLI)
├── agile/
│   ├── mod.rs               # 공개 애자일 API
│   ├── team.rs              # 13개 AgileRole 정의
│   ├── story.rs             # UserStory, Sprint 데이터 구조
│   ├── board.rs             # AgileBoard (칸반 상태 머신)
│   ├── pipeline.rs          # 전체 스프린트 파이프라인 + 체크포인팅
│   ├── runner.rs            # 공유 에이전트 러너 (병렬 툴)
│   ├── coordinator.rs       # Coordinator 멀티에이전트 모드
│   ├── retrospective.rs     # 스프린트 회고 (KPT)
│   ├── postmortem.rs        # 장애 포스트모템 파이프라인
│   ├── techdebt.rs          # 기술 부채 분석
│   ├── security.rs          # 보안 감사 파이프라인
│   └── hacker.rs            # 해커 모드 (CTF/연구)
├── discord/                 # Discord 봇 통합
├── ipc/                     # AI-to-AI JSON-RPC 통신
├── mcp/                     # Model Context Protocol 지원
├── skills/                  # 플러거블 스킬 시스템
├── monitor.rs               # 시스템 모니터링
└── history.rs               # 세션 히스토리 관리
```

---

## 설정

### `config.toml` (첫 실행 시 자동 생성)
```toml
[agent]
model = "gemma4:e4b"
api_url = "http://localhost:11434"
max_tokens = 4096
temperature = 0.7

[agile]
max_sprint_stories = 5
sprint_report_dir = "."
checkpoint_enabled = true

[roles]
# 역할별 특정 모델 지정 (선택사항)
# architect = "llama3.2:latest"
# developer = "codestral:latest"
# qa_engineer = "gemma4:e4b"
```

---

## 아키텍처 하이라이트

### 컨텍스트 압축
히스토리가 60개 메시지를 초과하면 자동 압축 — 시스템 메시지는 보존되고, 가장 오래된 비시스템 메시지는 삭제되며, 요약 플레이스홀더가 삽입됩니다. Claude Code의 컨텍스트 관리 방식을 참조했습니다.

### 병렬 툴 실행
에이전트가 여러 읽기 전용 툴로 `__multi__`를 호출하면 `tokio::spawn`으로 동시 실행합니다. 쓰기 툴은 항상 순차 실행하여 레이스 컨디션을 방지합니다.

### Coordinator 패턴
```
Coordinator
├── 태스크 분해 → SubTask[]
├── 워커 1 (tokio::spawn) → 결과
├── 워커 2 (tokio::spawn) → 결과
│   ...
└── 종합 → 최종 출력
```
모든 워커가 공유 콜백에 안전하게 보고할 수 있도록 `mpsc::unbounded_channel`로 진행 메시지를 수집합니다.

---

## 사용 가능한 툴 (50개 이상)

**파일 시스템:** `read_file`, `write_file`, `edit_file`, `list_dir`, `glob`, `grep`, `delete_file`, `move_file`, `copy_file`

**코드 실행:** `run_code` (Python/JS/Go/Rust), `run_shell`, `run_tests`

**Git:** `git_status`, `git_diff`, `git_log`, `git_blame`, `git_commit`, `git_push`, `git_branch`, `git_checkout`, `git_stash`, `git_root`, `git_changed_files`

**웹:** `web_fetch`, `web_search`, `research`, `docs_fetch`

**패키지:** `pkg_info`, `pkg_versions`, `pkg_search`, `pkg_list`

**Docker:** `docker_ps`, `docker_images`, `docker_stats`, `docker_exec`, `docker_logs`, `docker_build`, `docker_run`

**시스템:** `sysinfo`, `process_list`, `env_list`, `get_env`, `set_env`, `current_dir`

**데이터베이스:** `db_query`, `db_schema`

**Todo:** `todo_read`, `todo_write`

---

## 테스트

```bash
# 전체 테스트 실행
cargo test

# 특정 모듈 테스트
cargo test agile::
cargo test agent::rag
cargo test agile::coordinator
```

현재 테스트 커버리지: 전체 모듈에 걸쳐 **137개 테스트** 통과.

---

## 라이선스

MIT — [LICENSE](LICENSE) 참조

---

## 참고 자료

- [Ollama](https://ollama.ai) — 로컬 LLM 추론
- [Google Gemma](https://ai.google.dev/gemma) — 기본 모델
- [Claude Code 아키텍처](https://docs.anthropic.com/claude-code) — 병렬 실행 & Coordinator 패턴 참조
- [Google SRE Book](https://sre.google/books/) — SRE 파이프라인 참조
