#!/bin/bash
# 실행 보고서 생성 스크립트

echo "════════════════════════════════════════════════════════════"
echo "           🚀 프로젝트 실행 시뮬레이션 시작"
echo "════════════════════════════════════════════════════════════"
echo ""

# Phase 1: 환경 확인
echo "[Phase 1] 시스템 환경 확인 중..."
echo ""

# Git 확인
echo "Git 확인:"
git --version 2>/dev/null && echo "  ✅ Git 설치됨" || echo "  ❌ Git 미설치"

# Docker 확인
echo "Docker 확인:"
docker --version 2>/dev/null && echo "  ✅ Docker 설치됨" || echo "  ❌ Docker 미설치"

# Docker Compose 확인
echo "Docker Compose 확인:"
docker-compose --version 2>/dev/null && echo "  ✅ Docker Compose 설치됨" || echo "  ❌ Docker Compose 미설치"

# Rust 확인
echo "Rust 확인:"
rustc --version 2>/dev/null && echo "  ✅ Rust 설치됨" || echo "  ❌ Rust 미설치"

# Cargo 확인
echo "Cargo 확인:"
cargo --version 2>/dev/null && echo "  ✅ Cargo 설치됨" || echo "  ❌ Cargo 미설치"

echo ""

# Phase 2: 파일 구조 확인
echo "[Phase 2] 파일 구조 확인 중..."
echo ""

FILE_COUNT=$(find . -maxdepth 1 -type f \( -name "*.bat" -o -name "*.sh" -o -name "*.toml" -o -name "*.yml" -o -name "*.md" -o -name "*.txt" -o -name "*.json" -o -name "*.rs" \) | wc -l)
echo "  생성된 파일: $FILE_COUNT개"

if [ -f "Cargo.toml" ]; then
    echo "  ✅ Cargo.toml 존재"
else
    echo "  ❌ Cargo.toml 없음"
fi

if [ -f "docker-compose.yml" ]; then
    echo "  ✅ docker-compose.yml 존재"
else
    echo "  ❌ docker-compose.yml 없음"
fi

if [ -f "init_git.bat" ]; then
    echo "  ✅ init_git.bat 존재"
else
    echo "  ❌ init_git.bat 없음"
fi

if [ -f "quick-start.bat" ]; then
    echo "  ✅ quick-start.bat 존재"
else
    echo "  ❌ quick-start.bat 없음"
fi

echo ""

# Phase 3: Git 상태
echo "[Phase 3] Git 상태 확인 중..."
echo ""

if [ -d ".git" ]; then
    echo "  ✅ .git 저장소 존재"
    git status --short | head -5
else
    echo "  ⚠️  .git 저장소 아직 초기화 안 됨"
    echo "  (init_git.bat 실행 후 초기화됨)"
fi

echo ""

# Phase 4: 실행 준비 상태
echo "[Phase 4] 실행 준비 상태 평가"
echo ""

READY=true

if ! command -v git &> /dev/null; then
    echo "  ⚠️  Git 설치 필요"
    READY=false
fi

if ! command -v docker &> /dev/null; then
    echo "  ⚠️  Docker 설치 필요"
    READY=false
fi

if ! command -v cargo &> /dev/null; then
    echo "  ⚠️  Rust 설치 필요"
    READY=false
fi

if [ "$READY" = true ]; then
    echo "  ✅ 모든 필수 도구 설치됨!"
    echo ""
    echo "  준비 완료! 다음 명령어 실행:"
    echo "    1. init_git.bat"
    echo "    2. quick-start.bat"
else
    echo ""
    echo "  필수 도구 설치:"
    echo "    - Docker: https://www.docker.com/products/docker-desktop"
    echo "    - Rust: https://www.rust-lang.org/tools/install"
    echo "    - Git: https://git-scm.com/download/win"
fi

echo ""
echo "════════════════════════════════════════════════════════════"
echo "           보고서 생성 완료"
echo "════════════════════════════════════════════════════════════"
