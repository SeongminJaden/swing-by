#!/usr/bin/env bash
# AI Agent Installer — Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/USER/REPO/main/install.sh | bash

set -euo pipefail

# ─── Config ───────────────────────────────────────────────────────────────────
REPO="SeongminJaden/swing-by"
BINARY_NAME="ai_agent"
INSTALL_DIR="/usr/local/bin"
VERSION="latest"

# ─── Colors ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { echo -e "${BLUE}[INFO]${RESET} $*"; }
success() { echo -e "${GREEN}[✓]${RESET} $*"; }
warn()    { echo -e "${YELLOW}[!]${RESET} $*"; }
error()   { echo -e "${RED}[✗]${RESET} $*"; exit 1; }
ask()     { echo -e "${CYAN}[?]${RESET} $*"; }

# ─── Banner ───────────────────────────────────────────────────────────────────
echo -e "${BOLD}"
echo "  ╔══════════════════════════════════════════╗"
echo "  ║      AI Agent Installer — Linux         ║"
echo "  ║   Local LLM Multi-Agent System          ║"
echo "  ╚══════════════════════════════════════════╝"
echo -e "${RESET}"

# ─── Detect OS & Arch ─────────────────────────────────────────────────────────
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64)  ARCH_LABEL="x86_64" ;;
  aarch64|arm64) ARCH_LABEL="arm64" ;;
  *) error "Unsupported architecture: $ARCH" ;;
esac

ARTIFACT="${BINARY_NAME}-linux-${ARCH_LABEL}"
info "Detected: Linux ${ARCH_LABEL}"

# ─── Dependency check ─────────────────────────────────────────────────────────
need_cmd() {
  command -v "$1" &>/dev/null || return 1
}

# ─── Optional tools ───────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}── Optional Tool Installation ──────────────────────${RESET}"

install_if_wanted() {
  local tool="$1" install_cmd="$2" desc="$3"
  if need_cmd "$tool"; then
    success "$tool already installed"
    return
  fi
  ask "Install ${desc} (${tool})? [y/N]"
  read -r answer < /dev/tty
  if [[ "${answer,,}" == "y" ]]; then
    info "Installing $tool..."
    eval "$install_cmd"
    success "$tool installed"
  else
    warn "Skipping $tool"
  fi
}

# Package manager detection
if need_cmd apt-get; then
  PKG_UPDATE="sudo apt-get update -qq"
  PKG_INSTALL="sudo apt-get install -y"
elif need_cmd dnf; then
  PKG_UPDATE="sudo dnf check-update -q || true"
  PKG_INSTALL="sudo dnf install -y"
elif need_cmd pacman; then
  PKG_UPDATE="sudo pacman -Sy --noconfirm"
  PKG_INSTALL="sudo pacman -S --noconfirm"
else
  PKG_UPDATE="true"
  PKG_INSTALL="echo 'Please install manually:'"
fi

install_if_wanted "git"    "$PKG_INSTALL git"         "Git version control"
install_if_wanted "docker" "curl -fsSL https://get.docker.com | sh && sudo usermod -aG docker \$USER" "Docker container runtime"
install_if_wanted "cargo"  "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source \$HOME/.cargo/env" "Rust / Cargo"
install_if_wanted "python3" "$PKG_INSTALL python3 python3-pip" "Python 3"
install_if_wanted "node"   "curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash - && $PKG_INSTALL nodejs" "Node.js"

# ─── Ollama ───────────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}── Ollama Installation ─────────────────────────────${RESET}"

if need_cmd ollama; then
  success "Ollama already installed ($(ollama --version 2>/dev/null || echo 'unknown version'))"
else
  ask "Install Ollama? [Y/n]"
  read -r answer < /dev/tty
  if [[ "${answer,,}" != "n" ]]; then
    info "Installing Ollama..."
    curl -fsSL https://ollama.ai/install.sh | sh
    success "Ollama installed"
  else
    error "Ollama is required to run AI Agent. Please install it manually: https://ollama.ai"
  fi
fi

# Start Ollama if not running
if ! curl -sf http://localhost:11434/api/tags &>/dev/null; then
  info "Starting Ollama server..."
  ollama serve &>/dev/null &
  sleep 3
  if curl -sf http://localhost:11434/api/tags &>/dev/null; then
    success "Ollama server started"
  else
    warn "Could not start Ollama automatically. Run 'ollama serve' in a separate terminal."
  fi
fi

# ─── Model selection ──────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}── AI Model Selection ──────────────────────────────${RESET}"
echo ""
echo -e "  Select the model to install:"
echo ""
echo -e "  ${CYAN}Gemma 4 (Recommended)${RESET}"
echo -e "   1) gemma4:e4b       — 8B  Q4, fastest, ~5GB  ${GREEN}[recommended]${RESET}"
echo -e "   2) gemma4:12b       — 12B Q4, better quality, ~7GB"
echo -e "   3) gemma4:27b       — 27B Q4, best quality,  ~16GB"
echo ""
echo -e "  ${CYAN}Alternatives${RESET}"
echo -e "   4) llama3.2:latest  — Meta Llama 3.2 3B, ultra-light, ~2GB"
echo -e "   5) llama3.1:latest  — Meta Llama 3.1 8B, general purpose, ~5GB"
echo -e "   6) codestral:latest — Mistral code model, code-focused, ~12GB"
echo -e "   7) qwen2.5:7b       — Alibaba Qwen 2.5 7B, multilingual, ~5GB"
echo -e "   8) Enter custom model name"
echo -e "   9) Skip (use already installed model)"
echo ""

