@echo off
REM 배포 스크립트 - 프로덕션 빌드 및 배포 (Windows)

setlocal enabledelayedexpansion

echo.
echo 🚀 배포 시작...
echo.

REM 변수 정의
set PROJECT_NAME=ai_agent
set BUILD_DIR=target\release
set DIST_DIR=dist

REM 버전 읽기
for /f "tokens=*" %%i in ('findstr version Cargo.toml ^| findstr /v "^REM"') do (
    set CARGO_LINE=%%i
    goto found_version
)
:found_version
set VERSION=0.1.0

echo 프로젝트: %PROJECT_NAME%
echo 버전: %VERSION%
echo.

REM Step 1: 테스트
echo 📋 [1/4] 테스트 실행...
cargo test --release 2>&1 | findstr "test result running"
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ❌ 테스트 실패
    pause
    exit /b 1
)
echo ✅ 테스트 완료
echo.

REM Step 2: 린트
echo 🔍 [2/4] 코드 품질 확인...
cargo clippy --release 2>&1 | findstr "warning error" || echo (경고 없음)
echo ✅ 린트 완료
echo.

REM Step 3: 빌드
echo 🔨 [3/4] 최적화 빌드...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ❌ 빌드 실패
    pause
    exit /b 1
)
echo ✅ 빌드 완료
echo.

REM Step 4: 배포 패키지
echo 📦 [4/4] 배포 패키지 생성...
if exist %DIST_DIR% rmdir /s /q %DIST_DIR%
mkdir %DIST_DIR%

REM 바이너리 복사
copy %BUILD_DIR%\%PROJECT_NAME%.exe %DIST_DIR%\ >nul

REM 설정 파일 복사
copy docker-compose.yml %DIST_DIR%\ >nul
copy Cargo.toml %DIST_DIR%\ >nul
copy README.md %DIST_DIR%\ >nul
copy QUICK_START.md %DIST_DIR%\ >nul

REM 배포 정보 생성
(
    echo =================================
    echo %PROJECT_NAME% v%VERSION%
    echo 배포 패키지
    echo =================================
    echo.
    echo 빌드 시간: %date% %time%
    echo 바이너리: .\%PROJECT_NAME%.exe
    echo.
    echo 설치 및 실행:
    echo 1. dist\ 폴더 이동
    echo 2. %PROJECT_NAME%.exe 실행
    echo 3. 또는 docker-compose 사용
    echo.
    echo 포함된 파일:
    echo - %PROJECT_NAME%.exe
    echo - docker-compose.yml
    echo - README.md
    echo - QUICK_START.md
    echo.
    echo 필수 요구사항:
    echo - Docker ^& Docker Compose
    echo - 8GB+ 메모리
    echo - 포트 11434 사용 가능
    echo =================================
) > %DIST_DIR%\DEPLOY_INFO.txt

echo ✅ 배포 패키지 생성 완료
echo.

echo ════════════════════════════════════════
echo 🎉 배포 준비 완료!
echo ════════════════════════════════════════
echo.
echo 패키지 위치: .\%DIST_DIR%\
echo 바이너리: .\%DIST_DIR%\%PROJECT_NAME%.exe
echo.
echo 배포 방법:
echo   전체 dist\ 폴더를 대상 경로에 복사
echo.
pause
