# AI Agent Installer — Windows (PowerShell)
# Usage: irm https://raw.githubusercontent.com/USER/REPO/main/install.ps1 | iex

#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ─── Config ───────────────────────────────────────────────────────────────────
$REPO        = "SeongminJaden/swing-by"
$BINARY_NAME = "ai_agent"
$INSTALL_DIR = "$env:USERPROFILE\.local\bin"
$VERSION     = "latest"
$ARTIFACT    = "ai_agent-windows-x86_64.exe"

# ─── Colors ───────────────────────────────────────────────────────────────────
function Write-Info    { param($msg) Write-Host "[INFO] $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "[OK]   $msg" -ForegroundColor Green }
function Write-Warn    { param($msg) Write-Host "[!]    $msg" -ForegroundColor Yellow }
function Write-Err     { param($msg) Write-Host "[ERR]  $msg" -ForegroundColor Red; exit 1 }
function Ask           { param($prompt) Write-Host "[?]    $prompt" -ForegroundColor Magenta -NoNewline; return (Read-Host " ") }

# ─── Banner ───────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  +==========================================+" -ForegroundColor Cyan
Write-Host "  |     AI Agent Installer - Windows        |" -ForegroundColor Cyan
Write-Host "  |   Local LLM Multi-Agent System          |" -ForegroundColor Cyan
Write-Host "  +==========================================+" -ForegroundColor Cyan
Write-Host ""

# ─── Admin check ──────────────────────────────────────────────────────────────
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Warn "Not running as Administrator. Some installs may fail."
    Write-Warn "Re-run PowerShell as Administrator for full functionality."
}

# ─── Winget / Chocolatey check ────────────────────────────────────────────────
$useWinget = $false
$useChoco  = $false
if (Get-Command winget -ErrorAction SilentlyContinue) {
    $useWinget = $true
    Write-Success "winget available"
} elseif (Get-Command choco -ErrorAction SilentlyContinue) {
    $useChoco = $true
    Write-Success "Chocolatey available"
} else {
    Write-Warn "Neither winget nor Chocolatey found — manual installs may be needed"
}

# ─── Install helper ───────────────────────────────────────────────────────────
function Install-Optional {
    param($cmd, $wingetId, $chocoId, $desc, $url)
    if (Get-Command $cmd -ErrorAction SilentlyContinue) {
        Write-Success "$cmd already installed"
        return
    }
    $answer = Ask "Install $desc ($cmd)? [y/N]"
    if ($answer -ne 'y' -and $answer -ne 'Y') { Write-Warn "Skipping $cmd"; return }
    Write-Info "Installing $desc..."
    try {
        if ($useWinget -and $wingetId) {
            winget install --id $wingetId --accept-source-agreements --accept-package-agreements -e
        } elseif ($useChoco -and $chocoId) {
            choco install $chocoId -y
        } else {
            Write-Warn "Please install $desc manually: $url"
            return
        }
        Write-Success "$desc installed"
    } catch {
        Write-Warn "Failed to install $desc automatically. Please install from: $url"
    }
}

# ─── Optional tools ───────────────────────────────────────────────────────────
Write-Host ""
Write-Host "-- Optional Tool Installation ----------------------------" -ForegroundColor White

Install-Optional "git"    "Git.Git"              "git"    "Git"    "https://git-scm.com"
Install-Optional "docker" "Docker.DockerDesktop" "docker-desktop" "Docker Desktop" "https://docker.com"
Install-Optional "cargo"  $null                  "rust-ms" "Rust/Cargo" "https://rustup.rs"
Install-Optional "python" "Python.Python.3"      "python" "Python 3" "https://python.org"
Install-Optional "node"   "OpenJS.NodeJS.LTS"    "nodejs" "Node.js"  "https://nodejs.org"

# ─── Ollama ───────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "-- Ollama Installation -----------------------------------" -ForegroundColor White

