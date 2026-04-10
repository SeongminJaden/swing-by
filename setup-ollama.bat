@echo off
REM Ollama + Gemma 설정 자동화 스크립트 (Windows)

setlocal enabledelayedexpansion

echo.
echo ================================
echo Ollama + Gemma 설정 시작
echo ================================
echo.

REM 1. Docker 설치 확인
echo [1/4] Docker 설치 확인...
where docker >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo.❌ Docker가 설치되지 않았습니다.
    echo Docker Desktop을 설치하세요: https://www.docker.com/products/docker-desktop
    pause
    exit /b 1
)
for /f "tokens=*" %%i in ('docker --version') do set DOCKER_VERSION=%%i
echo ✅ Docker 설치됨: %DOCKER_VERSION%
echo.

REM 2. Ollama 컨테이너 시작
echo [2/4] Ollama 컨테이너 시작 중...
cd /d "%~dp0"
docker-compose down 2>nul
timeout /t 2 /nobreak >nul
docker-compose up -d

echo [3/4] Ollama 서비스 시작 대기 중... (최대 60초)
timeout /t 3 /nobreak >nul

setlocal enabledelayedexpansion
set "attempt=0"
:wait_loop
set /a attempt=!attempt!+1
if !attempt! gtr 30 (
    echo.❌ Ollama 서비스 시작 실패
    pause
    exit /b 1
)

docker exec ai_agent_ollama curl -s http://localhost:11434/api/tags >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo ✅ Ollama 서비스 준비 완료
    goto setup_model
)
echo   시도 !attempt!/30...
timeout /t 2 /nobreak >nul
goto wait_loop

:setup_model
echo.
echo [4/4] Gemma 모델 다운로드 중...
echo (이는 최초 실행 시 시간이 걸릴 수 있습니다)
docker exec ai_agent_ollama ollama pull gemma:latest

echo.
echo ================================
echo ✅ 설정 완료!
echo ================================
echo.
echo Ollama API 주소: http://localhost:11434
echo.
echo 모델 확인:
echo   docker exec ai_agent_ollama ollama list
echo.
echo 테스트:
echo   curl -X POST http://localhost:11434/api/generate ^
echo     -d "{\"model\": \"gemma:latest\", \"prompt\": \"Hello\", \"stream\": false}"
echo.
pause
