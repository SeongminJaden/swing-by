# Ollama + Gemma 설정 가이드

## 빠른 시작 (자동 스크립트)

### Windows
```powershell
.\setup-ollama.bat
```

### Linux/macOS
```bash
chmod +x setup-ollama.sh
./setup-ollama.sh
```

---

## 수동 설정 단계

### 1단계: Docker Compose로 Ollama 시작

```bash
docker-compose up -d
```

**확인:**
```bash
docker ps | grep ai_agent_ollama
```

### 2단계: Ollama 서비스 확인

```bash
# 로그 확인
docker logs -f ai_agent_ollama

# API 테스트
curl http://localhost:11434/api/tags
```

### 3단계: Gemma 모델 다운로드

```bash
# 최신 Gemma 모델 (권장)
docker exec ai_agent_ollama ollama pull gemma:latest

# 또는 특정 크기 선택:
docker exec ai_agent_ollama ollama pull gemma:7b      # 7B 모델
docker exec ai_agent_ollama ollama pull gemma:13b     # 13B 모델
```

**다운로드 시간:**
- gemma:7b: ~4-5GB (약 10-15분)
- gemma:13b: ~8-10GB (약 20-30분)

### 4단계: 설치 확인

```bash
# 설치된 모델 확인
docker exec ai_agent_ollama ollama list

# 간단한 테스트
curl -X POST http://localhost:11434/api/generate \
  -d '{
    "model": "gemma:latest",
    "prompt": "Hello, how are you?",
    "stream": false
  }'
```

---

## 정지/재시작

```bash
# 중지
docker-compose down

# 재시작
docker-compose up -d

# 로그 확인
docker-compose logs -f

# 컨테이너 내부 접속
docker exec -it ai_agent_ollama bash
```

---

## 문제 해결

### 메모리 부족
```bash
# Docker 메모리 설정 (보통 8GB 이상 필요)
# Docker Desktop → Settings → Resources → Memory
```

### 포트 충돌 (11434 포트 사용 중)
```yaml
# docker-compose.yml에서 포트 변경:
ports:
  - "11435:11434"  # 외부 11435로 변경
```

### 모델 삭제
```bash
docker exec ai_agent_ollama ollama rm gemma:latest
```

### 모든 데이터 초기화
```bash
docker-compose down -v
```

---

## Ollama API 엔드포인트

| 엔드포인트 | 설명 |
|-----------|------|
| `GET /api/tags` | 설치된 모델 목록 |
| `POST /api/generate` | 텍스트 생성 |
| `POST /api/chat` | 채팅 (대화) |
| `POST /api/pull` | 모델 다운로드 |
| `DELETE /api/delete` | 모델 삭제 |

---

## Rust 에이전트와 연동

Rust 코드에서 Ollama API 호출 예시:

```rust
use reqwest::Client;

#[tokio::main]
async fn main() {
    let client = Client::new();
    
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": "gemma:latest",
            "prompt": "Hello",
            "stream": false
        }))
        .send()
        .await
        .expect("API 호출 실패");
    
    println!("{:?}", response.json::<serde_json::Value>().await);
}
```

---

## 다음 단계

Docker와 Ollama 설정이 완료되었으면:

1. **Rust 프로젝트 초기화** → `cargo new ai_agent`
2. **의존성 추가** → reqwest, tokio, serde_json 등
3. **Ollama 클라이언트 구현** → API 연동
4. **Tool Layer 개발** → 코드 실행, 파일 I/O 등
