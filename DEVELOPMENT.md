# 개발 도움말

## 🏗️ 프로젝트 구성 방법

### 방법 1: 현재 폴더 (추천) ✅
현재 `C:\git`에서 직접 개발합니다.

```bash
# Cargo.toml이 이미 준비되어 있음
cargo build
cargo run
```

### 방법 2: 별도 폴더 (선택)
```bash
# C:\git에 별도 폴더 생성
cargo new my_agent
cd my_agent
cargo build
```

---

## 📂 소스 코드 추가하기

현재 `main.rs`는 임시 파일입니다. 실제 개발을 시작하려면:

### Step 1: src/ 폴더 생성
```bash
mkdir src
```

### Step 2: 파일 이동
```bash
move main.rs src\main.rs
```

또는 새로 생성:
```bash
# src/main.rs - 진입점
# src/models.rs - 데이터 구조
# src/agent/mod.rs - Agent 모듈
# src/tools/mod.rs - Tool 모듈
```

### Step 3: 빌드 및 실행
```bash
cargo build
cargo run
```

---

## 🔧 유용한 Cargo 명령어

```bash
# 프로젝트 초기화 (이미 완료)
cargo init --name ai_agent

# 빌드
cargo build                    # 디버그 빌드
cargo build --release         # 최적화 빌드

# 실행
cargo run                      # 직접 실행
cargo run --release          # 최적화된 버전으로 실행

# 테스트
cargo test                    # 모든 테스트 실행
cargo test -- --nocapture   # 출력 표시

# 코드 품질
cargo fmt                     # 코드 포맷
cargo clippy                  # 린트 (경고 확인)
cargo clippy --fix           # 자동 수정

# 문서
cargo doc --open             # 문서 생성 및 열기
cargo doc --no-deps          # 의존성 문서 제외

# 정보
cargo tree                    # 의존성 트리 출력
cargo metadata                # 메타데이터 출력

# 정리
cargo clean                   # 빌드 결과 정리
```

---

## 📦 의존성 추가

현재 `Cargo.toml`에 포함된 의존성:

```toml
tokio              # 비동기 런타임
reqwest            # HTTP 클라이언트
serde & serde_json # JSON 직렬화
tracing            # 로깅
anyhow & thiserror # 에러 처리
shlex              # 명령어 파싱
tempfile           # 임시 파일
```

새로운 의존성 추가:
```bash
cargo add <package_name>
# 예: cargo add rand
```

또는 수동으로 `Cargo.toml`에 추가:
```toml
[dependencies]
rand = "0.8"
```

---

## 🐛 디버깅

### 상세 로그 출력
```bash
RUST_LOG=debug cargo run
RUST_LOG=trace cargo run     # 더 상세
```

### 디버거 (GDB/LLDB)
```bash
# 디버그 빌드로 컴파일
cargo build

# GDB로 실행 (Linux)
gdb target/debug/ai_agent

# LLDB로 실행 (macOS)
lldb target/debug/ai_agent
```

---

## 📊 프로젝트 구조 (예상)

```
C:\git\
├── src/
│   ├── main.rs                   # 진입점
│   ├── models/
│   │   ├── mod.rs              # 모듈 정의
│   │   └── message.rs          # 메시지 타입
│   ├── agent/
│   │   ├── mod.rs
│   │   ├── ollama.rs           # Ollama 클라이언트
│   │   ├── chat.rs             # 채팅 루프
│   │   └── tools.rs            # Tool 관리
│   └── tools/
│       ├── mod.rs
│       ├── code_executor.rs
│       ├── file_handler.rs
│       ├── system.rs
│       └── debugger.rs
├── target/                       # 빌드 결과 (자동 생성)
│   ├── debug/
│   └── release/
├── Cargo.toml                    # 의존성 정의
├── Cargo.lock                    # 의존성 잠금 (자동 생성)
└── README.md                     # 프로젝트 설명
```

---

## ✅ 다음 단계

1. **`quick-start.bat`** (또는 `.sh`) 실행 → Docker + Ollama 설정
2. **`cargo build`** 실행 → 의존성 다운로드 및 빌드
3. **`src/` 폴더** 구성 → 실제 코드 작성 시작
4. **`cargo run`** → 에이전트 실행

---

## 🎯 개발 흐름

```
1. Ollama 시작 (quick-start.bat)
2. Rust 빌드 (cargo build)
3. 코드 작성 (src/*.rs)
4. 테스트 (cargo test)
5. 실행 (cargo run)
6. 배포 (cargo build --release)
```

---

**모든 준비가 완료되었습니다! 행운을 빕니다! 🚀**
