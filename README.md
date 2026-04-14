# Swing-by — Local LLM Multi-Agent System

> A fully local, privacy-first AI agent built on **Ollama + Gemma** (or any compatible model).  
> 13 specialized AI roles, parallel tool execution, RAG codebase indexing, GitHub PR automation, and AI-to-AI communication — all running on your machine.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Ollama](https://img.shields.io/badge/ollama-compatible-blue)](https://ollama.ai)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow)](LICENSE)

---

## Installation

### Linux
```bash
curl -fsSL https://raw.githubusercontent.com/SeongminJaden/swing-by/main/install.sh | bash
```

### macOS
```bash
curl -fsSL https://raw.githubusercontent.com/SeongminJaden/swing-by/main/install_mac.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/SeongminJaden/swing-by/main/install.ps1 | iex
```

The installer interactively guides you through:

1. Optional tool installation (git, Docker, Python, Node.js) — **yes/no prompts**
2. Ollama installation
3. AI model selection from a menu
4. Binary download & PATH setup
5. Environment variable configuration

### Model Selection Menu
```
  Gemma 4 (Recommended)
   1) gemma4:e4b       — 8B  Q4, fastest,  ~5GB  [recommended]
   2) gemma4:12b       — 12B Q4, better quality, ~7GB
   3) gemma4:27b       — 27B Q4, best quality,   ~16GB

  Alternatives
   4) llama3.2:latest  — Meta Llama 3.2 3B, ultra-light, ~2GB
   5) llama3.1:latest  — Meta Llama 3.1 8B, general purpose, ~5GB
   6) codestral:latest — Code-focused, ~12GB
   7) qwen2.5:7b       — Multilingual, ~5GB
   8) Enter custom model name
```

### Manual Install
Download the binary directly from the [Releases page](https://github.com/SeongminJaden/swing-by/releases/latest):

| Platform | File |
|----------|------|
| Linux x86_64 | `ai_agent-linux-x86_64` |
| Linux ARM64 | `ai_agent-linux-arm64` |
| macOS Intel | `ai_agent-macos-x86_64` |
| macOS Apple Silicon | `ai_agent-macos-arm64` |
| Windows x86_64 | `ai_agent-windows-x86_64.exe` |

```bash
# Linux / macOS example
chmod +x ai_agent-linux-x86_64
sudo mv ai_agent-linux-x86_64 /usr/local/bin/ai_agent
```

---

## Quick Start

```bash
# Interactive chat
ai_agent

# Single prompt (non-interactive)
ai_agent --print "Explain this codebase"

# Full Agile sprint
ai_agent --agile "Build a REST API for user auth" --project myapp

# Resume previous session
ai_agent --resume
```

### Environment Variables
```bash
OLLAMA_API_URL=http://localhost:11434   # Ollama server (default)
OLLAMA_MODEL=gemma4:e4b                 # Model to use (default)
DISCORD_TOKEN=...                       # Required for --discord mode
```

---

## Features

### Core Agent
- **Streaming chat** — real-time token-by-token output
- **Multi-turn tool use** — read/write files, run shell commands, web search, code execution
- **Session persistence** — auto-save/restore chat history (`--resume`)
- **Context compression** — automatic history compaction near token limits
- **CLAUDE.md support** — auto-loads project + global instructions

### Agile Multi-Agent Pipeline
13 specialized AI roles collaborate across the full software development lifecycle:

| Role | Responsibility |
|------|----------------|
| 📦 ProductOwner | Requirements → User Stories |
| 🏃 ScrumMaster | Sprint planning & impediment removal |
| 📊 BusinessAnalyst | Stakeholder analysis, ROI, KPI |
| 🎨 UXDesigner | Personas, wireframes, accessibility |
| 🏛️ Architect | System design, tech stack, SOLID |
| 💻 Developer | TDD implementation, OWASP compliance |
| 🔬 QAEngineer | Test cases, bug reports, regression |
| 👁️ Reviewer | Code review, security, performance scoring |
| 🎯 TechLead | Gate review, ADR, release approval |
| 🚀 DevOpsEngineer | CI/CD, Docker, Kubernetes, IaC |
| 📝 TechnicalWriter | README, API docs, changelogs |
| 📡 SRE | SLO/SLI, runbooks, chaos engineering |
| 🎁 ReleaseManager | Release notes, SemVer, rollback plans |

### Coordinator Multi-Agent Mode
- Leader decomposes complex tasks into independent subtasks
- Up to **8 workers run in parallel** via `tokio::spawn`
- Real-time progress streaming via `mpsc` channel
- Leader synthesizes all results into a final output

### Parallel Tool Execution
Read-only tools run concurrently; write tools always run sequentially for safety.

### RAG Codebase Indexing
- Indexes project files into 800-char chunks with TF-IDF scoring
- No external vector DB required
- Inject relevant code context into any query with `/rag query`

### GitHub PR Automation
- AI-generated PR title, description, and test plan
- Sprint-aware: auto-creates PRs from release notes

### Sprint Checkpointing
- Progress auto-saved after each story
- Resume interrupted sprints: `/agile checkpoint resume`
- Reports saved to `sprint-report-{id}-{timestamp}.md`

### Per-Role Model Assignment
```toml
# config.toml
[roles]
architect = "llama3.2:latest"
developer = "codestral:latest"
qa_engineer = "gemma4:e4b"
```

---

## Slash Commands

```
/help                        Show all commands
/agile <task>                Run full Agile sprint (13 roles)
/agile checkpoint resume     Resume an interrupted sprint
/coordinator <task>          Parallel multi-agent decomposition
/retro                       Sprint retrospective (KPT format)
/postmortem <incident>       Incident analysis → fix → runbook
/techdebt [path]             Technical debt analysis
/security [path]             Security audit
/ba <topic>                  Business analysis
/ux <topic>                  UX/UI design
/devops [path]               DevOps & infrastructure
/docs [path]                 Documentation generation
/sre [path]                  SRE analysis
/rag index [path]            Index codebase for RAG
/rag query <question>        Query indexed codebase
/rag status                  Show RAG index stats
/pr list                     List open GitHub PRs
/pr create [title]           Create PR with AI-generated description
/pipeline <task>             Multi-agent pipeline (plan→impl→review)
/save / /load                Save / restore session
/memory save <note>          Add persistent memory
/compact                     Manually compact context
/config                      Show current configuration
/ipc [port]                  Start AI-to-AI HTTP server
```

---

## AI-to-AI Communication

```bash
# stdio JSON-RPC (Claude Code or other agents)
echo '{"jsonrpc":"2.0","id":1,"method":"chat","params":{"prompt":"hello"}}' \
  | ai_agent --ipc-stdio

# HTTP server mode
ai_agent --ipc-server 8765
curl -X POST http://localhost:8765 \
  -H 'Content-Type: application/json' \
  -H 'X-Caller-ID: claude-code' \
  -d '{"jsonrpc":"2.0","id":1,"method":"agile_sprint","params":{"project":"myapp","request":"Add login"}}'
```

---

## Available Tools (50+)

| Category | Tools |
|----------|-------|
| **File System** | `read_file`, `write_file`, `edit_file`, `list_dir`, `glob`, `grep`, `delete_file` |
| **Code** | `run_code` (Python/JS/Go/Rust), `run_shell`, `run_tests` |
| **Git** | `git_status`, `git_diff`, `git_log`, `git_commit`, `git_push`, `git_branch` |
| **Web** | `web_fetch`, `web_search`, `research`, `docs_fetch` |
| **Docker** | `docker_ps`, `docker_images`, `docker_exec`, `docker_logs`, `docker_build` |
| **System** | `sysinfo`, `process_list`, `env_list`, `get_env`, `current_dir` |
| **Packages** | `pkg_info`, `pkg_versions`, `pkg_search` |
| **Database** | `db_query`, `db_schema` |

---

## License

MIT

---

## References

- [Ollama](https://ollama.ai) — Local LLM inference
- [Google Gemma](https://ai.google.dev/gemma) — Default model
- [Claude Code](https://claude.ai/code) — Parallel execution & coordinator pattern inspiration
