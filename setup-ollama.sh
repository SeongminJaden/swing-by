#!/bin/bash
# Ollama와 Gemma 설정 자동화 스크립트

set -e

echo "================================"
echo "Ollama + Gemma 설정 시작"
echo "================================"

# 1. Docker 설치 확인
echo ""
echo "[1/4] Docker 설치 확인..."
if ! command -v docker &> /dev/null; then
    echo "❌ Docker가 설치되지 않았습니다."
    echo "다음을 실행하세요: https://www.docker.com/products/docker-desktop"
    exit 1
fi
echo "✅ Docker 설치됨: $(docker --version)"

# 2. Docker Compose 확인
echo ""
echo "[2/4] Docker Compose 확인..."
if ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose가 설치되지 않았습니다."
    exit 1
fi
echo "✅ Docker Compose 설치됨"

# 3. Ollama 컨테이너 시작
echo ""
echo "[3/4] Ollama 컨테이너 시작 중..."
cd "$(dirname "$0")"
docker-compose down 2>/dev/null || true
docker-compose up -d
sleep 3

# 컨테이너가 준비될 때까지 대기
echo "Ollama 서비스 시작 대기 중..."
max_attempts=30
attempt=0
while [ $attempt -lt $max_attempts ]; do
    if docker exec ai_agent_ollama curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
        echo "✅ Ollama 서비스 준비 완료"
        break
    fi
    attempt=$((attempt + 1))
    echo "  시도 $attempt/$max_attempts..."
    sleep 2
done

if [ $attempt -eq $max_attempts ]; then
    echo "❌ Ollama 서비스 시작 실패"
    exit 1
fi

# 4. Gemma 모델 다운로드
echo ""
echo "[4/4] Gemma 모델 다운로드 중..."
echo "(이는 최초 실행 시 시간이 걸릴 수 있습니다)"
docker exec ai_agent_ollama ollama pull gemma:latest

echo ""
echo "================================"
echo "✅ 설정 완료!"
echo "================================"
echo ""
echo "Ollama API 주소: http://localhost:11434"
echo "모델 확인: curl http://localhost:11434/api/tags"
echo ""
echo "테스트:"
echo "  curl -X POST http://localhost:11434/api/generate \\"
echo "    -d '{\"model\": \"gemma:latest\", \"prompt\": \"Hello\", \"stream\": false}'"
