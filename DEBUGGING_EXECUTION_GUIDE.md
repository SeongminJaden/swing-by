# 🐛 디버깅 시뮬레이션 & 테스트 가이드

## 📋 실행 & 디버깅 단계별 가이드

### Phase 1: 프로젝트 설정 확인 ✅

#### 1.1 파일 구조 검증
```bash
# Windows CMD
cd C:\git
dir /B

# 확인할 파일들:
# ✓ Cargo.toml - Rust 설정
# ✓ main.rs - 진입점
# ✓ docker-compose.yml - Ollama 설정
# ✓ init_git.bat - Git 초기화
# ✓ quick-start.bat - 환경 설정
```

#### 1.2 Git 저장소 확인
```bash
git status          # 현재 상태
git log --oneline   # 커밋 로그
git branch -a       # 모든 브랜치
```

#### 1.3 설치된 도구 확인
```bash
git --version              # Git 버전
docker --version           # Docker 버전
docker-compose --version   # Docker Compose 버전
rustc --version            # Rust 컴파일러
cargo --version            # Cargo 빌드 도구
```

---

### Phase 2: Docker & Ollama 디버깅 🐳

#### 2.1 Docker 상태 확인
```bash
# Docker 서비스 상태
docker ps                    # 실행 중인 컨테이너
docker ps -a                 # 모든 컨테이너
docker version              # 버전 정보
docker info                 # 상세 정보 (메모리, CPU 등)
```

#### 2.2 Ollama 컨테이너 확인
```bash
# 컨테이너 시작
docker-compose up -d

# 컨테이너 상태 확인
docker-compose ps

# 로그 확인 (실시간)
docker logs -f ai_agent_ollama

# 마지막 50줄만
docker logs --tail 50 ai_agent_ollama

# 타임스탬프 포함
docker logs -t ai_agent_ollama

# 문제 디버깅
docker exec ai_agent_ollama bash -c "ollama list"
docker exec ai_agent_ollama curl http://localhost:11434/api/tags
```

#### 2.3 리소스 모니터링
```bash
# 실시간 모니터링
docker stats ai_agent_ollama

# 메모리 사용량 확인
docker exec ai_agent_ollama free -h

# 디스크 사용량
docker exec ai_agent_ollama df -h
```

---

### Phase 3: Rust 프로젝트 디버깅 🦀

#### 3.1 기본 빌드 테스트
```bash
# 디버그 빌드
cargo build

# 릴리스 빌드
cargo build --release

# 빌드 타임 측정
cargo build --timings
```

#### 3.2 테스트 실행
```bash
# 모든 테스트
cargo test

# 특정 테스트만
cargo test test_name

# 출력 표시
cargo test -- --nocapture

# 싱글 스레드
cargo test -- --test-threads=1
```

#### 3.3 코드 품질 확인
```bash
# 포맷 확인
cargo fmt --check

# 포맷 자동 수정
cargo fmt

# 린트 (경고 확인)
cargo clippy

# 린트 (strict 모드)
cargo clippy -- -D warnings
```

#### 3.4 상세 로그 출력
```bash
# 디버그 로그
RUST_LOG=debug cargo run

# 트레이스 로그 (매우 상세)
RUST_LOG=trace cargo run

# 특정 모듈만
RUST_LOG=ai_agent=debug,tokio=info cargo run

# 백트레이스 활성화
RUST_BACKTRACE=1 cargo run
RUST_BACKTRACE=full cargo run
```

---

### Phase 4: VSCode 디버깅 🎯

#### 4.1 VSCode 설정
```
1. Extensions 설치
   - "CodeLLDB" (vadimcn.vscode-lldb)
   - "Rust Analyzer" (rust-lang.rust-analyzer)

2. .vscode/launch.json 생성 (또는 수정)
   - 파일 내용: vscode_launch.json 참고

3. .vscode/settings.json 생성 (또는 수정)
   - 파일 내용: vscode_settings.json 참고
```

#### 4.2 디버거 사용 (F5)
```
1. 중단점 설정
   - 코드 옆 숫자 줄에서 클릭

2. 디버깅 시작
   - F5 누르거나
   - Run > Start Debugging

3. 디버거 명령
   - F10: Step over
   - F11: Step into
   - Shift+F11: Step out
   - F6: Continue
   - Shift+F5: Stop

4. 변수 검사
   - 왼쪽 "Variables" 탭
   - 마우스 호버로 변수 값 확인
```

---

### Phase 5: 성능 프로파일링 📊

#### 5.1 Flamegraph (병목 구간 찾기)
```bash
# 설치
cargo install flamegraph

# 프로파일링 실행
cargo flamegraph

# SVG 파일 생성됨: flamegraph.svg
# 웹 브라우저로 열기
start flamegraph.svg  # Windows
open flamegraph.svg   # macOS
xdg-open flamegraph.svg  # Linux
```

#### 5.2 메모리 프로파일링 (Linux)
```bash
# Valgrind 설치 (Linux만 가능)
sudo apt install valgrind  # Ubuntu/Debian

# 메모리 누수 검사
valgrind --leak-check=full ./target/debug/ai_agent

# 요약만 보기
valgrind --leak-check=summary ./target/debug/ai_agent
```

---

### Phase 6: 배포 검증 📦

#### 6.1 배포 패키지 생성
```bash
# Windows
deploy.bat

# Linux/macOS
./deploy.sh

# 결과: dist/ 폴더 생성
# - ai_agent (바이너리)
# - docker-compose.yml
# - README.md
# - DEPLOY_INFO.txt
```

