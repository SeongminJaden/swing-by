#!/bin/bash
# Git 저장소 초기화 및 커밋 스크립트

set -e

echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║        Git 저장소 초기화 및 첫 커밋 시작              ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Git 저장소 확인
if [ -d .git ]; then
    echo "✓ Git 저장소 이미 초기화됨"
else
    echo "📍 Git 저장소 초기화 중..."
    git init
    git config user.name "AI Agent Setup"
    git config user.email "agent@local"
fi

# 현재 상태 확인
echo ""
echo "📋 현재 상태:"
git status --short | head -20 || echo "변경 사항 없음"

# 모든 파일 추가
echo ""
echo "📍 모든 파일 스테이징 중..."
git add -A

# 커밋
echo ""
echo "📍 첫 커밋 실행 중..."
git commit -m "Initial commit: Complete Docker + Ollama + Rust AI Agent setup

This commit includes:

🐳 Docker & Ollama Setup
- docker-compose.yml: Ollama container configuration
- setup-ollama.bat/sh: Automated setup scripts

🦀 Rust Project
- Cargo.toml: All dependencies configured
- main.rs: Entry point

📚 Documentation (14 files)
- README.md: Project overview
- QUICK_START.md: 5-minute quick start
- DEBUG_GUIDE.md: Debugging setup and tools
- DEPLOY_GUIDE.md: Deployment procedures
- And 10 more detailed guides

⚡ Automation & Deployment
- quick-start.bat/sh: One-click environment setup
- deploy.bat/sh: Production deployment scripts

🔧 Development Setup
- VSCode debug configuration
- GitHub Actions CI/CD pipeline

✨ Key Features:
✓ Complete Docker + Ollama configuration
✓ Rust project with all dependencies
✓ Debug tools (VSCode, GDB, LLDB, Tracing)
✓ Deployment automation
✓ CI/CD pipeline setup
✓ 14 comprehensive documentation files

Ready for development and deployment!

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"

# 커밋 로그 확인
echo ""
echo "✅ 커밋 완료!"
echo ""
echo "📊 커밋 로그:"
git log --oneline -5

# 상태 확인
echo ""
echo "📋 현재 상태:"
git status

echo ""
echo "════════════════════════════════════════════════════════"
echo "🎉 Git 저장소 준비 완료!"
echo "════════════════════════════════════════════════════════"
echo ""
echo "다음 단계:"
echo "  1. 원격 저장소 추가 (선택):"
echo "     git remote add origin <repository-url>"
echo "     git branch -M main"
echo "     git push -u origin main"
echo ""
echo "  2. 개발 시작:"
echo "     ./quick-start.bat  또는  ./quick-start.sh"
echo ""
echo "  3. 배포:"
echo "     ./deploy.bat  또는  ./deploy.sh"
echo ""
