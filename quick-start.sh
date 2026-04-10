#!/bin/bash
# ⚡ 빠른 시작 스크립트 - 전체 환경 자동 설정

set -e

echo ""
echo "╔═══════════════════════════════════════════════════════╗"
echo "║     Rust AI Agent - 전체 환경 자동 설정 시작         ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo ""

# 색상 정의
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Docker 및 Ollama 확인
echo -e "${BLUE}[1/3] Docker 및 Ollama 설정 확인...${NC}"
if ! command -v docker &> /dev/null; then
    echo -e "${YELLOW}⚠️  Docker가 설치되지 않았습니다.${NC}"
    echo "Docker Desktop 설치: https://www.docker.com/products/docker-desktop"
    exit 1
fi

# Ollama 시작
echo -e "${GREEN}✓ Docker 확인 완료${NC}"
echo "Ollama 컨테이너 시작 중..."
docker-compose up -d

# Ollama 준비 대기
echo "Ollama 서비스 시작 대기 중... (최대 60초)"
for i in {1..30}; do
    if docker exec ai_agent_ollama curl -s http://localhost:11434/api/tags &>/dev/null; then
        echo -e "${GREEN}✓ Ollama 준비 완료${NC}"
        break
    fi
    echo "  시도 $i/30..."
    sleep 2
done

# Step 2: Gemma 모델 다운로드
echo ""
echo -e "${BLUE}[2/3] Gemma 모델 다운로드${NC}"
echo "첫 실행 시 시간이 걸릴 수 있습니다. (10-30분)"
docker exec ai_agent_ollama ollama pull gemma:latest
echo -e "${GREEN}✓ 모델 다운로드 완료${NC}"

# Step 3: Rust 프로젝트 준비
echo ""
echo -e "${BLUE}[3/3] Rust 프로젝트 준비${NC}"

if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠️  Rust가 설치되지 않았습니다.${NC}"
    echo "Rust 설치: https://www.rust-lang.org/tools/install"
    exit 1
fi

echo "의존성 다운로드 중..."
cargo build --release 2>&1 | tail -5

echo -e "${GREEN}✓ Rust 프로젝트 준비 완료${NC}"

# 완료
echo ""
echo "╔═══════════════════════════════════════════════════════╗"
echo "║          🎉 모든 설정이 완료되었습니다!              ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo ""
echo "다음 명령어로 에이전트를 실행하세요:"
echo "  ${GREEN}cargo run --release${NC}"
echo ""
echo "기타 유용한 명령어:"
echo "  - 모델 확인: ${BLUE}docker exec ai_agent_ollama ollama list${NC}"
echo "  - 로그 확인: ${BLUE}docker logs -f ai_agent_ollama${NC}"
echo "  - API 테스트: ${BLUE}curl http://localhost:11434/api/tags${NC}"
echo ""