#### 6.2 배포 패키지 검증
```bash
# dist 폴더로 이동
cd dist

# 바이너리 크기 확인
ls -lh ai_agent

# 필수 파일 확인
ls -la

# 설정 파일 확인
cat docker-compose.yml
cat DEPLOY_INFO.txt
```

---

### Phase 7: 문제 해결 🔧

#### 7.1 Docker 문제

**포트 충돌**
```bash
# 11434 포트 사용 중 확인
netstat -ano | findstr 11434  # Windows
lsof -i :11434                # macOS/Linux

# 해결: docker-compose.yml에서 포트 변경
# ports:
#   - "11435:11434"
```

**메모리 부족**
```bash
# 현재 메모리 사용량 확인
docker stats --no-stream

# 컨테이너 재시작
docker-compose down
docker-compose up -d

# Docker 메모리 설정 증가
# Docker Desktop → Settings → Resources → Memory
```

**컨테이너 시작 실패**
```bash
# 로그 확인
docker logs -f ai_agent_ollama

# 컨테이너 상세 정보
docker inspect ai_agent_ollama

# 강제 재시작
docker-compose down -v
docker-compose up -d
```

#### 7.2 Rust 컴파일 문제

**Cargo 캐시 오류**
```bash
# 캐시 정리
cargo clean

# 재빌드
cargo build

# 또는 .cargo 디렉토리 삭제
rm -rf ~/.cargo
cargo build
```

**의존성 충돌**
```bash
# Cargo.lock 재생성
rm Cargo.lock
cargo build

# 또는 업데이트
cargo update
```

**컴파일 성능 개선**
```bash
# Incremental 빌드 활성화
CARGO_INCREMENTAL=1 cargo build

# 병렬 빌드 수 조절
cargo build -j 4

# 최적화 빌드
cargo build --release -Z codegen-backend=cranelift
```

#### 7.3 로그 디버깅

**로그 활성화**
```bash
# 주요 로그 레벨
RUST_LOG=error cargo run     # 에러만
RUST_LOG=warn cargo run      # 경고 이상
RUST_LOG=info cargo run      # 정보 이상
RUST_LOG=debug cargo run     # 디버그 이상
RUST_LOG=trace cargo run     # 모든 로그

# 다중 모듈 필터
RUST_LOG=ai_agent::agent=debug,ai_agent::tools=info cargo run

# 백트레이스
RUST_BACKTRACE=1 cargo run   # 짧은 백트레이스
RUST_BACKTRACE=full cargo run # 전체 백트레이스
```

**로그 파일로 저장**
```bash
# Windows
RUST_LOG=debug cargo run > debug.log 2>&1

# Linux/macOS
RUST_LOG=debug cargo run > debug.log 2>&1

# 실시간 모니터링
tail -f debug.log
```

---

### Phase 8: CI/CD 테스트 🔄

#### 8.1 로컬에서 CI 테스트
```bash
# GitHub Actions 시뮬레이션
# (act 도구 필요: https://github.com/nektos/act)

# 설치
choco install act  # Windows
brew install act   # macOS

# 실행
act push  # push 이벤트 시뮬레이션
act pull_request  # PR 시뮬레이션
```

#### 8.2 수동 CI 검증
```bash
# 테스트
cargo test --release

# 린트
cargo clippy --release -- -D warnings

# 포맷 확인
cargo fmt -- --check

# 문서
cargo doc --no-deps
```

---

## 📊 디버깅 체크리스트

### 초기 디버깅
- [ ] Git 저장소 확인
- [ ] Docker 서비스 실행
- [ ] Ollama 컨테이너 상태
- [ ] Rust 버전 확인
- [ ] Cargo 의존성 다운로드

### 코드 디버깅
- [ ] 디버그 빌드 성공
- [ ] 모든 테스트 통과
- [ ] 린트 경고 없음
- [ ] 포맷 정상
- [ ] 로그 출력 정상

### 성능 디버깅
- [ ] 메모리 사용량 정상
- [ ] CPU 사용률 정상
- [ ] 응답 시간 정상
- [ ] 리소스 누수 없음

### 배포 디버깅
- [ ] 배포 패키지 생성
- [ ] 바이너리 실행 가능
- [ ] 설정 파일 포함
- [ ] 문서 포함

---

## 🎯 권장 디버깅 순서

1. **파일 검증** → 모든 파일이 제대로 있는지
2. **도구 확인** → Git, Docker, Rust 설치됨
3. **Docker 테스트** → docker-compose up -d
4. **Rust 빌드** → cargo build
5. **테스트 실행** → cargo test
6. **로그 확인** → RUST_LOG=debug cargo run
7. **성능 분석** → flamegraph, profiling
8. **배포 테스트** → deploy.bat
9. **CI/CD 테스트** → GitHub Actions 시뮬레이션

---

## 🆘 빠른 참고

| 문제 | 해결 |
|------|------|
| Docker 안 켜짐 | Docker Desktop 재시작 |
| 포트 충돌 | docker-compose.yml 포트 변경 |
| 메모리 부족 | Docker에 8GB+ 할당 |
| 컴파일 오류 | cargo clean && cargo build |
| 로그 안 보임 | RUST_LOG=debug 설정 |

---

**모든 디버깅 준비가 완료되었습니다! 🚀**
