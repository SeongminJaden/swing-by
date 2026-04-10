@echo off
REM ⚡ 빠른 시작 배치 스크립트 - 전체 환경 자동 설정 (Windows)

setlocal enabledelayedexpansion

cls
echo.
echo =====================================================
echo     Rust AI Agent - 전체 환경 자동 설정 시작
echo =====================================================
echo.

REM Step 1: Docker 확인
echo [1/3] Docker 설치 확인...
where docker >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ❌ Docker가 설치되지 않았습니다.
    echo Docker Desktop 설치: https://www.docker.com/products/docker-desktop
    pause
    exit /b 1
)

for /f "tokens=*" %%i in ('docker --version') do set DOCKER_VER=%%i
echo ✓ Docker 확인 완료: %DOCKER_VER%
echo.

REM Step 2: Ollama 시작
echo [2/3] Ollama 컨테이너 시작...
docker-compose down 2>nul
timeout /t 2 /nobreak >nul
docker-compose up -d

echo Ollama 서비스 시작 대기 중... (최대 60초)
setlocal enabledelayedexpansion
set attempt=0
:check_ollama
set /a attempt=!attempt!+1
if !attempt! gtr 30 (
    echo.
    echo ❌ Ollama 서비스 시작 실패
    pause
    exit /b 1
)

docker exec ai_agent_ollama curl -s http://localhost:11434/api/tags >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo ✓ Ollama 준비 완료
    goto download_model
)
echo   시도 !attempt!/30...
timeout /t 2 /nobreak >nul
goto check_ollama

:download_model
echo.
echo [3/3] Gemma 모델 다운로드...
echo 첫 실행 시 시간이 걸릴 수 있습니다. (10-30분)
echo.
docker exec ai_agent_ollama ollama pull gemma:latest

REM Rust 확인
echo.
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ❌ Rust가 설치되지 않았습니다.
    echo Rust 설치: https://www.rust-lang.org/tools/install
    pause
    exit /b 1
)

echo ✓ Rust 설치됨
echo 의존성 다운로드 중...
cargo build --release 2>nul

echo.
echo =====================================================
echo        🎉 모든 설정이 완료되었습니다!
echo =====================================================
echo.
echo 다음 명령어로 에이전트를 실행하세요:
echo   cargo run --release
echo.
echo 기타 유용한 명령어:
echo   - 모델 확인: docker exec ai_agent_ollama ollama list
echo   - 로그 확인: docker logs -f ai_agent_ollama
echo   - API 테스트: curl http://localhost:11434/api/tags
echo.
pause
