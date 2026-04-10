# Rust 프로젝트 디버깅 가이드

## 🐛 로컬 디버깅

### 1. GDB 디버깅 (Linux)

```bash
# 디버그 빌드
cargo build

# GDB 시작
gdb target/debug/ai_agent

# GDB 명령어
(gdb) break main              # 중단점 설정
(gdb) run                     # 실행
(gdb) step                    # 한 줄 실행
(gdb) next                    # 다음 줄
(gdb) continue                # 계속 실행
(gdb) print variable_name     # 변수 출력
(gdb) backtrace               # 스택 추적
(gdb) quit                    # 종료
```

### 2. LLDB 디버깅 (macOS)

```bash
# 디버그 빌드
cargo build

# LLDB 시작
lldb target/debug/ai_agent

# LLDB 명령어 (GDB와 유사)
(lldb) breakpoint set --name main
(lldb) run
(lldb) step
(lldb) frame variable
```

### 3. Rust 통합 디버거 (Cargo)

```bash
# 상세 로그 출력
RUST_LOG=debug cargo run
RUST_LOG=trace cargo run      # 더 상세한 로그

# 백트레이스 활성화
RUST_BACKTRACE=1 cargo run
RUST_BACKTRACE=full cargo run # 전체 백트레이스
```

---

## 🔍 VSCode 디버깅

### 1. 확장 설치
- **CodeLLDB** (vadimcn.vscode-lldb)
- **Rust-analyzer** (rust-lang.rust-analyzer)

### 2. `.vscode/launch.json` 설정

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug AI Agent",
      "cargo": {
        "args": [
          "build",
          "--bin=ai_agent",
          "--package=ai_agent"
        ],
        "filter": {
          "name": "ai_agent",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "sourceLanguages": ["rust"]
    }
  ]
}
```

### 3. 디버깅 실행
- **F5** 키 또는 **Run > Start Debugging**
- 중단점 설정 후 실행
- 변수 검사 (왼쪽 "Variables" 탭)

---

## 🧪 로깅 & 추적

### 1. Tracing 설정 (코드)

```rust
use tracing::{info, debug, error};

#[tokio::main]
async fn main() {
    // 로그 레벨 설정
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    info!("애플리케이션 시작");
    debug!("디버그 정보");
    error!("에러 발생!");
}
```

### 2. 환경 변수로 로그 제어

```bash
# 전체 DEBUG 레벨
RUST_LOG=debug cargo run

# 특정 모듈만
RUST_LOG=ai_agent=debug cargo run

# 여러 모듈
RUST_LOG=ai_agent=debug,tokio=info cargo run

# 타겟별 필터
RUST_LOG=debug,hyper=info cargo run
```

---

## 🧩 단위 테스트

### 테스트 작성

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_async_function() {
        // 비동기 테스트
    }
}
```

### 테스트 실행

```bash
# 모든 테스트
cargo test

# 특정 테스트
cargo test test_name

# 멀티 스레드 비활성화
cargo test -- --test-threads=1

# 출력 표시
cargo test -- --nocapture

# 무시된 테스트 실행
cargo test -- --ignored

# 모든 테스트 (무시된 것 포함)
cargo test -- --include-ignored
```

---

## 📊 성능 프로파일링

### Flamegraph

```bash
# 설치
cargo install flamegraph

# 프로파일링 실행
cargo flamegraph

# 결과 (flamegraph.svg) 열기
```

### Perf (Linux)

```bash
# 릴리스 빌드
cargo build --release

# 프로파일링
perf record ./target/release/ai_agent

# 결과 분석
perf report
```

---

## 🔧 문제 디버깅 팁

### 1. Panic 백트레이스
```bash
# 상세 백트레이스 출력
RUST_BACKTRACE=1 cargo run
RUST_BACKTRACE=full cargo run  # 전체 정보
```

### 2. 메모리 누수 탐지
```bash
# Valgrind (Linux)
valgrind --leak-check=full ./target/debug/ai_agent
```

### 3. 데드락 탐지
```bash
# 타이밍 이슈 재현 (반복 실행)
for i in {1..100}; do cargo test || break; done
```

### 4. 컴파일 타임 문제
```bash
# 컴파일 타임 측정
cargo build --timings

# Incremental 빌드 비활성화
CARGO_INCREMENTAL=0 cargo build
```

---

## 📱 원격 디버깅

### Docker 컨테이너 디버깅

```bash
# 컨테이너 내부에서 디버거 실행
docker exec -it ai_agent_ollama bash

# 또는 코드에서 디버그 로그 추가
```

### 로그 원격 전송

```rust
// 실시간 로그 모니터링
docker logs -f ai_agent_ollama
```

---

## ✅ 디버깅 체크리스트

- [ ] RUST_LOG 환경 변수 설정
- [ ] Tracing 초기화 확인
- [ ] 중요 함수에 디버그 로그 추가
- [ ] 테스트 케이스 작성
- [ ] 순환 참조 확인
- [ ] 메모리 누수 확인
- [ ] 성능 프로파일링 수행

---

## 🎯 권장 디버깅 전략

1. **로그 추가** → RUST_LOG로 문제 위치 파악
2. **단위 테스트** → 작은 단위부터 검증
3. **단계적 실행** → GDB/LLDB로 단계 추적
4. **백트레이스** → Panic 발생 시 RUST_BACKTRACE=full
5. **프로파일링** → 성능 병목 찾기

---

**Happy Debugging! 🐛✨**
