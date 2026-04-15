# Changelog

## [0.1.0-alpha] - 2026-03-18

### Added

#### Backend Services (15)
- **Auth**: Email login/register + social OAuth (Google, GitHub, Apple)
- **Supabase**: Cloud DB integration (profiles, projects, subscriptions)
- **Connections**: Encrypted credential storage (AES-256-GCM) for 20 services
- **Payment**: Real Stripe integration (Checkout, Customer Portal, webhooks)
- **AI**: Claude/OpenAI real-time streaming chat
- **File System**: Local file read/write/delete with tilde path resolution
- **Terminal**: node-pty based real terminal emulator
- **Git**: simple-git version control (status, commit, push, branch)
- **Security**: Code security scanning (secrets, XSS, eval, dependencies)
- **Deploy**: Vercel, Netlify, Cloudflare Pages, Railway deployment
- **Monitoring**: URL uptime monitoring with alerts
- **Error Tracking**: Error capture and reporting
- **Cost Tracking**: AI usage cost tracking with budget alerts
- **Team**: Team creation, invitation, member management
- **Updater**: electron-updater auto-update with GitHub Releases

#### Frontend Pages (8)
- Login (email + social) / Pricing / Onboarding / Dashboard
- IDE (VS Code layout) / Settings / New Service / Watchdog

#### Service Integrations (20 with real webview + guide)
- AI: Claude, OpenAI, Gemini, Ollama
- Git: GitHub, GitLab, Bitbucket
- Deploy: Vercel, Railway, Netlify, Cloudflare, AWS
- DB: Supabase, Firebase
- Payment: Stripe, TossPayments
- Notification: Slack, Discord
- App Store: Apple Developer, Google Developer

#### IDE Features
- Monaco Editor with syntax highlighting
- VS Code style layout (sidebar + editor + bottom panel + chat)
- Real-time AI chat panel (Claude/OpenAI streaming)
- xterm.js terminal
- File explorer tree view
- Git source control panel
- Debug console (Console/Network/Problems/Terminal)
- Preview panel with responsive viewport

#### Infrastructure
- React.lazy code splitting (main bundle 397KB)
- TypeScript strict mode (0 errors)
- i18n support (Korean + English)
- Dark / Light / Monokai themes
- electron-builder packaging (Linux/Mac/Windows)
- Landing page (static HTML)
- Supabase DB schema with RLS policies

### Architecture
- Electron 41 + React 19 + TypeScript 5.9
- 90+ IPC handlers via preload bridge
- Zustand state management
- AES-256-GCM encrypted credential storage
- OAuth with BrowserWindow popup + localhost callback server
- Supabase Auth with local file-based fallback