while true; do
  ask "Enter choice [1-9]:"
  read -r choice < /dev/tty
  case "$choice" in
    1) MODEL="gemma4:e4b";       break ;;
    2) MODEL="gemma4:12b";       break ;;
    3) MODEL="gemma4:27b";       break ;;
    4) MODEL="llama3.2:latest";  break ;;
    5) MODEL="llama3.1:latest";  break ;;
    6) MODEL="codestral:latest"; break ;;
    7) MODEL="qwen2.5:7b";       break ;;
    8)
      ask "Enter model name (e.g. mistral:latest):"
      read -r MODEL < /dev/tty
      [[ -n "$MODEL" ]] && break
      warn "Model name cannot be empty"
      ;;
    9) MODEL=""; break ;;
    *) warn "Please enter 1-9" ;;
  esac
done

if [[ -n "$MODEL" ]]; then
  info "Pulling model: $MODEL (this may take a while...)"
  ollama pull "$MODEL"
  success "Model '$MODEL' ready"
else
  info "Skipping model download"
  # Try to detect already-installed model
  MODEL=$(ollama list 2>/dev/null | awk 'NR>1 {print $1}' | head -1 || echo "gemma4:e4b")
  warn "Using existing model: $MODEL"
fi

# ─── Download ai_agent binary ─────────────────────────────────────────────────
echo ""
echo -e "${BOLD}── Installing AI Agent ─────────────────────────────${RESET}"

if [[ "$VERSION" == "latest" ]]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ARTIFACT}"
else
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"
fi

info "Downloading AI Agent binary..."
info "URL: $DOWNLOAD_URL"

TMP_BIN=$(mktemp)
if curl -fSL --progress-bar "$DOWNLOAD_URL" -o "$TMP_BIN"; then
  chmod +x "$TMP_BIN"
  if [[ -w "$INSTALL_DIR" ]]; then
    mv "$TMP_BIN" "${INSTALL_DIR}/${BINARY_NAME}"
  else
    sudo mv "$TMP_BIN" "${INSTALL_DIR}/${BINARY_NAME}"
  fi
  success "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
else
  rm -f "$TMP_BIN"
  error "Download failed. Check your internet connection or visit: https://github.com/${REPO}/releases"
fi

# ─── Environment variables ────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}── Environment Setup ───────────────────────────────${RESET}"

SHELL_RC=""
if [[ -n "${BASH_VERSION:-}" ]] || [[ "$SHELL" == */bash ]]; then
  SHELL_RC="$HOME/.bashrc"
elif [[ -n "${ZSH_VERSION:-}" ]] || [[ "$SHELL" == */zsh ]]; then
  SHELL_RC="$HOME/.zshrc"
else
  SHELL_RC="$HOME/.profile"
fi

ENV_BLOCK=$(cat <<EOF

# ── AI Agent ──────────────────────────────────────────────
export OLLAMA_API_URL="http://localhost:11434"
export OLLAMA_MODEL="${MODEL:-gemma4:e4b}"
# export DISCORD_TOKEN=""   # Uncomment for Discord bot mode
# ──────────────────────────────────────────────────────────
EOF
)

if grep -q "OLLAMA_MODEL" "$SHELL_RC" 2>/dev/null; then
  warn "Environment variables already set in $SHELL_RC — skipping"
else
  echo "$ENV_BLOCK" >> "$SHELL_RC"
  success "Environment variables added to $SHELL_RC"
fi

# Also set for current session
export OLLAMA_API_URL="http://localhost:11434"
export OLLAMA_MODEL="${MODEL:-gemma4:e4b}"

# ─── Verify installation ──────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}── Verification ────────────────────────────────────${RESET}"

if "${INSTALL_DIR}/${BINARY_NAME}" --version &>/dev/null; then
  success "AI Agent $("${INSTALL_DIR}/${BINARY_NAME}" --version)"
else
  warn "Binary installed but could not verify. Try: ${BINARY_NAME} --version"
fi

# ─── Done ─────────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}${BOLD}"
echo "  ╔══════════════════════════════════════════╗"
echo "  ║        Installation Complete! 🎉        ║"
echo "  ╚══════════════════════════════════════════╝"
echo -e "${RESET}"
echo -e "  Model    : ${CYAN}${MODEL:-gemma4:e4b}${RESET}"
echo -e "  Binary   : ${CYAN}${INSTALL_DIR}/${BINARY_NAME}${RESET}"
echo -e "  Config   : ${CYAN}${SHELL_RC}${RESET}"
echo ""
echo -e "  ${BOLD}Quick start:${RESET}"
echo -e "    ${GREEN}source ${SHELL_RC}${RESET}   # reload shell env"
echo -e "    ${GREEN}${BINARY_NAME}${RESET}           # start interactive chat"
echo -e "    ${GREEN}${BINARY_NAME} --help${RESET}    # show all options"
echo ""
echo -e "  ${BOLD}Agile sprint:${RESET}"
echo -e "    ${GREEN}${BINARY_NAME} --agile \"Build a REST API\" --project myapp${RESET}"
echo ""
