@echo off
echo [*] Starting Git initialization and commit process...

cd /d C:\git

REM Check if git is installed
where git >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Git is not installed. Please install Git from https://git-scm.com/download/win
    pause
    exit /b 1
)

echo [*] Git is installed, proceeding...

REM Check if .git exists
if exist .git (
    echo [*] Git repository already exists
) else (
    echo [*] Initializing new Git repository...
    git init
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to initialize git repository
        pause
        exit /b 1
    )
)

REM Configure git user
echo [*] Configuring Git user...
git config user.email "bot@example.com"
git config user.name "Copilot Bot"

REM Add all files
echo [*] Adding all files to staging area...
git add -A
if %errorlevel% neq 0 (
    echo [ERROR] Failed to add files
    pause
    exit /b 1
)

REM Show status
echo.
echo [*] Current git status:
git status
echo.

REM Commit
echo [*] Creating initial commit...
git commit -m "Initial commit: Docker + Ollama + Gemma + Rust AI Agent

- Complete project scaffold with 38 files
- Docker Compose for Ollama service with Gemma model
- Rust project with 10 dependencies (tokio, reqwest, serde, etc)
- Automation scripts for setup, deployment, and testing
- Comprehensive documentation (21 guides)
- VSCode debugging configuration with LLDB
- GitHub Actions CI/CD pipeline
- System validation and testing tools
- Development environment setup scripts

Project Structure:
  - docker-compose.yml: Ollama service on port 11434
  - Cargo.toml: Complete Rust dependencies
  - main.rs: Basic entry point
  - Automation: init_git.bat, quick-start.bat, deploy.bat
  - Documentation: 21 comprehensive guides
  - Configuration: VSCode, Git, GitHub Actions

Status: Ready for production deployment

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"

if %errorlevel% neq 0 (
    echo [ERROR] Failed to commit
    pause
    exit /b 1
)

REM Show commit log
echo.
echo [*] Commit successful! Here's the log:
git log --oneline -5
echo.

REM Check for remote
echo [*] Checking for remote repository...
git remote -v >nul 2>&1
if %errorlevel% equ 0 (
    echo [*] Remote repository found, attempting push...
    
    REM Try to determine default branch
    for /f "tokens=*" %%i in ('git rev-parse --abbrev-ref HEAD') do set BRANCH=%%i
    echo [*] Current branch: %BRANCH%
    
    git push origin %BRANCH% 2>&1
    if %errorlevel% equ 0 (
        echo [SUCCESS] Push successful!
    ) else (
        echo [WARNING] Push failed - check remote configuration
    )
) else (
    echo [INFO] No remote repository configured
    echo [INFO] To push later, configure remote with:
    echo [INFO] git remote add origin ^<your-repo-url^>
    echo [INFO] git push -u origin master
)

echo.
echo [SUCCESS] Git commit completed!
echo.
pause