$ollamaInstalled = Get-Command ollama -ErrorAction SilentlyContinue
if ($ollamaInstalled) {
    Write-Success "Ollama already installed"
} else {
    $answer = Ask "Install Ollama? [Y/n]"
    if ($answer -ne 'n' -and $answer -ne 'N') {
        Write-Info "Downloading Ollama installer..."
        $ollamaInstaller = "$env:TEMP\OllamaSetup.exe"
        Invoke-WebRequest "https://ollama.ai/download/OllamaSetup.exe" -OutFile $ollamaInstaller
        Write-Info "Running Ollama installer..."
        Start-Process $ollamaInstaller -Wait -ArgumentList "/SILENT"
        Remove-Item $ollamaInstaller -Force
        # Refresh PATH
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
        Write-Success "Ollama installed"
    } else {
        Write-Err "Ollama is required. Download from: https://ollama.ai"
    }
}

# Start Ollama if not running
try {
    $null = Invoke-RestMethod "http://localhost:11434/api/tags" -TimeoutSec 2
    Write-Success "Ollama server running"
} catch {
    Write-Info "Starting Ollama server..."
    Start-Process ollama -ArgumentList "serve" -WindowStyle Hidden
    Start-Sleep 4
    try {
        $null = Invoke-RestMethod "http://localhost:11434/api/tags" -TimeoutSec 2
        Write-Success "Ollama started"
    } catch {
        Write-Warn "Could not auto-start Ollama. Run 'ollama serve' in a terminal."
    }
}

# ─── Model selection ──────────────────────────────────────────────────────────
Write-Host ""
Write-Host "-- AI Model Selection ------------------------------------" -ForegroundColor White
Write-Host ""
Write-Host "  Gemma 4 (Recommended):" -ForegroundColor Cyan
Write-Host "   1) gemma4:e4b       - 8B  Q4, fastest,  ~5GB  [recommended]" -ForegroundColor Green
Write-Host "   2) gemma4:12b       - 12B Q4, better quality, ~7GB"
Write-Host "   3) gemma4:27b       - 27B Q4, best quality,   ~16GB"
Write-Host ""
Write-Host "  Alternatives:" -ForegroundColor Cyan
Write-Host "   4) llama3.2:latest  - Meta Llama 3.2 3B, ultra-light, ~2GB"
Write-Host "   5) llama3.1:latest  - Meta Llama 3.1 8B, ~5GB"
Write-Host "   6) codestral:latest - Code-focused, ~12GB"
Write-Host "   7) qwen2.5:7b       - Multilingual, ~5GB"
Write-Host "   8) Enter custom model name"
Write-Host "   9) Skip"
Write-Host ""

$MODEL = "gemma4:e4b"
while ($true) {
    $choice = Ask "Enter choice [1-9]"
    switch ($choice) {
        "1" { $MODEL = "gemma4:e4b";       break }
        "2" { $MODEL = "gemma4:12b";       break }
        "3" { $MODEL = "gemma4:27b";       break }
        "4" { $MODEL = "llama3.2:latest";  break }
        "5" { $MODEL = "llama3.1:latest";  break }
        "6" { $MODEL = "codestral:latest"; break }
        "7" { $MODEL = "qwen2.5:7b";       break }
        "8" { $MODEL = Ask "Enter model name (e.g. mistral:latest)"; if ($MODEL) { break } }
        "9" { $MODEL = ""; break }
        default { Write-Warn "Please enter 1-9" }
    }
    if ($choice -in "1","2","3","4","5","6","7","9" -or ($choice -eq "8" -and $MODEL)) { break }
}

if ($MODEL) {
    Write-Info "Pulling model: $MODEL (this may take a while...)"
    & ollama pull $MODEL
    Write-Success "Model '$MODEL' ready"
} else {
    $MODEL = (& ollama list 2>$null | Select-Object -Skip 1 -First 1).Split(" ")[0]
    if (-not $MODEL) { $MODEL = "gemma4:e4b" }
    Write-Warn "Using existing model: $MODEL"
}

# ─── Download binary ──────────────────────────────────────────────────────────
Write-Host ""
Write-Host "-- Installing AI Agent -----------------------------------" -ForegroundColor White

if ($VERSION -eq "latest") {
    $DOWNLOAD_URL = "https://github.com/$REPO/releases/latest/download/$ARTIFACT"
} else {
    $DOWNLOAD_URL = "https://github.com/$REPO/releases/download/$VERSION/$ARTIFACT"
}

