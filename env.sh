#!/usr/bin/env bash
# 개발 환경 PATH 설정 스크립트
# 사용법: source env.sh

MSVC_BIN="/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64"
CARGO_BIN="/c/Users/Administrator/.cargo/bin"
OLLAMA_BIN="/c/Users/Administrator/AppData/Local/Programs/Ollama"

export PATH="$MSVC_BIN:$CARGO_BIN:$OLLAMA_BIN:$PATH"
export OLLAMA_MODEL="gemma4:e4b"
export OLLAMA_API_URL="http://localhost:11434"
export RUST_LOG="info"

echo "환경 설정 완료:"
echo "  cargo: $(cargo --version 2>/dev/null || echo '없음')"
echo "  ollama: $(ollama --version 2>/dev/null || echo '없음')"
echo "  모델: $OLLAMA_MODEL"
