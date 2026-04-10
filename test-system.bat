@echo off
REM 🧪 테스트 및 검증 스크립트
REM 모든 설정이 제대로 작동하는지 확인

setlocal enabledelayedexpansion
cls

echo.
echo ═══════════════════════════════════════════════════════════
echo          🧪 시스템 검증 및 환경 테스트 시작
echo ═══════════════════════════════════════════════════════════
echo.

REM 결과 저장
set PASS=0
set FAIL=0

REM Test 1: Git 확인
echo [Test 1/8] Git 설치 확인...
where git >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo ✅ Git 설치됨
    set /a PASS+=1
) else (
    echo ❌ Git 미설치
    set /a FAIL+=1
)

REM Test 2: Docker 확인
echo [Test 2/8] Docker 설치 확인...
where docker >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo ✅ Docker 설치됨
    for /f "tokens=*" %%i in ('docker --version') do echo   %%i
    set /a PASS+=1
) else (
    echo ❌ Docker 미설치
    set /a FAIL+=1
)

REM Test 3: Docker Compose 확인
echo [Test 3/8] Docker Compose 확인...
where docker-compose >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo ✅ Docker Compose 설치됨
    for /f "tokens=*" %%i in ('docker-compose --version') do echo   %%i
    set /a PASS+=1
) else (
    echo ⚠️  Docker Compose 찾기 어려움 (docker-compose 사용 가능할 수도 있음)
    set /a FAIL+=1
)

REM Test 4: Rust 확인
echo [Test 4/8] Rust 설치 확인...
where rustc >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo ✅ Rust 설치됨
    for /f "tokens=*" %%i in ('rustc --version') do echo   %%i
    set /a PASS+=1
) else (
    echo ❌ Rust 미설치
    set /a FAIL+=1
)

REM Test 5: Cargo 확인
echo [Test 5/8] Cargo 확인...
where cargo >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo ✅ Cargo 설치됨
    for /f "tokens=*" %%i in ('cargo --version') do echo   %%i
    set /a PASS+=1
) else (
    echo ❌ Cargo 미설치
    set /a FAIL+=1
)

REM Test 6: Cargo.toml 확인
echo [Test 6/8] Cargo.toml 파일 확인...
if exist "Cargo.toml" (
    echo ✅ Cargo.toml 파일 존재
    echo   프로젝트명: ai_agent
    echo   버전: 0.1.0
    set /a PASS+=1
) else (
    echo ❌ Cargo.toml 파일 없음
    set /a FAIL+=1
)

REM Test 7: docker-compose.yml 확인
echo [Test 7/8] docker-compose.yml 확인...
if exist "docker-compose.yml" (
    echo ✅ docker-compose.yml 파일 존재
    set /a PASS+=1
) else (
    echo ❌ docker-compose.yml 파일 없음
    set /a FAIL+=1
)

REM Test 8: 스크립트 파일 확인
echo [Test 8/8] 스크립트 파일 확인...
if exist "init_git.bat" if exist "quick-start.bat" if exist "deploy.bat" (
    echo ✅ 모든 배치 스크립트 존재
    set /a PASS+=1
) else (
    echo ❌ 스크립트 파일 누락
    set /a FAIL+=1
)

REM 결과 출력
echo.
echo ═══════════════════════════════════════════════════════════
echo                      📊 테스트 결과
echo ═══════════════════════════════════════════════════════════
echo.
echo 성공: %PASS%/8
echo 실패: %FAIL%/8
echo.

if %FAIL% EQU 0 (
    echo ✅ 모든 테스트 통과! 환경이 준비되었습니다.
    echo.
    echo 다음 단계:
    echo   1. init_git.bat 실행
    echo   2. quick-start.bat 실행 (20-40분)
    echo.
) else (
    echo ⚠️  일부 필수 도구가 없습니다.
    echo.
    echo 설치 필요:
    if %ERRORLEVEL% NEQ 0 (
        echo   • Docker: https://www.docker.com/products/docker-desktop
        echo   • Rust: https://www.rust-lang.org/tools/install
        echo   • Git: https://git-scm.com/download/win
    )
)

echo.
echo ═══════════════════════════════════════════════════════════
echo.
pause
