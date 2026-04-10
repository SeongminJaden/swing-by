# 🚀 배포 가이드

## 빠른 배포 (5분)

### Windows
```powershell
.\deploy.bat
```

### Linux/macOS
```bash
chmod +x deploy.sh
./deploy.sh
```

**결과**: `dist/` 폴더에 배포 가능한 패키지 생성

---

## 📦 배포 프로세스

### Step 1: 테스트
```bash
cargo test --release
```

### Step 2: 린트 확인
```bash
cargo clippy --release -- -D warnings
```

### Step 3: 최적화 빌드
```bash
cargo build --release
```

### Step 4: 패키지 생성
```bash
# Windows
deploy.bat

# Linux/macOS
./deploy.sh
```

### Step 5: 배포
```bash
# dist/ 폴더를 대상 서버로 복사
scp -r dist/ user@server:/path/to/deployment/
```

---

## 📋 배포 검사 항목

- [ ] 모든 테스트 통과
- [ ] 린트 경고 없음
- [ ] 빌드 성공
- [ ] 바이너리 생성됨
- [ ] 의존성 파일 포함됨
- [ ] 문서 포함됨
- [ ] 배포 정보 생성됨

---

## 🔧 배포 후 확인

```bash
# 1. 설정 확인
docker-compose config

# 2. 컨테이너 시작
docker-compose up -d

# 3. 서비스 확인
docker-compose ps

# 4. 로그 확인
docker-compose logs -f

# 5. API 테스트
curl http://localhost:11434/api/tags
```

---

## 🐳 Docker 배포

### 1. Dockerfile 생성 (선택)

```dockerfile
FROM rust:latest as builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/ai_agent .
COPY docker-compose.yml .

EXPOSE 11434

CMD ["./ai_agent"]
```

### 2. Docker 빌드 및 실행

```bash
# 빌드
docker build -t ai_agent:latest .

# 실행
docker run -p 11434:11434 ai_agent:latest
```

---

## 📍 배포 위치별 지침

### 로컬 개발
```bash
./quick-start.bat  # 자동 설정
cargo run          # 실행
```

### 로컬 서버
```bash
# deploy 폴더 실행
cd dist/
./ai_agent
```

### 원격 서버 (Linux)
```bash
# 배포
scp -r dist/ user@server:/opt/ai_agent/

# 원격 실행
ssh user@server
cd /opt/ai_agent
docker-compose up -d
```

### 클라우드 (AWS/GCP/Azure)
```bash
# ECR에 푸시
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin 123456789.dkr.ecr.us-east-1.amazonaws.com

docker tag ai_agent:latest 123456789.dkr.ecr.us-east-1.amazonaws.com/ai_agent:latest
docker push 123456789.dkr.ecr.us-east-1.amazonaws.com/ai_agent:latest
```

---

## 📊 배포 체크리스트

```
[] 1. 로컬에서 테스트 완료
[] 2. 모든 커밋 푸시 완료
[] 3. deploy.bat/sh 실행
[] 4. dist/ 폴더 생성 확인
[] 5. 배포 대상 환경 준비
[] 6. 패키지 전송
[] 7. 서비스 시작
[] 8. 헬스 체크 확인
[] 9. 로그 모니터링
[] 10. 배포 완료 기록
```

---

## 🔄 업데이트 배포

### 간단 업데이트
```bash
git pull
cargo build --release
# 기존 프로세스 종료
# 새 바이너리 실행
```

### 무중단 배포 (선택)
```bash
# 로드 밸런서에서 기존 인스턴스 제거
# 새 인스턴스 시작
# 로드 밸런서에 추가
# 기존 인스턴스 종료
```

---

## 🆘 배포 문제 해결

### "배포 후 서비스 시작 안 됨"
```bash
# 로그 확인
docker logs -f ai_agent_ollama

# 포트 확인
netstat -ano | findstr 11434

# 메모리 확인
docker stats
```

### "성능 저하"
```bash
# 리소스 모니터링
docker stats --no-stream

# 프로파일링
cargo flamegraph --release
```

### "메모리 누수"
```bash
# 상세 로그
RUST_LOG=debug cargo run --release

# Valgrind (Linux)
valgrind --leak-check=full ./target/release/ai_agent
```

---

## 📈 배포 모니터링

### 리소스 모니터링
```bash
# 지속적 모니터링
watch -n 1 'docker stats --no-stream'

# 또는
docker stats ai_agent_ollama
```

### 로그 모니터링
```bash
# 실시간 로그
docker logs -f ai_agent_ollama

# 마지막 N줄
docker logs --tail 100 ai_agent_ollama

# 타임스탬프 포함
docker logs -t ai_agent_ollama
```

### 헬스 체크
```bash
# API 응답성 확인
curl -s http://localhost:11434/api/tags | jq .

# 주기적 확인 스크립트
while true; do
  curl -s http://localhost:11434/api/tags > /dev/null && echo "OK" || echo "FAIL"
  sleep 30
done
```

---

## 🎯 배포 후 확인 사항

1. ✅ 서비스 시작 확인
2. ✅ API 응답 확인
3. ✅ 모델 로드 확인
4. ✅ 로그 이상 없음
5. ✅ 메모리 정상
6. ✅ CPU 정상
7. ✅ 네트워크 정상

---

**배포 준비 완료! 행운을 빕니다! 🚀**