# Create install dir
if (-not (Test-Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
}

$BIN_PATH = Join-Path $INSTALL_DIR "$BINARY_NAME.exe"
Write-Info "Downloading AI Agent binary..."
Write-Info "URL: $DOWNLOAD_URL"

try {
    Invoke-WebRequest $DOWNLOAD_URL -OutFile $BIN_PATH -UseBasicParsing
    Write-Success "Installed to $BIN_PATH"
} catch {
    Write-Err "Download failed. Visit: https://github.com/$REPO/releases"
}

# ─── Add to PATH ──────────────────────────────────────────────────────────────
$currentPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$INSTALL_DIR*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$currentPath;$INSTALL_DIR", "User")
    $env:Path += ";$INSTALL_DIR"
    Write-Success "Added $INSTALL_DIR to PATH"
} else {
    Write-Success "$INSTALL_DIR already in PATH"
}

# ─── Environment variables ────────────────────────────────────────────────────
Write-Host ""
Write-Host "-- Environment Setup -------------------------------------" -ForegroundColor White

[System.Environment]::SetEnvironmentVariable("OLLAMA_API_URL", "http://localhost:11434", "User")
[System.Environment]::SetEnvironmentVariable("OLLAMA_MODEL",   $MODEL,                   "User")
$env:OLLAMA_API_URL = "http://localhost:11434"
$env:OLLAMA_MODEL   = $MODEL

Write-Success "Environment variables set (OLLAMA_MODEL=$MODEL)"

# ─── Desktop IDE (optional) ───────────────────────────────────────────────────
Write-Host ""
Write-Host "-- Desktop IDE (optional) --------------------------------" -ForegroundColor White
Write-Host ""
Write-Host "  Swing-by IDE is a GUI desktop app for the multi-agent pipeline." -ForegroundColor Gray
Write-Host ""
$ideAnswer = Ask "Install Swing-by Desktop IDE? [y/N]"
$ideInstalled = $false
if ($ideAnswer -eq 'y' -or $ideAnswer -eq 'Y') {
    $IDE_URL  = "https://github.com/$REPO/releases/latest/download/swing-by-ide-windows-setup.exe"
    $IDE_PATH = "$env:TEMP\swing-by-ide-setup.exe"
    Write-Info "Downloading Swing-by IDE..."
    try {
        Invoke-WebRequest $IDE_URL -OutFile $IDE_PATH -UseBasicParsing
        Write-Info "Running installer (silent)..."
        Start-Process $IDE_PATH -Wait -ArgumentList "/SILENT", "/NORESTART"
        Remove-Item $IDE_PATH -Force -ErrorAction SilentlyContinue
        Write-Success "Swing-by IDE installed"
        $ideInstalled = $true
    } catch {
        Write-Warn "IDE download failed. Download from: https://github.com/$REPO/releases"
    }
} else {
    Write-Info "Skipping Desktop IDE"
}

# ─── Verify ───────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "-- Verification ------------------------------------------" -ForegroundColor White

try {
    $ver = & $BIN_PATH --version 2>&1
    Write-Success "AI Agent: $ver"
} catch {
    Write-Warn "Binary installed but could not verify. Try: $BINARY_NAME --version"
}

# ─── Done ─────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  +==========================================+" -ForegroundColor Green
Write-Host "  |        Installation Complete!           |" -ForegroundColor Green
Write-Host "  +==========================================+" -ForegroundColor Green
Write-Host ""
Write-Host "  Model   : $MODEL" -ForegroundColor Cyan
Write-Host "  Binary  : $BIN_PATH" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Quick start (restart terminal first):" -ForegroundColor White
Write-Host "    $BINARY_NAME                              # start chat" -ForegroundColor Green
Write-Host "    $BINARY_NAME --help                       # show options" -ForegroundColor Green
Write-Host "    $BINARY_NAME --agile `"Build a REST API`"  # agile sprint" -ForegroundColor Green
if ($ideInstalled) {
    Write-Host ""
    Write-Host "  Desktop IDE:" -ForegroundColor White
    Write-Host "    Search 'Swing-by IDE' in Start Menu" -ForegroundColor Green
}
Write-Host ""
