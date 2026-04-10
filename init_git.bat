@echo off
REM Git 저장소 초기화 및 커밋 스크립트 (Windows)

setlocal enabledelayedexpansion

cls
echo.
echo ════════════════════════════════════════════════════════
echo         Git 저장소 초기화 및 첫 커밋 시작
echo ════════════════════════════════════════════════════════
echo.

REM Git 설치 확인
where git >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Git이 설치되지 않았습니다.
    echo Git 설치: https://git-scm.com/download/win
    pause
    exit /b 1
)

REM Git 저장소 확인
if exist .git (
    echo ✓ Git 저장소 이미 초기화됨
) else (
    echo 📍 Git 저장소 초기화 중...
    git init
    git config user.name "AI Agent Setup"
    git config user.email "agent@local"
)

REM 상태 확인
echo.
echo 📋 현재 상태:
git status --short | findstr /R "." || echo 변경 사항 없음
echo.

REM 파일 추가
echo 📍 모든 파일 스테이징 중...
git add -A

REM 커밋
echo 📍 첫 커밋 실행 중...
git commit -m "Initial commit: Complete Docker + Ollama + Rust AI Agent setup

This commit includes:

Docker and Ollama Setup
- docker-compose.yml: Ollama container configuration
- setup-ollama.bat/sh: Automated setup scripts

Rust Project
- Cargo.toml: All dependencies configured
- main.rs: Entry point

Documentation (14 files)
- README.md: Project overview
- QUICK_START.md: 5-minute quick start
- DEBUG_GUIDE.md: Debugging setup and tools
- DEPLOY_GUIDE.md: Deployment procedures
- Complete development and deployment guides

Automation and Deployment
- quick-start.bat/sh: One-click environment setup
- deploy.bat/sh: Production deployment scripts

Development Setup
- VSCode debug configuration
- GitHub Actions CI/CD pipeline

Key Features:
✓ Complete Docker + Ollama configuration
✓ Rust project with all dependencies
✓ Debug tools (VSCode, GDB, LLDB, Tracing)
✓ Deployment automation
✓ CI/CD pipeline setup
✓ 14 comprehensive documentation files

Ready for development and deployment!

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"

REM 로그 확인
echo.
echo ✅ 커밋 완료!
echo.
echo 📊 커밋 로그:
git log --oneline -5

REM 상태 확인
echo.
echo 📋 현재 상태:
git status

echo.
echo ════════════════════════════════════════════════════════
echo 🎉 Git 저장소 준비 완료!
echo ════════════════════════════════════════════════════════
echo.
echo 다음 단계:
echo   1. 원격 저장소 추가 (선택):
echo      git remote add origin ^<repository-url^>
echo      git branch -M main
echo      git push -u origin main
echo.
echo   2. 개발 시작:
echo      .\quick-start.bat
echo.
echo   3. 배포:
echo      .\deploy.bat
echo.
pause
