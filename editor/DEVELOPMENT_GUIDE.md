# VidEplace 개발 가이드

> **"Write the idea. We build, verify, and ship it."**
>
> PRD를 작성하면 AI가 코드를 생성하고, 자동 보안/품질 검증 후 원클릭 출시까지 해주는 올인원 Electron IDE

---

## 목차

1. [프로젝트 아키텍처](#1-프로젝트-아키텍처)
2. [전체 기능 명세](#2-전체-기능-명세)
3. [Phase별 개발 계획](#3-phase별-개발-계획)
4. [API 설계](#4-api-설계)
5. [데이터 모델](#5-데이터-모델)
6. [보안 고려사항](#6-보안-고려사항)

---

## 1. 프로젝트 아키텍처

### 1.1 Electron 프로세스 구조도

```
┌─────────────────────────────────────────────────────────────────┐
│                        Electron App                             │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Main Process (Node.js)                  │   │
│  │                                                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐     │   │
│  │  │ 파일시스템    │  │ AI 프로바이더 │  │ Git 서비스    │     │   │
│  │  │ (Node fs)    │  │ (HTTP 직접)  │  │ (simple-git) │     │   │
│  │  └─────────────┘  └─────────────┘  └──────────────┘     │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐     │   │
│  │  │ 터미널 관리   │  │ 출시 서비스   │  │ 인증/키체인   │     │   │
│  │  │ (node-pty)   │  │ (Platform   │  │ (keytar)     │     │   │
│  │  │              │  │  Adapters)  │  │              │     │   │
│  │  └─────────────┘  └─────────────┘  └──────────────┘     │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐     │   │
│  │  │ 보안 스캐너  │  │ 모니터링     │  │ BrowserView  │     │   │
│  │  │ (ESLint,    │  │ (WebSocket) │  │ 관리자        │     │   │
│  │  │  Semgrep)   │  │             │  │              │     │   │
│  │  └─────────────┘  └─────────────┘  └──────────────┘     │   │
│  └──────────────────────────┬───────────────────────────────┘   │
│                             │ IPC (ipcMain ↔ ipcRenderer)       │
│  ┌──────────────────────────┼───────────────────────────────┐   │
│  │                  Preload Script                           │   │
│  │         contextBridge.exposeInMainWorld('api', {          │   │
│  │           fs, ai, git, deploy, auth, terminal, ...       │   │
│  │         })                                                │   │
│  └──────────────────────────┬───────────────────────────────┘   │
│                             │ window.api.*                       │
│  ┌──────────────────────────┼───────────────────────────────┐   │
│  │              Renderer Process (Chromium)                   │   │
│  │                                                          │   │
│  │  ┌─────────────────────────────────────────────────┐     │   │
│  │  │              React + TypeScript App              │     │   │
│  │  │                                                 │     │   │
│  │  │  ┌──────────┐ ┌───────────┐ ┌──────────────┐  │     │   │
│  │  │  │ 파일탐색기 │ │ Monaco    │ │ AI 채팅 패널  │  │     │   │
│  │  │  │          │ │ Editor    │ │              │  │     │   │
│  │  │  └──────────┘ └───────────┘ └──────────────┘  │     │   │
│  │  │  ┌──────────┐ ┌───────────┐ ┌──────────────┐  │     │   │
│  │  │  │ 미리보기   │ │ 디버그    │ │ 소스 컨트롤   │  │     │   │
│  │  │  │ 패널     │ │ 콘솔      │ │              │  │     │   │
│  │  │  └──────────┘ └───────────┘ └──────────────┘  │     │   │
│  │  │                                                 │     │   │
│  │  │  Zustand (전역 상태) + Tailwind CSS (스타일링)   │     │   │
│  │  └─────────────────────────────────────────────────┘     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                BrowserView 인스턴스들                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐     │   │
│  │  │ 미리보기     │  │ OAuth 모달   │  │ 추가 뷰      │     │   │
│  │  │ (localhost)  │  │ (github.com │  │ (필요 시)    │     │   │
│  │  │             │  │  등 서비스)  │  │              │     │   │
│  │  └─────────────┘  └─────────────┘  └──────────────┘     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 모듈간 통신 방식 (IPC)

```
Renderer Process                    Main Process
─────────────────                   ─────────────

[React Component]                   [IPC Handler]
      │                                   │
      │  ipcRenderer.invoke(channel, args) │
      │ ──────────────────────────────────>│
      │                                   │
      │       (async 처리 후 결과 반환)      │
      │ <──────────────────────────────────│
      │                                   │

[Zustand Store]                     [Service Layer]
      │                                   │
      │  ipcRenderer.on(channel, callback) │
      │ <──────────── 이벤트 푸시 ──────────│
      │  (AI 스트리밍, 로그, 모니터링 등)    │
```

**통신 패턴:**

| 패턴 | 용도 | 구현 |
|------|------|------|
| Request-Response | 파일 읽기/쓰기, Git 명령, 설정 조회 | `ipcRenderer.invoke()` / `ipcMain.handle()` |
| One-way (R→M) | 사용자 액션 알림 | `ipcRenderer.send()` / `ipcMain.on()` |
| One-way (M→R) | AI 스트리밍, 실시간 로그, 모니터링 데이터 | `mainWindow.webContents.send()` / `ipcRenderer.on()` |
| Broadcast | 테마 변경, 설정 업데이트 | `BrowserWindow.getAllWindows().forEach(w => w.webContents.send())` |

### 1.3 디렉토리 구조

```
videplace/
├── package.json
├── electron-builder.yml            # electron-builder 빌드 설정
├── tsconfig.json                   # 루트 TypeScript 설정
├── tailwind.config.ts
├── vite.config.ts                  # Renderer용 Vite 설정
│
├── src/
│   ├── main/                       # ── Electron Main Process ──
│   │   ├── index.ts                # 엔트리포인트, BrowserWindow 생성
│   │   ├── ipc/                    # IPC 핸들러 등록
│   │   │   ├── index.ts            # 모든 IPC 핸들러 등록 진입점
│   │   │   ├── fs.ipc.ts           # 파일시스템 IPC
│   │   │   ├── ai.ipc.ts           # AI 프로바이더 IPC
│   │   │   ├── git.ipc.ts          # Git/GitHub IPC
│   │   │   ├── deploy.ipc.ts       # 출시 IPC
│   │   │   ├── auth.ipc.ts         # 인증 IPC
│   │   │   ├── terminal.ipc.ts     # 터미널 IPC
│   │   │   ├── security.ipc.ts     # 보안 스캔 IPC
│   │   │   ├── monitoring.ipc.ts   # 모니터링 IPC
│   │   │   └── settings.ipc.ts     # 설정 IPC
│   │   │
│   │   ├── services/               # 비즈니스 로직 서비스
│   │   │   ├── fs/
│   │   │   │   ├── FileSystemService.ts
│   │   │   │   ├── FileWatcher.ts
│   │   │   │   └── ProjectScanner.ts
│   │   │   ├── ai/
│   │   │   │   ├── AIProviderManager.ts      # 프로바이더 관리자
│   │   │   │   ├── adapters/
│   │   │   │   │   ├── BaseAIAdapter.ts      # 어댑터 인터페이스
│   │   │   │   │   ├── ClaudeAdapter.ts
│   │   │   │   │   ├── OpenAIAdapter.ts
│   │   │   │   │   ├── GeminiAdapter.ts
│   │   │   │   │   ├── OllamaAdapter.ts
│   │   │   │   │   └── OpenRouterAdapter.ts
│   │   │   │   ├── ModelRouter.ts            # 작업별 모델 라우팅
│   │   │   │   ├── TokenCounter.ts           # 토큰 사용량 추적
│   │   │   │   └── StreamHandler.ts          # 스트리밍 응답 처리
│   │   │   ├── git/
│   │   │   │   ├── GitService.ts             # simple-git 래퍼
│   │   │   │   ├── GitHubService.ts          # GitHub API 연동
│   │   │   │   ├── SSHKeyManager.ts          # SSH 키 생성/등록
│   │   │   │   └── BranchManager.ts
│   │   │   ├── deploy/
│   │   │   │   ├── DeployManager.ts          # 출시 오케스트레이터
│   │   │   │   ├── adapters/
│   │   │   │   │   ├── BaseDeployAdapter.ts
│   │   │   │   │   ├── VercelAdapter.ts
│   │   │   │   │   ├── RailwayAdapter.ts
│   │   │   │   │   ├── NetlifyAdapter.ts
│   │   │   │   │   ├── FlyioAdapter.ts
│   │   │   │   │   ├── AWSAdapter.ts
│   │   │   │   │   └── GCPAdapter.ts
│   │   │   │   ├── EnvDetector.ts            # 환경변수 자동 감지
│   │   │   │   ├── StackAnalyzer.ts          # 프로젝트 스택 분석
│   │   │   │   └── PipelineManager.ts        # 출시 파이프라인
│   │   │   ├── security/
│   │   │   │   ├── SecurityScanner.ts        # 통합 보안 스캐너
│   │   │   │   ├── ESLintRunner.ts
│   │   │   │   ├── SemgrepRunner.ts
│   │   │   │   ├── DependencyAuditor.ts      # npm audit 등
│   │   │   │   ├── SecretDetector.ts         # 하드코딩된 시크릿 탐지
│   │   │   │   └── AICodeReviewer.ts         # AI 기반 코드 리뷰
│   │   │   ├── auth/
│   │   │   │   ├── AuthManager.ts            # 인증 통합 관리자
│   │   │   │   ├── OAuthService.ts           # OAuth 플로우 처리
│   │   │   │   ├── KeychainService.ts        # keytar 래퍼
│   │   │   │   └── TokenRefresher.ts         # 토큰 자동 갱신
│   │   │   ├── terminal/
│   │   │   │   ├── TerminalManager.ts        # node-pty 관리
│   │   │   │   └── ProcessRunner.ts          # 빌드/서버 프로세스
│   │   │   ├── monitoring/
│   │   │   │   ├── MonitoringService.ts
│   │   │   │   ├── MetricsCollector.ts
│   │   │   │   ├── ErrorTracker.ts
│   │   │   │   ├── LogAggregator.ts
│   │   │   │   ├── AlertManager.ts
│   │   │   │   ├── CostTracker.ts
│   │   │   │   └── AgentInjector.ts          # 모니터링 에이전트 삽입
│   │   │   └── settings/
│   │   │       ├── SettingsManager.ts
│   │   │       ├── ThemeManager.ts
│   │   │       └── I18nManager.ts
│   │   │
│   │   ├── windows/                # 윈도우/뷰 관리
│   │   │   ├── MainWindow.ts
│   │   │   ├── PreviewBrowserView.ts
│   │   │   └── OAuthBrowserView.ts
│   │   │
│   │   └── utils/
│   │       ├── logger.ts
│   │       ├── paths.ts            # 앱 경로 유틸
│   │       └── platform.ts         # OS별 분기
│   │
│   ├── preload/                    # ── Preload Script ──
│   │   ├── index.ts                # contextBridge 설정
│   │   ├── api/
│   │   │   ├── fs.api.ts           # window.api.fs.*
│   │   │   ├── ai.api.ts           # window.api.ai.*
│   │   │   ├── git.api.ts          # window.api.git.*
│   │   │   ├── deploy.api.ts       # window.api.deploy.*
│   │   │   ├── auth.api.ts         # window.api.auth.*
│   │   │   ├── terminal.api.ts     # window.api.terminal.*
│   │   │   ├── security.api.ts     # window.api.security.*
│   │   │   ├── monitoring.api.ts   # window.api.monitoring.*
│   │   │   └── settings.api.ts     # window.api.settings.*
│   │   └── types.ts                # Preload API 타입 정의
│   │
│   ├── renderer/                   # ── Renderer Process (React) ──
│   │   ├── index.html
│   │   ├── main.tsx                # React 엔트리포인트
│   │   ├── App.tsx                 # 루트 컴포넌트, 뷰 라우팅 (login→pricing→onboarding→dashboard→ide)
│   │   │
│   │   ├── pages/                  # 페이지 컴포넌트
│   │   │   ├── LoginPage.tsx       # 로그인 화면 (이메일/소셜 로그인)
│   │   │   ├── PricingPage.tsx     # 요금제 선택 화면 (free/pro/team/enterprise)
│   │   │   ├── OnboardingPage.tsx  # 온보딩 (4스텝: 웰컴→AI연결→GitHub→테마)
│   │   │   ├── DashboardPage.tsx   # 서비스 대시보드 (서비스 목록 + 최근 활동 + 연결 서비스)
│   │   │   ├── IDEPage.tsx         # IDE 메인 (워크플로우 캔버스 / 코드 에디터 토글)
│   │   │   └── SettingsPage.tsx    # 설정 (일반/테마/언어/에디터/AI/Git/출시/알림/키바인딩)
│   │   │
│   │   ├── components/             # UI 컴포넌트
│   │   │   ├── common/             # 공통 UI
│   │   │   │   ├── NavBar.tsx      # 커스텀 네비게이션바 (뒤로/홈/브레드크럼/뷰토글/설정/유저/윈도우컨트롤)
│   │   │   │   ├── Button.tsx
│   │   │   │   ├── Modal.tsx
│   │   │   │   ├── Toast.tsx
│   │   │   │   ├── StatusBadge.tsx # 상태 뱃지 (개발중/검증중/출시완료/오류)
│   │   │   │   ├── Dropdown.tsx
│   │   │   │   ├── Tabs.tsx
│   │   │   │   ├── SplitPane.tsx   # 리사이즈 가능한 패널
│   │   │   │   ├── ContextMenu.tsx
│   │   │   │   └── LoadingSpinner.tsx
│   │   │   │
│   │   │   ├── workflow/           # 워크플로우 캔버스
│   │   │   │   └── WorkflowCanvas.tsx  # DAG 워크플로우 (팬/줌, 노드 클릭→인포패널)
│   │   │   │
│   │   │   ├── chat/               # AI 채팅
│   │   │   │   ├── ChatPanel.tsx   # 코드 에디터 모드 우측 채팅 패널
│   │   │   │   └── ChatWidget.tsx  # 워크플로우 모드 하단 채팅 오버레이 위젯
│   │   │   │
│   │   │   ├── auth/               # 인증
│   │   │   │   └── AuthModal.tsx   # OAuth 인증 모달 (BrowserView 기반)
│   │   │   │
│   │   │   ├── sidebar/            # 사이드바
│   │   │   │   ├── ActivityBar.tsx # 아이콘 네비게이션 바
│   │   │   │   └── Sidebar.tsx     # 좌측 사이드바 (파일탐색기 등)
│   │   │   │
│   │   │   ├── dashboard/          # 모듈 A: 서비스 대시보드
│   │   │   │   ├── Dashboard.tsx
│   │   │   │   ├── ProjectCard.tsx
│   │   │   │   ├── ProjectCreateModal.tsx
│   │   │   │   ├── RecentActivity.tsx
│   │   │   │   └── ProjectImportWizard.tsx
│   │   │   │
│   │   │   ├── editor/             # 모듈 B: Monaco Editor
│   │   │   │   ├── EditorPanel.tsx
│   │   │   │   ├── MonacoEditor.tsx
│   │   │   │   ├── EditorTabs.tsx
│   │   │   │   ├── Breadcrumb.tsx
│   │   │   │   └── EditorToolbar.tsx
│   │   │   │
│   │   │   ├── file-explorer/      # 파일 탐색기
│   │   │   │   ├── FileExplorer.tsx
│   │   │   │   ├── FileTree.tsx
│   │   │   │   ├── FileTreeNode.tsx
│   │   │   │   └── FileSearch.tsx
│   │   │   │
│   │   │   ├── ai-chat/            # 모듈 B: AI 채팅
│   │   │   │   ├── AIChatPanel.tsx
│   │   │   │   ├── ChatMessage.tsx
│   │   │   │   ├── ChatInput.tsx
│   │   │   │   ├── CodeBlock.tsx
│   │   │   │   ├── DiffView.tsx
│   │   │   │   ├── FileProgress.tsx
│   │   │   │   ├── ModelSelector.tsx
│   │   │   │   ├── TokenCounter.tsx
│   │   │   │   └── ContextTagger.tsx # @파일명 태깅
│   │   │   │
│   │   │   ├── preview/            # 모듈 B-3: 미리보기
│   │   │   │   ├── PreviewPanel.tsx
│   │   │   │   ├── PreviewToolbar.tsx
│   │   │   │   └── ViewportSelector.tsx
│   │   │   │
│   │   │   ├── debug/              # 모듈 B-4: 디버그 콘솔
│   │   │   │   ├── DebugPanel.tsx
│   │   │   │   ├── ConsoleTab.tsx
│   │   │   │   ├── NetworkTab.tsx
│   │   │   │   ├── ProblemsTab.tsx
│   │   │   │   └── TerminalTab.tsx
│   │   │   │
│   │   │   ├── ai-provider/        # 모듈 B-1: AI 프로바이더
│   │   │   │   ├── AIProviderPanel.tsx
│   │   │   │   ├── ProviderCard.tsx
│   │   │   │   ├── APIKeyInput.tsx
│   │   │   │   ├── ModelRoleConfig.tsx
│   │   │   │   └── UsageGauge.tsx
│   │   │   │
│   │   │   ├── security/           # 모듈 C: 보안/품질 검증
│   │   │   │   ├── SecurityPanel.tsx
│   │   │   │   ├── SecurityReport.tsx
│   │   │   │   ├── IssueCard.tsx
│   │   │   │   ├── ScoreGauge.tsx
│   │   │   │   └── VerificationGate.tsx
│   │   │   │
│   │   │   ├── git/                # 모듈 D: Git/GitHub
│   │   │   │   ├── SourceControlPanel.tsx
│   │   │   │   ├── ChangesList.tsx
│   │   │   │   ├── CommitForm.tsx
│   │   │   │   ├── BranchSelector.tsx
│   │   │   │   ├── GitHistory.tsx
│   │   │   │   ├── MergeConflictView.tsx
│   │   │   │   ├── PullRequestPanel.tsx
│   │   │   │   ├── IssuePanel.tsx
│   │   │   │   ├── ActionsPanel.tsx
│   │   │   │   └── GitHubSetupWizard.tsx
│   │   │   │
│   │   │   ├── deploy/             # 모듈 E: 출시 센터
│   │   │   │   ├── DeployCenter.tsx
│   │   │   │   ├── DeployDashboard.tsx
│   │   │   │   ├── DeployWizard.tsx
│   │   │   │   ├── DeployProgress.tsx
│   │   │   │   ├── DeployHistory.tsx
│   │   │   │   ├── PipelineConfig.tsx
│   │   │   │   ├── EnvVarsEditor.tsx
│   │   │   │   ├── DomainManager.tsx
│   │   │   │   └── RollbackPanel.tsx
│   │   │   │
│   │   │   ├── monitoring/         # 모듈 F: 워치독
│   │   │   │   ├── WatchdogDashboard.tsx
│   │   │   │   ├── TrafficGraph.tsx
│   │   │   │   ├── ResourceMonitor.tsx
│   │   │   │   ├── ErrorTracker.tsx
│   │   │   │   ├── LogViewer.tsx
│   │   │   │   ├── AlertCenter.tsx
│   │   │   │   ├── CostTracker.tsx
│   │   │   │   ├── ScalingControl.tsx
│   │   │   │   └── AIOpsPanel.tsx
│   │   │   │
│   │   │   ├── settings/           # 모듈 G: 설정
│   │   │   │   ├── SettingsPanel.tsx
│   │   │   │   ├── ThemeSettings.tsx
│   │   │   │   ├── LanguageSettings.tsx
│   │   │   │   ├── EditorSettings.tsx
│   │   │   │   ├── KeybindingSettings.tsx
│   │   │   │   └── AccessibilitySettings.tsx
│   │   │   │
│   │   │   └── auth/               # 모듈 H: 인증 허브
│   │   │       ├── AccountHub.tsx
│   │   │       ├── ServiceConnector.tsx
│   │   │       ├── OAuthModal.tsx
│   │   │       └── ConnectionStatus.tsx
│   │   │
│   │   ├── stores/                 # Zustand 상태 관리
│   │   │   ├── appStore.ts         # 앱 전역 상태 (currentView, theme, language, user, workflowView 등)
│   │   │   ├── projectStore.ts     # 서비스(프로젝트) 상태
│   │   │   ├── useEditorStore.ts
│   │   │   ├── useFileStore.ts
│   │   │   ├── useAIChatStore.ts
│   │   │   ├── useAIProviderStore.ts
│   │   │   ├── useGitStore.ts
│   │   │   ├── useDeployStore.ts
│   │   │   ├── useSecurityStore.ts
│   │   │   ├── useMonitoringStore.ts
│   │   │   ├── useTerminalStore.ts
│   │   │   ├── useSettingsStore.ts
│   │   │   └── useAuthStore.ts
│   │   │
│   │   ├── hooks/                  # 커스텀 React Hooks
│   │   │   ├── useIPC.ts           # IPC 통신 헬퍼
│   │   │   ├── useIPCEvent.ts      # IPC 이벤트 리스너
│   │   │   ├── useMonaco.ts        # Monaco 에디터 훅
│   │   │   ├── useTheme.ts
│   │   │   ├── useI18n.ts
│   │   │   ├── useKeyBinding.ts
│   │   │   └── useDebounce.ts
│   │   │
│   │   ├── i18n/                   # 다국어 지원
│   │   │   ├── index.ts
│   │   │   ├── ko.json             # 한국어
│   │   │   ├── en.json             # 영어
│   │   │   ├── ja.json             # 일본어
│   │   │   └── zh-CN.json          # 중국어 간체
│   │   │
│   │   ├── themes/                 # 테마 정의
│   │   │   ├── index.ts
│   │   │   ├── dark.ts
│   │   │   ├── light.ts
│   │   │   ├── monokai.ts
│   │   │   ├── dracula.ts
│   │   │   ├── nord.ts
│   │   │   ├── github.ts
│   │   │   ├── one-dark.ts
│   │   │   ├── tokyo-night.ts
│   │   │   ├── solarized.ts
│   │   │   └── vscode-compat.ts    # VSCode 테마 파서
│   │   │
│   │   ├── styles/
│   │   │   ├── globals.css         # Tailwind import + 디자인 토큰 + 글로벌 스타일
│   │   │   ├── components.css      # 공통 컴포넌트 클래스 (카드, 버튼, 뱃지, 타이포그래피 등)
│   │   │   ├── navbar.css          # 커스텀 네비게이션바 스타일
│   │   │   ├── monaco-overrides.css
│   │   │   └── pages/
│   │   │       ├── login.css       # 로그인 페이지 전용
│   │   │       ├── pricing.css     # 요금제 페이지 전용
│   │   │       ├── onboarding.css  # 온보딩 페이지 전용
│   │   │       ├── dashboard.css   # 대시보드 페이지 전용
│   │   │       ├── ide.css         # IDE 페이지 전용
│   │   │       ├── settings.css    # 설정 페이지 전용
│   │   │       └── workflow.css    # 워크플로우 캔버스 전용
│   │   │
│   │   └── utils/
│   │       ├── format.ts
│   │       ├── diff.ts
│   │       └── validators.ts
│   │
│   └── shared/                     # ── Main/Renderer 공유 ──
│       ├── types/                  # 공유 타입 정의
│       │   ├── project.ts
│       │   ├── ai.ts
│       │   ├── git.ts
│       │   ├── deploy.ts
│       │   ├── security.ts
│       │   ├── monitoring.ts
│       │   ├── auth.ts
│       │   ├── settings.ts
│       │   └── ipc-channels.ts     # IPC 채널명 상수
│       ├── constants/
│       │   ├── providers.ts        # AI 프로바이더 목록
│       │   ├── platforms.ts        # 출시 플랫폼 목록
│       │   └── defaults.ts         # 기본 설정값
│       └── utils/
│           ├── validators.ts
│           └── formatters.ts
│
├── resources/                      # 앱 리소스
│   ├── icons/                      # 앱 아이콘 (Mac/Win/Linux)
│   ├── file-icons/                 # 파일 탐색기 아이콘
│   └── tray/                       # 시스템 트레이 아이콘
│
├── scripts/                        # 빌드/개발 스크립트
│   ├── dev.ts                      # 개발 서버 실행
│   ├── build.ts                    # 프로덕션 빌드
│   └── notarize.ts                 # macOS 공증
│
├── tests/
│   ├── unit/
│   │   ├── main/
│   │   └── renderer/
│   ├── integration/
│   └── e2e/
│
└── .github/
    └── workflows/
        ├── ci.yml
        └── release.yml
```

### 1.4 기술 스택 상세

| 레이어 | 기술 | 버전 (권장) | 용도 |
|--------|------|------------|------|
| **프레임워크** | Electron | 33+ | 크로스플랫폼 데스크톱 앱 |
| **빌드 도구** | electron-vite | 2.x | Electron + Vite 통합 빌드 |
| **UI 프레임워크** | React | 19.x | 컴포넌트 기반 UI |
| **타입시스템** | TypeScript | 5.x | 정적 타입 |
| **코드 에디터** | Monaco Editor | 0.52+ | VSCode 동일 엔진 |
| **스타일링** | Tailwind CSS | 4.x | 유틸리티 기반 CSS |
| **상태관리** | Zustand | 5.x | 경량 상태관리 |
| **터미널** | xterm.js + node-pty | 5.x / 1.x | 내장 터미널 에뮬레이터 |
| **Git** | simple-git | 3.x | Node.js Git 클라이언트 |
| **키체인** | keytar | 7.x | OS 키체인 접근 (토큰 저장) |
| **HTTP** | axios | 1.x | AI API / 외부 서비스 호출 |
| **WebSocket** | ws | 8.x | 실시간 통신 (모니터링, AI 스트리밍) |
| **보안 스캔** | ESLint + Semgrep | 9.x / latest | 정적 분석 |
| **테스트** | Vitest + Playwright | 2.x / 1.x | 단위/E2E 테스트 |
| **패키징** | electron-builder | 25.x | 앱 패키징/배포 |
| **아이콘** | Lucide React | latest | UI 아이콘 세트 |

---

### 1.5 앱 플로우 & 네비게이션

#### AppView 타입과 화면 전환

```typescript
type AppView = 'login' | 'pricing' | 'onboarding' | 'dashboard' | 'ide' | 'settings' | 'deploy' | 'watchdog';
```

**기본 플로우:**
```
login → pricing → onboarding → dashboard → ide (워크플로우 캔버스 + 채팅 위젯)
                                          ↕
                                       settings
```

- **login**: 이메일/소셜 로그인. 로그인 성공 시 userPlan 존재하면 dashboard, 없으면 pricing으로 이동
- **pricing**: 요금제 선택 (free/pro/team/enterprise). 선택 후 onboarding으로 이동
- **onboarding**: 4스텝 (웰컴 → AI 서비스 연결 → GitHub 연결 → 테마 선택). 완료 후 dashboard
- **dashboard**: 서비스 목록 + 최근 활동 + 연결된 서비스 (AI/Git/출시/DB/결제/알림/인증/스토리지)
- **ide**: 두 가지 모드 (workflowView 토글로 전환)
  - 워크플로우 캔버스 (기본): 전체화면 DAG 캔버스 + ChatWidget 오버레이
  - 코드 에디터: 4패널 구조 (사이드바 + 에디터 + 디버그 + ChatPanel)
- **settings**: 좌측 사이드바 탭 (일반/테마/언어/에디터/AI/Git/출시/알림/키바인딩/단축키)

#### NavBar

커스텀 네비게이션바는 login, pricing, onboarding 화면에서는 숨겨진다.
표시되는 화면(dashboard, ide, settings)에서는:
- 좌측: 뒤로가기, 홈(대시보드), 구분선, 브레드크럼
- 우측: IDE에서 워크플로우/코드에디터 토글, 설정, 유저 메뉴 (이메일+플랜+로그아웃), 구분선, 윈도우 컨트롤 (최소화/최대화/닫기)

#### 사용자 상태 (appStore)

```typescript
isLoggedIn: boolean;
userPlan: 'free' | 'pro' | 'team' | 'enterprise' | null;
userEmail: string | null;
workflowView: boolean;  // IDE 모드 토글 (true=워크플로우, false=코드에디터)
```

#### CSS 아키텍처

CSS 파일 계층 구조: globals.css에서 모든 스타일을 import한다.

```
globals.css (엔트리)
├── @import "tailwindcss"           # Tailwind 유틸리티
├── @import "./components.css"      # 공통 컴포넌트 (카드, 버튼, 뱃지, 타이포그래피, 리스트 등)
├── @import "./navbar.css"          # 커스텀 네비게이션바
└── @import "./pages/*.css"         # 페이지별 전용 스타일
    ├── login.css
    ├── pricing.css
    ├── onboarding.css
    ├── dashboard.css
    ├── ide.css
    ├── settings.css
    └── workflow.css
```

**규칙:**
- 공통 컴포넌트 → `components.css`
- 페이지 전용 → `pages/해당페이지.css`
- 인라인 Tailwind는 일회성 스타일에만 사용
- 호버/트랜지션은 반드시 CSS 클래스로 정의

---

## 2. 전체 기능 명세

### 2.1 모듈 A: 서비스 대시보드

**목적:** 사용자의 모든 서비스를 한 눈에 관리하는 진입점.

#### 기능 목록

| 기능 | 상세 | 구현 방식 |
|------|------|----------|
| 서비스 목록 | 카드/리스트 뷰로 모든 프로젝트 표시 | Zustand `useProjectStore` + 로컬 SQLite |
| 서비스 생성 | 3가지: PRD 입력, 로컬 폴더 연결, GitHub 클론 | 모달 위자드 → IPC `project:create` |
| 서비스 삭제 | 확인 다이얼로그 후 메타데이터 삭제 (파일은 선택) | IPC `project:delete` |
| 서비스 수정 | 이름, 설명, 스택 태그 편집 | IPC `project:update` |
| 상태 표시 | 개발중 / 검증중 / 출시완료 / 오류 | 배지 컴포넌트 |
| 요약 정보 | 스택, 최근 활동, 출시 URL, 트래픽 | 프로젝트 카드 내 표시 |
| 최근 활동 피드 | 코드 생성, 출시, 에러 등 타임라인 | 이벤트 로그 기반 |
| 서비스 검색/필터 | 이름, 스택, 상태로 검색 | 로컬 필터링 |
| 로컬 폴더 연결 | Electron `dialog.showOpenDialog` | IPC `project:link-folder` |

#### 상태 관리 (`useProjectStore`)

```typescript
interface ProjectStore {
  projects: Project[];
  currentProject: Project | null;
  isLoading: boolean;

  // Actions
  fetchProjects: () => Promise<void>;
  createProject: (input: CreateProjectInput) => Promise<Project>;
  updateProject: (id: string, input: UpdateProjectInput) => Promise<void>;
  deleteProject: (id: string) => Promise<void>;
  setCurrentProject: (id: string) => void;
  linkLocalFolder: (id: string) => Promise<void>;
  cloneFromGitHub: (repoUrl: string) => Promise<Project>;
}
```

### 2.2 모듈 B: AI Web IDE

**목적:** Monaco Editor 기반의 풀 피처 코드 에디터 + AI 채팅 + 미리보기 + 디버그를 4패널 구조로 제공.

#### 4패널 레이아웃 구조

```
┌──────────────────────────────────────────────────────────┐
│ [Activity Bar]                                            │
├───────────┬───────────────────────────┬──────────────────┤
│           │                           │                  │
│  좌측 패널  │      중앙 패널 (상단)       │   우측 패널      │
│           │    Monaco Editor          │   AI 채팅        │
│ 파일탐색기  │    (멀티탭)                │   (챗 형식 UI)   │
│ AI 프로바이더│                           │                  │
│ 소스컨트롤  │                           │                  │
│ 검색      ├───────────────────────────┤                  │
│ 배포      │      중앙 패널 (하단)       │                  │
│ 워치독    │  미리보기 | 디버그 콘솔      │                  │
│ 설정      │  (탭 전환)                 │                  │
│           │                           │                  │
├───────────┴───────────────────────────┴──────────────────┤
│ [Status Bar: 브랜치, 인코딩, 언어, 행:열, AI 모델, 토큰]   │
└──────────────────────────────────────────────────────────┘
```

#### B-1: 코드 에디터 (Monaco Editor)

| 기능 | 상세 | 구현 |
|------|------|------|
| 구문 강조 | 50+ 언어 지원 | Monaco 내장 |
| 자동완성 | IntelliSense + AI 인라인 제안 | Monaco + 커스텀 CompletionProvider |
| 멀티탭 | 여러 파일 동시 편집, 탭 드래그 | 커스텀 EditorTabs 컴포넌트 |
| 분할 편집 | 좌/우 에디터 분할 | Monaco 에디터 인스턴스 복수 생성 |
| 미니맵 | 코드 미니맵 표시 | Monaco 내장 옵션 |
| 검색/치환 | 파일 내 + 프로젝트 전체 검색 | Monaco 검색 + 커스텀 전역 검색 |
| diff 에디터 | Git 변경사항 비교 | `monaco.editor.createDiffEditor()` |
| 포매팅 | 저장 시 자동 포맷 (Prettier) | `editor.getAction('editor.action.formatDocument')` |
| 테마 적용 | Monaco 테마와 UI 테마 동기화 | `monaco.editor.defineTheme()` |
| 키바인딩 | VidEplace/VSCode/Vim/Emacs 프리셋 | Monaco keybinding 커스텀 |
| Breadcrumb | 파일 경로 네비게이션 | 커스텀 컴포넌트 |

#### B-2: AI 채팅 패널 (챗 형식)

| 기능 | 상세 | 구현 |
|------|------|------|
| 메시지 히스토리 | 스크롤 가능한 대화 기록, 세션별 저장 | 가상화 리스트 + IndexedDB |
| 코드블록 렌더링 | 구문 강조 + 복사 + "에디터에서 열기" | react-syntax-highlighter |
| diff 인라인 표시 | 생성/수정된 파일 변경사항 diff | 커스텀 DiffView 컴포넌트 |
| 승인/거절 버튼 | AI 제안에 대한 원클릭 응답 | 인터랙티브 메시지 카드 |
| 파일 생성 진행률 | 실시간 생성 상태 표시 | FileProgress 컴포넌트 |
| 토큰 카운터 | 실시간 토큰 사용량 + 예상 비용 | TokenCounter 컴포넌트 |
| 모델 전환 | 채팅 중 모델 변경 (드롭다운) | ModelSelector 컴포넌트 |
| 이미지 첨부 | 스크린샷/와이어프레임 → 멀티모달 입력 | 파일 드래그앤드롭 + base64 |
| 컨텍스트 태깅 | `@파일명`으로 컨텍스트 추가 | ContextTagger + 자동완성 |
| 대화 분기 | 특정 메시지에서 분기하여 다른 방향 시도 | 트리 구조 대화 히스토리 |
| 스트리밍 응답 | AI 응답 실시간 스트리밍 표시 | SSE/WebSocket → 점진적 렌더링 |

**AI 코드 생성 플로우:**

```
1. 사용자가 AI 채팅에서 요구사항 입력
2. Renderer → IPC 'ai:generate' → Main Process
3. Main → AIProviderManager → 적절한 어댑터 선택
4. 어댑터 → AI API 스트리밍 호출
5. Main → IPC 이벤트 'ai:stream' → Renderer (점진적 표시)
6. 코드 생성 완료 → 파일 시스템 기록 (IPC 'fs:write')
7. 자동 보안 스캔 트리거 (IPC 'security:scan')
8. 미리보기 자동 새로고침
```

#### B-3: 미리보기 패널

| 기능 | 상세 | 구현 |
|------|------|------|
| 실시간 프리뷰 | 코드 변경 시 자동 리로드 | Electron BrowserView + HMR 감지 |
| 반응형 테스트 | 모바일/태블릿/데스크톱 뷰포트 전환 | BrowserView 크기 조절 |
| URL 네비게이션 | 주소바로 라우트 이동 | BrowserView `loadURL()` |
| DevTools 연동 | 미리보기 내 요소 검사 | `webContents.openDevTools()` |
| 스크린샷 캡처 | 현재 화면 캡처 → AI 채팅 첨부 | `webContents.capturePage()` |

**BrowserView 관리 설계:**

```typescript
// Main Process: PreviewBrowserView.ts
class PreviewBrowserView {
  private view: BrowserView;

  create(parentWindow: BrowserWindow): void;
  loadURL(url: string): void;
  setViewport(width: number, height: number): void;
  reload(): void;
  captureScreenshot(): Promise<Buffer>;
  attachDevTools(): void;
  destroy(): void;
}
```

#### B-4: 디버그 콘솔 (4탭)

| 탭 | 기능 | 구현 |
|----|------|------|
| **Console** | stdout/stderr 로그, console.log 캡처, 에러 스택트레이스 | node-pty 출력 캡처 + BrowserView console 가로채기 |
| **Network** | HTTP 요청/응답 모니터링, 상태코드, 응답시간 | BrowserView webRequest API |
| **Problems** | ESLint 경고, TS 에러, 보안 스캔 결과 통합 | SecurityScanner 결과 수집 |
| **Terminal** | 내장 터미널 (npm run, git 등) | xterm.js + node-pty |

**디버그 → AI 연동:**
- 에러 로그 옆 "AI에게 수정 요청" 버튼 클릭 시 에러 컨텍스트를 AI 채팅 스토어에 주입
- `useAIChatStore.addSystemMessage({ type: 'error-context', error, file, line })`

### 2.3 모듈 B-1: AI 프로바이더 연동 (BYOK)

**목적:** 사용자 소유의 AI 계정/키를 연결하여 AI 기능 사용. 서버를 거치지 않고 직접 통신.

#### 지원 프로바이더

| 프로바이더 | 1순위: OAuth 로그인 | 2순위: API Key | 지원 모델 |
|-----------|-------------------|----------------|----------|
| Anthropic (Claude) | Claude.ai OAuth | API Key 입력 | Opus, Sonnet, Haiku |
| OpenAI | ChatGPT OAuth | API Key 입력 | GPT-4o, GPT-4.1, o3 |
| Google (Gemini) | Google 계정 OAuth | API Key 입력 | Gemini 2.5 Pro, Flash |
| Ollama (로컬) | 로컬 자동 감지 | 서버 URL 수동 입력 | Llama, Mistral 등 |
| OpenRouter | - | API Key 입력 | 다양한 모델 통합 |

#### 어댑터 패턴 설계

```typescript
// src/main/services/ai/adapters/BaseAIAdapter.ts
abstract class BaseAIAdapter {
  abstract readonly providerId: string;
  abstract readonly providerName: string;

  abstract authenticate(credentials: OAuthToken | APIKey): Promise<AuthResult>;
  abstract getAvailableModels(): Promise<AIModel[]>;
  abstract detectSubscription(): Promise<SubscriptionInfo>;

  abstract chat(params: ChatParams): AsyncGenerator<ChatStreamChunk>;
  abstract generateCode(params: CodeGenParams): AsyncGenerator<CodeStreamChunk>;
  abstract reviewCode(params: CodeReviewParams): Promise<ReviewResult>;
  abstract generateCommitMessage(diff: string): Promise<string>;

  abstract getTokenCount(messages: Message[]): number;
  abstract estimateCost(tokenCount: number, model: string): number;
}
```

#### 모델 라우팅

```typescript
// src/main/services/ai/ModelRouter.ts
interface ModelRouting {
  codeGeneration: { provider: string; model: string };
  codeReview:     { provider: string; model: string };
  commitMessage:  { provider: string; model: string };
  debugging:      { provider: string; model: string };
  prDescription:  { provider: string; model: string };
  general:        { provider: string; model: string };
}
```

사용자는 작업 유형별로 다른 프로바이더/모델을 지정할 수 있다. 예를 들어 코드 생성은 Claude Opus, 커밋 메시지는 Haiku를 사용하여 비용을 최적화한다.

#### 토큰 카운터 & 비용 추적

```typescript
// src/main/services/ai/TokenCounter.ts
class TokenCounter {
  countTokens(text: string, model: string): number;
  estimateCost(inputTokens: number, outputTokens: number, model: string): number;
  getSessionUsage(): { tokens: number; cost: number };
  getDailyUsage(): { tokens: number; cost: number };
  getMonthlyUsage(): { tokens: number; cost: number };
}
```

#### 폴백 전략

1순위 모델 실패 시 → 2순위 모델 자동 전환. `AIProviderManager`가 retry 로직과 폴백 체인을 관리한다.

```
Claude Opus (실패) → Claude Sonnet → OpenAI GPT-4o → 에러 표시
```

### 2.4 모듈 C: 보안/품질 검증 엔진

**목적:** AI가 생성한 코드의 보안과 품질을 자동으로 검증하는 게이트웨이. 검증 없이는 출시 불가.

#### 검증 파이프라인

```
코드 변경 감지
    │
    ├─ ESLint (품질/패턴)
    │   └─ 커스텀 Rule Set (React, Next.js, Express 등)
    │
    ├─ Semgrep (보안)
    │   └─ OWASP Top 10 룰
    │   └─ 커스텀 룰 (하드코딩 시크릿 등)
    │
    ├─ npm audit (의존성 취약점)
    │   └─ CVE 데이터베이스 대조
    │
    ├─ AI 코드 리뷰 (논리적 취약점)
    │   └─ 코드 컨텍스트 이해 기반 분석
    │
    └─ 점수 계산
        ├─ 보안 점수: 0~100
        └─ 품질 점수: 0~100
```

#### 검증 항목 상세

| 카테고리 | 검증 항목 | 도구 | 심각도 |
|----------|----------|------|--------|
| 보안 | SQL Injection | Semgrep | HIGH |
| 보안 | XSS (Cross-Site Scripting) | Semgrep | HIGH |
| 보안 | CSRF | Semgrep | HIGH |
| 보안 | 하드코딩된 시크릿/API 키 | SecretDetector (커스텀) | HIGH |
| 보안 | 인증/인가 로직 결함 | AI 분석 | HIGH |
| 보안 | 의존성 취약점 (CVE) | npm audit | MEDIUM~HIGH |
| 품질 | 코드 복잡도 (Cyclomatic) | ESLint complexity rule | MEDIUM |
| 품질 | 미사용 코드/import | ESLint no-unused-vars | LOW |
| 품질 | 에러 핸들링 누락 | ESLint + AI 분석 | MEDIUM |
| 품질 | 성능 안티패턴 (N+1 쿼리 등) | AI 분석 | MEDIUM |
| 구조 | 프로젝트 구조 베스트 프랙티스 | AI 분석 | LOW |
| 구조 | 환경변수 관리 (.env 미사용) | SecretDetector | MEDIUM |

#### 점수 시스템

```typescript
interface VerificationResult {
  securityScore: number;     // 0~100
  qualityScore: number;      // 0~100
  overallScore: number;      // 가중 평균
  issues: VerificationIssue[];
  canDeploy: boolean;        // securityScore >= 70 && HIGH 이슈 0건
  summary: string;
}

interface VerificationIssue {
  id: string;
  severity: 'HIGH' | 'MEDIUM' | 'LOW' | 'INFO';
  category: 'security' | 'quality' | 'structure';
  message: string;
  file: string;
  line: number;
  column: number;
  rule: string;
  tool: 'eslint' | 'semgrep' | 'npm-audit' | 'ai' | 'secret-detector';
  fixSuggestion?: string;
  autoFixable: boolean;
}
```

### 2.5 모듈 D: Git/GitHub 완전 통합

**목적:** VSCode 수준의 Git 워크플로우 + GitHub 연동. GitHub 로그인 한 번으로 SSH 설정부터 모든 Git 작업까지 자동 처리.

#### D-1: GitHub OAuth + SSH 자동 셋업

**자동 셋업 플로우:**

```
1. GitHub OAuth 로그인 (BrowserView 모달)
2. OAuth 토큰 수신 → keytar에 저장
3. GitHub API로 사용자 정보 조회 (name, email)
4. SSH 키 생성: ssh-keygen -t ed25519 -f ~/.ssh/videplace_ed25519 -N ""
5. GitHub API (POST /user/keys)로 SSH 공개키 자동 등록
6. ~/.ssh/config에 항목 추가:
   Host github.com
     IdentityFile ~/.ssh/videplace_ed25519
7. Git 글로벌 설정: git config --global user.name / user.email
8. SSH 연결 테스트: ssh -T git@github.com
9. 완료
```

```typescript
// src/main/services/git/SSHKeyManager.ts
class SSHKeyManager {
  async generateKey(): Promise<{ publicKey: string; privateKeyPath: string }>;
  async registerOnGitHub(token: string, publicKey: string): Promise<void>;
  async updateSSHConfig(privateKeyPath: string): Promise<void>;
  async testConnection(): Promise<boolean>;
  async configureGitUser(name: string, email: string): Promise<void>;
}
```

#### D-2: Git 소스 컨트롤

| 기능 | 구현 | simple-git 메서드 |
|------|------|------------------|
| Stage / Unstage | 파일 단위, 라인 단위 | `git.add()` / `git.reset()` |
| Commit | 일반 커밋 + AI 메시지 생성 | `git.commit()` |
| Push / Pull / Fetch | 원격 동기화 | `git.push()` / `git.pull()` / `git.fetch()` |
| 브랜치 관리 | 생성, 전환, 삭제, 이름 변경 | `git.branch()` / `git.checkout()` |
| Merge | 머지 + 충돌 해결 UI | `git.merge()` |
| Stash | 변경사항 임시 저장/복원 | `git.stash()` |
| 커밋 히스토리 | 그래프 뷰 + diff | `git.log()` |
| Blame | 라인별 작성자/커밋 확인 | `git.raw(['blame'])` |
| Diff | 변경사항 비교 | `git.diff()` |

#### D-3: GitHub API 연동

| 기능 | GitHub API | HTTP 메서드 |
|------|-----------|------------|
| PR 생성 | `/repos/{owner}/{repo}/pulls` | POST |
| PR 목록 | `/repos/{owner}/{repo}/pulls` | GET |
| PR 머지 | `/repos/{owner}/{repo}/pulls/{number}/merge` | PUT |
| PR 리뷰 | `/repos/{owner}/{repo}/pulls/{number}/reviews` | POST |
| Issue 생성 | `/repos/{owner}/{repo}/issues` | POST |
| Issue 목록 | `/repos/{owner}/{repo}/issues` | GET |
| Actions 상태 | `/repos/{owner}/{repo}/actions/runs` | GET |
| 리포 생성 | `/user/repos` | POST |
| 리포 클론 | simple-git `clone()` | - |
| SSH 키 등록 | `/user/keys` | POST |

#### AI 연동 Git 기능

```typescript
// AI가 diff를 분석하여 커밋 메시지 자동 생성
async generateCommitMessage(diff: string): Promise<string>;

// AI가 PR 변경사항을 요약하여 본문 자동 생성
async generatePRDescription(commits: Commit[], diff: string): Promise<string>;

// AI가 코드 리뷰 코멘트 생성
async reviewPullRequest(diff: string): Promise<ReviewComment[]>;

// 머지 충돌 시 AI가 해결 방안 제안
async suggestConflictResolution(conflict: MergeConflict): Promise<string>;

// CI 실패 로그를 AI가 분석
async analyzeActionFailure(log: string): Promise<FailureAnalysis>;
```

### 2.6 모듈 E: 출시 센터

**목적:** 서비스 분석 → 플랫폼 추천 → 환경 구성 → 빌드 → 검증 게이트 → 출시 → 롤백까지 전체 파이프라인 관리.

#### 스마트 출시 위자드 (6단계)

```
Step 1: 프로젝트 자동 분석
  └─ StackAnalyzer: 프레임워크, 런타임, DB, 외부 서비스 자동 감지
  └─ package.json, requirements.txt, Dockerfile 등 파싱

Step 2: 플랫폼 AI 추천
  └─ 스택에 맞는 최적 플랫폼 + 예상 비용 제시
  └─ 사용자가 플랫폼 선택

Step 3: 계정 연결
  └─ 선택한 플랫폼에 OAuth 로그인 (BrowserView 모달)
  └─ 토큰 keytar에 저장

Step 4: 환경변수 설정
  └─ EnvDetector: 코드에서 process.env.* 패턴 스캔
  └─ .env 파일에서 값 자동 매핑
  └─ 시크릿 자동 생성 (NEXTAUTH_SECRET 등)

Step 5: 출시 전 검증 게이트
  └─ 빌드 테스트 (npm run build)
  └─ 보안 스캔 (HIGH 이슈 0건 필수)
  └─ 환경변수 완전성 확인
  └─ 시크릿 노출 검사 (.gitignore 확인)

Step 6: 출시 실행
  └─ 실시간 빌드 로그 스트리밍
  └─ DNS + SSL 자동 설정
  └─ 헬스체크
```

#### 출시 플랫폼 어댑터

```typescript
// src/main/services/deploy/adapters/BaseDeployAdapter.ts
abstract class BaseDeployAdapter {
  abstract readonly platformId: string;
  abstract readonly platformName: string;

  abstract authenticate(token: OAuthToken | APIToken): Promise<void>;
  abstract analyzeCompatibility(stack: StackInfo): CompatibilityResult;
  abstract estimateCost(stack: StackInfo): CostEstimate;

  abstract deploy(config: DeployConfig): AsyncGenerator<DeployEvent>;
  abstract getDeployStatus(deployId: string): Promise<DeployStatus>;
  abstract rollback(deployId: string, targetVersion: string): Promise<void>;
  abstract deleteDeploy(deployId: string): Promise<void>;

  abstract setEnvVars(projectId: string, vars: EnvVar[]): Promise<void>;
  abstract getEnvVars(projectId: string): Promise<EnvVar[]>;

  abstract addDomain(projectId: string, domain: string): Promise<DomainConfig>;
  abstract getDomains(projectId: string): Promise<DomainConfig[]>;
  abstract checkDNS(domain: string): Promise<DNSStatus>;

  abstract getLogs(deployId: string): AsyncGenerator<LogEntry>;
  abstract getDeployHistory(projectId: string): Promise<DeployRecord[]>;
}
```

#### 지원 플랫폼 로드맵

| 플랫폼 | Phase | 연결 방식 | 강점 |
|--------|-------|----------|------|
| Vercel | MVP | OAuth | Next.js 최적, Edge, Preview 출시 |
| Railway | Phase 2 | OAuth | 풀스택 (서버+DB), 쉬운 설정 |
| Netlify | Phase 2 | OAuth | 정적 사이트, Forms, Functions |
| Fly.io | Phase 2 | CLI 토큰 | 글로벌 엣지, Docker |
| AWS (ECS/Lambda) | Phase 3 | IAM/SSO | 엔터프라이즈급 |
| GCP (Cloud Run) | Phase 3 | OAuth | Firebase 연계 |
| Cloudflare | Phase 3 | OAuth | Pages, Workers, R2 |
| DigitalOcean | Phase 4 | OAuth | App Platform |
| 자체 서버 | Phase 4 | SSH | Docker Compose 출시 |

#### 출시 파이프라인 & 멀티 환경

```typescript
interface DeployPipeline {
  triggers: DeployTrigger[];         // 자동 출시 트리거
  gates: VerificationGate[];          // 검증 게이트
  environments: Environment[];         // Preview / Staging / Production
  rollbackPolicy: RollbackPolicy;     // 자동 롤백 조건
  notifications: NotificationConfig[]; // 출시 알림
}

interface Environment {
  name: 'preview' | 'staging' | 'production';
  branch: string;                     // 연결된 브랜치
  autoDeployEnabled: boolean;
  requireApproval: boolean;           // Production은 수동 승인
  envVars: EnvVar[];
  domains: string[];
}
```

### 2.7 모듈 F: 워치독 모니터링

**목적:** 출시된 앱의 실시간 모니터링. Datadog/Grafana 수준의 모니터링을 비개발자도 쓸 수 있게 단순화.

#### 모니터링 에이전트 삽입

출시 시 자동으로 경량 에이전트 코드를 삽입한다:

| 프레임워크 | 삽입 위치 | 수집 데이터 |
|-----------|----------|-----------|
| Next.js | `middleware.ts` | HTTP 요청/응답, 에러, 서버 메트릭 |
| Express | 미들웨어 자동 삽입 | 요청/응답, console 캡처 |
| Flask | WSGI 미들웨어 | 요청/응답, 에러 |

수집 데이터 → WebSocket으로 VidEplace 앱에 실시간 전송 → 로컬 SQLite 저장.

#### 기능 영역

**F-1: 실시간 대시보드**

| 지표 | 표시 | 구현 |
|------|------|------|
| 서비스 상태 | 정상/경고/장애 | 헬스체크 ping |
| 업타임 | 99.97% 등 | 연속 가동시간 계산 |
| 응답시간 | p50, p90, p95, p99 | 히스토그램 |
| 에러율 | 5xx/전체 비율 | 실시간 계산 |
| 트래픽 | req/s 그래프 | 시계열 차트 (recharts) |
| 상태코드 분포 | 2xx/3xx/4xx/5xx | 파이 차트 |

**F-2: 리소스 모니터링**

- CPU, 메모리, 디스크, 네트워크 사용량
- DB 커넥션 풀, 슬로우 쿼리
- 인스턴스별 개별 상태

**F-3: 에러 트래킹**

```typescript
interface TrackedError {
  id: string;
  type: string;                    // TypeError, 500 등
  message: string;
  stackTrace: string;
  file: string;
  line: number;
  firstOccurrence: Date;
  lastOccurrence: Date;
  occurrenceCount: number;
  affectedUsers: number;
  severity: 'HIGH' | 'MEDIUM' | 'LOW';
  status: 'open' | 'resolved' | 'ignored';
  aiAnalysis?: string;             // AI 원인 분석 결과
  aiSuggestedFix?: string;         // AI 수정 코드 제안
}
```

**F-4: 실시간 로그 뷰어**

- 로그 스트리밍 (WebSocket)
- 레벨별 필터 (info/warn/error)
- 검색, JSON 포맷, 시간대 설정

**F-5: 알림 & 인시던트**

| 알림 규칙 (기본) | 조건 | 채널 |
|----------------|------|------|
| 에러율 급증 | 에러율 > 5% (5분간) | 앱 + Slack |
| 응답 지연 | p95 > 2초 | 앱 알림 |
| CPU 과부하 | CPU > 80% (10분간) | 앱 + 이메일 |
| 서비스 다운 | 헬스체크 실패 3회 연속 | 앱 + Slack + 이메일 |
| SSL 만료 임박 | 만료 7일 전 | 이메일 |
| DB 용량 경고 | 80% 초과 | 앱 알림 |
| 예산 초과 | 80% 도달 | 이메일 |

**F-6: 비용 트래커**

- 서비스별 비용 추적 (Vercel, Supabase, AWS S3, Stripe 수수료 등)
- 월별 비용 추세 그래프
- AI 비용 최적화 추천 (더 저렴한 플랜/플랫폼 제안)
- 예산 설정 및 알림

**F-7: AIOps (AI 자동 운영)**

| 기능 | 상세 |
|------|------|
| 에러 자동 분석 | 새 에러 발생 시 AI가 즉시 원인 분석 + 수정 코드 제안 |
| 슬로우 쿼리 최적화 | 느린 DB 쿼리 감지 → 인덱스/쿼리 최적화 제안 |
| 트래픽 예측 | 과거 패턴 학습 → 미래 트래픽 예측 |
| 이상 징후 탐지 | 정상 패턴 학습 → 비정상 자동 감지 |
| 주간 리포트 | 운영 요약 + 핵심 제안 자동 생성 |
| 자동 대응 | 설정에 따라 자동 롤백/스케일업 실행 |

**F-8: 스케일링 컨트롤**

```typescript
interface AutoScaleConfig {
  enabled: boolean;
  scaleUpConditions: ScaleCondition[];    // 하나라도 만족 시 스케일업
  scaleDownConditions: ScaleCondition[];  // 모두 만족 시 스케일다운
  minInstances: number;
  maxInstances: number;
  cooldownMinutes: number;               // 스케일링 후 대기 시간
  budgetLimitMonthly: number;            // 예산 초과 시 스케일링 중단
}
```

### 2.8 모듈 G: 설정 & 개인화

**목적:** IDE의 완성도를 결정하는 커스터마이징 옵션.

#### 테마 시스템

| 기능 | 상세 |
|------|------|
| 내장 테마 9+ | Dark, Light, Monokai, Dracula, Nord, GitHub, One Dark, Tokyo Night, Solarized |
| VSCode 테마 호환 | VSCode 테마 JSON 파일 그대로 import (색상 토큰 매핑) |
| 커스텀 테마 | 색상 팔레트 직접 편집, 저장, 내보내기 |
| 자동 전환 | 시간대별 라이트/다크 자동 전환 |
| 에디터/UI 분리 | Monaco 에디터 테마와 UI 테마 독립 설정 |

**테마 구조:**

```typescript
interface Theme {
  id: string;
  name: string;
  type: 'dark' | 'light';
  colors: {
    // UI 색상
    'editor.background': string;
    'editor.foreground': string;
    'sidebar.background': string;
    'statusBar.background': string;
    'activityBar.background': string;
    'titleBar.background': string;
    'accent': string;
    // ... 50+ 색상 토큰
  };
  tokenColors: MonacoTokenColor[]; // Monaco 구문 강조 색상
}
```

#### i18n (다국어)

- 지원 언어: 한국어, English, 日本語, 中文(简体), 中文(繁體), Espanol, Francais, Deutsch, Portugues, Tieng Viet
- AI 응답 언어: UI 언어 연동 / 항상 영어 / 항상 한국어 / 자동 감지
- AI 코드 주석 언어: 한국어 / 영어 / 없음
- 커밋 메시지 언어: 한국어 / 영어

**구현:** `react-i18next` 또는 경량 커스텀 i18n (`src/renderer/i18n/`에 JSON 번역 파일)

#### 에디터 설정

| 설정 | 옵션 |
|------|------|
| 폰트 | JetBrains Mono, Fira Code, Cascadia Code, D2Coding, Pretendard, Source Code Pro |
| 폰트 크기 | 10~24px (기본 14px) |
| 줄 간격 | 1.0~2.0 (기본 1.6) |
| 리가처 | on/off |
| 탭 크기 | 2/4 (기본 2) |
| 탭 스타일 | 스페이스 / 탭 |
| 자동 저장 | on/off, 지연 시간 |
| 줄바꿈 | 자동 / 수동(80자) / 끔 |
| 미니맵 | 표시/숨김 |
| 포맷 저장 시 | Prettier 연동 |

#### 키바인딩

| 프리셋 | 설명 |
|--------|------|
| VidEplace | 기본 단축키 세트 |
| VSCode | VSCode 호환 키바인딩 |
| Vim | Vim 모드 (Monaco vim 확장) |
| Emacs | Emacs 키바인딩 |
| 커스텀 | 사용자 정의 키 매핑 |

### 2.9 모듈 H: 내장 인증 허브

**목적:** 모든 외부 서비스 로그인이 IDE 내부에서 이루어짐. 사용자가 앱을 절대 떠나지 않는다.

#### BrowserView 모달 OAuth 플로우

```
1. 사용자가 [서비스명 연결하기] 클릭
2. OAuthBrowserView 인스턴스 생성
3. BrowserView에 서비스의 OAuth 로그인 URL 로드
4. 사용자가 모달 내에서 로그인 (ID/PW, SSO, 2FA 모두 지원)
5. OAuth 콜백 URL 감지 (webContents.on('will-redirect'))
6. 인증 코드 추출 → 토큰 교환
7. 토큰을 keytar(OS 키체인)에 암호화 저장
8. BrowserView 모달 자동 닫힘
9. "연결 완료!" 토스트 알림
```

```typescript
// src/main/windows/OAuthBrowserView.ts
class OAuthBrowserView {
  private view: BrowserView;

  async startOAuthFlow(service: OAuthService): Promise<OAuthToken> {
    // 1. BrowserView 생성 (모달)
    // 2. OAuth authorize URL 로드
    // 3. 콜백 URL 감지 → 토큰 교환
    // 4. keytar에 저장
    // 5. BrowserView 닫기
    // 6. 토큰 반환
  }
}
```

#### 지원 서비스 (20+)

| 카테고리 | 서비스 | 로그인 방식 | 용도 |
|----------|--------|-----------|------|
| AI | Claude (Anthropic) | OAuth / API Key | 코드 생성, 검증 |
| AI | OpenAI (ChatGPT) | OAuth / API Key | 코드 생성, 검증 |
| AI | Google (Gemini) | Google OAuth | 코드 생성, 검증 |
| Git | GitHub | OAuth + SSH 자동설정 | 소스 관리, PR, Issues |
| Git | GitLab | OAuth | 소스 관리 |
| Git | Bitbucket | OAuth | 소스 관리 |
| 출시 | Vercel | OAuth | 출시 |
| 출시 | Railway | OAuth | 출시 |
| 출시 | Netlify | OAuth | 출시 |
| 출시 | AWS | IAM SSO | 출시 |
| 출시 | GCP | Google OAuth | 출시 |
| 출시 | Cloudflare | OAuth | 출시 |
| 출시 | DigitalOcean | OAuth | 출시 |
| DB/BaaS | Supabase | OAuth | DB, Auth |
| DB/BaaS | Firebase | Google OAuth | DB, Auth, Hosting |
| DB/BaaS | PlanetScale | OAuth | DB |
| 결제 | Stripe | OAuth | 결제 연동 |
| 알림 | Slack | OAuth | 알림 |
| 알림 | Discord | OAuth (Bot) | 알림 |
| 알림 | Telegram | Bot Token | 알림 |

#### keytar를 통한 토큰 관리

```typescript
// src/main/services/auth/KeychainService.ts
class KeychainService {
  private readonly SERVICE_NAME = 'videplace';

  async saveToken(service: string, token: string): Promise<void> {
    await keytar.setPassword(this.SERVICE_NAME, service, token);
  }

  async getToken(service: string): Promise<string | null> {
    return keytar.getPassword(this.SERVICE_NAME, service);
  }

  async deleteToken(service: string): Promise<boolean> {
    return keytar.deletePassword(this.SERVICE_NAME, service);
  }

  async getAllServices(): Promise<string[]> {
    const credentials = await keytar.findCredentials(this.SERVICE_NAME);
    return credentials.map(c => c.account);
  }
}
```

---

## 3. Phase별 개발 계획

### Phase 1 (MVP) - 3개월

> **목표:** 핵심 루프(코드 작성 → AI 생성 → 검증 → Git → 출시)가 동작하는 최소 제품

#### Month 1: Electron 쉘 + Monaco IDE + 파일시스템

**주차별 계획:**

**Week 1: 프로젝트 초기 세팅**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| electron-vite 서비스 생성 | `package.json`, `electron-builder.yml`, `vite.config.ts` | electron, electron-vite |
| TypeScript 설정 | `tsconfig.json`, `tsconfig.node.json`, `tsconfig.web.json` | typescript |
| Tailwind CSS 설정 | `tailwind.config.ts`, `postcss.config.js`, `src/renderer/styles/globals.css` | tailwindcss |
| Main Process 엔트리포인트 | `src/main/index.ts`, `src/main/windows/MainWindow.ts` | electron |
| Preload 스크립트 기본 구조 | `src/preload/index.ts`, `src/preload/types.ts` | - |
| Renderer 엔트리포인트 | `src/renderer/index.html`, `src/renderer/main.tsx`, `src/renderer/App.tsx` | react, react-dom |
| ESLint + Prettier 설정 | `.eslintrc.cjs`, `.prettierrc` | eslint, prettier |
| IPC 채널 상수 정의 | `src/shared/types/ipc-channels.ts` | - |

**Week 2: 4패널 레이아웃 + 파일 탐색기**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| 4패널 IDE 레이아웃 | `src/renderer/components/layout/IDELayout.tsx`, `Sidebar.tsx`, `ActivityBar.tsx`, `StatusBar.tsx`, `TitleBar.tsx` | - |
| 리사이즈 가능한 SplitPane | `src/renderer/components/common/SplitPane.tsx` | - |
| 파일시스템 서비스 | `src/main/services/fs/FileSystemService.ts`, `FileWatcher.ts` | chokidar |
| 파일시스템 IPC | `src/main/ipc/fs.ipc.ts`, `src/preload/api/fs.api.ts` | - |
| 파일 탐색기 컴포넌트 | `src/renderer/components/file-explorer/FileExplorer.tsx`, `FileTree.tsx`, `FileTreeNode.tsx` | - |
| 파일 스토어 | `src/renderer/stores/useFileStore.ts` | zustand |
| 레이아웃 스토어 | `src/renderer/stores/useLayoutStore.ts` | zustand |

**Week 3: Monaco Editor 통합**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| Monaco Editor 통합 | `src/renderer/components/editor/MonacoEditor.tsx` | @monaco-editor/react |
| 에디터 멀티탭 | `src/renderer/components/editor/EditorTabs.tsx`, `EditorPanel.tsx` | - |
| 에디터 스토어 | `src/renderer/stores/useEditorStore.ts` | zustand |
| Breadcrumb | `src/renderer/components/editor/Breadcrumb.tsx` | - |
| 검색/치환 기본 | `src/renderer/components/file-explorer/FileSearch.tsx` | - |
| Monaco 에디터 훅 | `src/renderer/hooks/useMonaco.ts` | - |

**Week 4: 테마 시스템 + i18n + 설정**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| 테마 정의 (Dark/Light/Monokai) | `src/renderer/themes/dark.ts`, `light.ts`, `monokai.ts`, `index.ts` | - |
| 테마 관리자 (Main) | `src/main/services/settings/ThemeManager.ts` | - |
| 테마 훅 + 적용 | `src/renderer/hooks/useTheme.ts` | - |
| i18n 기본 (한국어/영어) | `src/renderer/i18n/index.ts`, `ko.json`, `en.json` | - |
| i18n 훅 | `src/renderer/hooks/useI18n.ts` | - |
| 설정 패널 기본 | `src/renderer/components/settings/SettingsPanel.tsx`, `ThemeSettings.tsx`, `LanguageSettings.tsx`, `EditorSettings.tsx` | - |
| 설정 스토어 | `src/renderer/stores/useSettingsStore.ts` | zustand |
| 설정 서비스 (Main) | `src/main/services/settings/SettingsManager.ts` | electron-store |
| 설정 IPC | `src/main/ipc/settings.ipc.ts`, `src/preload/api/settings.api.ts` | - |

**Month 1 완료 기준:**
- Electron 앱이 실행되고 4패널 레이아웃이 표시됨
- 로컬 폴더를 열어 파일 탐색기에서 트리 구조가 보임
- Monaco Editor에서 파일을 열고 편집/저장 가능
- Dark/Light/Monokai 테마 전환 가능
- 한국어/영어 UI 전환 가능

---

#### Month 2: AI 채팅 + 코드 생성 + 기본 검증

**Week 5: AI 프로바이더 기반 구조**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| AI 어댑터 인터페이스 | `src/main/services/ai/adapters/BaseAIAdapter.ts` | - |
| Claude 어댑터 | `src/main/services/ai/adapters/ClaudeAdapter.ts` | @anthropic-ai/sdk |
| OpenAI 어댑터 | `src/main/services/ai/adapters/OpenAIAdapter.ts` | openai |
| AI 프로바이더 매니저 | `src/main/services/ai/AIProviderManager.ts` | - |
| 모델 라우터 | `src/main/services/ai/ModelRouter.ts` | - |
| 토큰 카운터 | `src/main/services/ai/TokenCounter.ts` | tiktoken |
| 스트림 핸들러 | `src/main/services/ai/StreamHandler.ts` | - |
| AI IPC | `src/main/ipc/ai.ipc.ts`, `src/preload/api/ai.api.ts` | - |
| AI 프로바이더 스토어 | `src/renderer/stores/useAIProviderStore.ts` | zustand |
| 공유 타입 | `src/shared/types/ai.ts` | - |

**Week 6: AI 채팅 UI**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| AI 채팅 패널 | `src/renderer/components/ai-chat/AIChatPanel.tsx` | - |
| 채팅 메시지 컴포넌트 | `src/renderer/components/ai-chat/ChatMessage.tsx` | - |
| 채팅 입력 | `src/renderer/components/ai-chat/ChatInput.tsx` | - |
| 코드블록 렌더링 | `src/renderer/components/ai-chat/CodeBlock.tsx` | react-syntax-highlighter |
| Diff 뷰 | `src/renderer/components/ai-chat/DiffView.tsx` | diff |
| 파일 생성 진행률 | `src/renderer/components/ai-chat/FileProgress.tsx` | - |
| 모델 셀렉터 | `src/renderer/components/ai-chat/ModelSelector.tsx` | - |
| 토큰 카운터 UI | `src/renderer/components/ai-chat/TokenCounter.tsx` | - |
| 컨텍스트 태거 | `src/renderer/components/ai-chat/ContextTagger.tsx` | - |
| AI 채팅 스토어 | `src/renderer/stores/useAIChatStore.ts` | zustand |
| AI 프로바이더 설정 패널 | `src/renderer/components/ai-provider/AIProviderPanel.tsx`, `ProviderCard.tsx`, `APIKeyInput.tsx` | - |

**Week 7: 코드 생성 통합 + AI 인라인 제안**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| AI → 에디터 코드 삽입 로직 | (기존 파일 수정) | - |
| 파일 생성/수정 자동화 | (AIProviderManager 확장) | - |
| AI 인라인 코드 제안 | Monaco InlineCompletionProvider 등록 | - |
| 승인/거절 UI | ChatMessage 확장 | - |
| 이미지 첨부 (멀티모달) | ChatInput 확장 | - |

**Week 8: 기본 보안/품질 검증 (ESLint)**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| ESLint 러너 | `src/main/services/security/ESLintRunner.ts` | eslint |
| Semgrep 러너 (기본 룰) | `src/main/services/security/SemgrepRunner.ts` | semgrep (CLI) |
| 시크릿 탐지기 | `src/main/services/security/SecretDetector.ts` | - |
| 통합 보안 스캐너 | `src/main/services/security/SecurityScanner.ts` | - |
| 보안 IPC | `src/main/ipc/security.ipc.ts`, `src/preload/api/security.api.ts` | - |
| 보안 리포트 UI | `src/renderer/components/security/SecurityPanel.tsx`, `SecurityReport.tsx`, `IssueCard.tsx`, `ScoreGauge.tsx` | - |
| 보안 스토어 | `src/renderer/stores/useSecurityStore.ts` | zustand |
| Problems 탭 연동 | `src/renderer/components/debug/ProblemsTab.tsx` | - |

**Month 2 완료 기준:**
- Claude/OpenAI 계정으로 로그인(API Key 입력) 가능
- AI 채팅에서 "쇼핑몰 만들어줘" 입력 시 코드 생성 + 파일 저장
- 스트리밍 응답이 채팅 UI에 실시간 표시
- 생성된 코드에 대해 ESLint + 기본 Semgrep 보안 스캔 자동 실행
- 보안/품질 점수와 이슈 목록 표시

---

#### Month 3: Git + 출시 + 미리보기/디버그 + 베타 출시

**Week 9: GitHub 로그인 + SSH 자동설정 + Git 기본**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| 인증 매니저 | `src/main/services/auth/AuthManager.ts` | - |
| OAuth 서비스 | `src/main/services/auth/OAuthService.ts` | - |
| keytar 래퍼 | `src/main/services/auth/KeychainService.ts` | keytar |
| 토큰 자동 갱신 | `src/main/services/auth/TokenRefresher.ts` | - |
| OAuth BrowserView | `src/main/windows/OAuthBrowserView.ts` | - |
| 인증 IPC | `src/main/ipc/auth.ipc.ts`, `src/preload/api/auth.api.ts` | - |
| Git 서비스 (simple-git) | `src/main/services/git/GitService.ts` | simple-git |
| GitHub API 서비스 | `src/main/services/git/GitHubService.ts` | - |
| SSH 키 매니저 | `src/main/services/git/SSHKeyManager.ts` | - |
| Git IPC | `src/main/ipc/git.ipc.ts`, `src/preload/api/git.api.ts` | - |
| 소스 컨트롤 패널 | `src/renderer/components/git/SourceControlPanel.tsx`, `ChangesList.tsx`, `CommitForm.tsx`, `BranchSelector.tsx` | - |
| Git 스토어 | `src/renderer/stores/useGitStore.ts` | zustand |
| GitHub 셋업 위자드 | `src/renderer/components/git/GitHubSetupWizard.tsx` | - |
| 인증 허브 UI | `src/renderer/components/auth/AccountHub.tsx`, `ServiceConnector.tsx`, `OAuthModal.tsx` | - |
| 인증 스토어 | `src/renderer/stores/useAuthStore.ts` | zustand |

**Week 10: Vercel 출시**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| 출시 매니저 | `src/main/services/deploy/DeployManager.ts` | - |
| Vercel 어댑터 | `src/main/services/deploy/adapters/VercelAdapter.ts`, `BaseDeployAdapter.ts` | - |
| 스택 분석기 | `src/main/services/deploy/StackAnalyzer.ts` | - |
| 환경변수 감지기 | `src/main/services/deploy/EnvDetector.ts` | - |
| 출시 IPC | `src/main/ipc/deploy.ipc.ts`, `src/preload/api/deploy.api.ts` | - |
| 출시 위자드 UI | `src/renderer/components/deploy/DeployWizard.tsx`, `DeployProgress.tsx`, `EnvVarsEditor.tsx` | - |
| 출시 대시보드 | `src/renderer/components/deploy/DeployDashboard.tsx`, `DeployHistory.tsx` | - |
| 출시 스토어 | `src/renderer/stores/useDeployStore.ts` | zustand |

**Week 11: 미리보기 + 디버그 콘솔 + 터미널**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| 미리보기 BrowserView | `src/main/windows/PreviewBrowserView.ts` | - |
| 미리보기 패널 | `src/renderer/components/preview/PreviewPanel.tsx`, `PreviewToolbar.tsx`, `ViewportSelector.tsx` | - |
| 터미널 매니저 | `src/main/services/terminal/TerminalManager.ts`, `ProcessRunner.ts` | node-pty |
| 터미널 IPC | `src/main/ipc/terminal.ipc.ts`, `src/preload/api/terminal.api.ts` | - |
| 디버그 패널 | `src/renderer/components/debug/DebugPanel.tsx` | - |
| Console 탭 | `src/renderer/components/debug/ConsoleTab.tsx` | - |
| Network 탭 | `src/renderer/components/debug/NetworkTab.tsx` | - |
| Terminal 탭 (xterm.js) | `src/renderer/components/debug/TerminalTab.tsx` | xterm, @xterm/addon-fit |
| 터미널 스토어 | `src/renderer/stores/useTerminalStore.ts` | zustand |

**Week 12: 서비스 대시보드 + 통합 테스트 + 베타 준비**

| 태스크 | 생성 파일 | 의존성 |
|--------|----------|--------|
| 서비스 대시보드 | `src/renderer/components/dashboard/Dashboard.tsx`, `ProjectCard.tsx`, `ProjectCreateModal.tsx`, `RecentActivity.tsx` | - |
| 프로젝트 스토어 | `src/renderer/stores/useProjectStore.ts` | zustand |
| 프로젝트 스캐너 | `src/main/services/fs/ProjectScanner.ts` | - |
| 전체 플로우 통합 테스트 | `tests/integration/full-flow.test.ts` | vitest |
| electron-builder 설정 | `electron-builder.yml` 완성 | electron-builder |
| CI/CD 파이프라인 | `.github/workflows/ci.yml`, `.github/workflows/release.yml` | - |
| 앱 아이콘/리소스 | `resources/icons/`, `resources/tray/` | - |

**Month 3 완료 기준 (MVP 베타):**
- GitHub OAuth 로그인 → SSH 키 자동 생성/등록 → Git 작업 가능
- Stage/Commit/Push/Pull 동작, AI 커밋 메시지 생성
- Vercel OAuth 연결 후 원클릭 출시
- 미리보기 패널에서 localhost 실시간 프리뷰
- 디버그 콘솔 4탭 (Console/Network/Problems/Terminal) 동작
- Mac/Windows/Linux 빌드 생성

---

### Phase 2 - +2개월 (Month 4~5)

> **목표:** 모니터링, 추가 출시 플랫폼, AI 자동 수정, 보안 고도화

#### Month 4: 워치독 모니터링 기본 + Railway/Fly.io

| 태스크 | 주요 파일 |
|--------|----------|
| 모니터링 서비스 기본 | `src/main/services/monitoring/MonitoringService.ts`, `MetricsCollector.ts` |
| 모니터링 에이전트 삽입기 | `src/main/services/monitoring/AgentInjector.ts` |
| 에러 트래커 | `src/main/services/monitoring/ErrorTracker.ts` |
| 로그 수집기 | `src/main/services/monitoring/LogAggregator.ts` |
| 모니터링 IPC | `src/main/ipc/monitoring.ipc.ts`, `src/preload/api/monitoring.api.ts` |
| 워치독 대시보드 UI | `src/renderer/components/monitoring/WatchdogDashboard.tsx`, `TrafficGraph.tsx`, `ResourceMonitor.tsx` |
| 에러 트래커 UI | `src/renderer/components/monitoring/ErrorTracker.tsx` |
| 로그 뷰어 UI | `src/renderer/components/monitoring/LogViewer.tsx` |
| 모니터링 스토어 | `src/renderer/stores/useMonitoringStore.ts` |
| Railway 어댑터 | `src/main/services/deploy/adapters/RailwayAdapter.ts` |
| Fly.io 어댑터 | `src/main/services/deploy/adapters/FlyioAdapter.ts` |

#### Month 5: AI 자동 수정 + 보안 고도화

| 태스크 | 주요 파일 |
|--------|----------|
| AI 코드 리뷰어 | `src/main/services/security/AICodeReviewer.ts` |
| 의존성 감사기 | `src/main/services/security/DependencyAuditor.ts` |
| 검증 게이트 고도화 | `src/renderer/components/security/VerificationGate.tsx` |
| AI 자동 수정 플로우 | AI 채팅 → 보안 이슈 자동 수정 제안 → 승인 → 적용 |
| 알림 매니저 | `src/main/services/monitoring/AlertManager.ts` |
| 알림 센터 UI | `src/renderer/components/monitoring/AlertCenter.tsx` |
| 비용 트래커 | `src/main/services/monitoring/CostTracker.ts` |
| 비용 트래커 UI | `src/renderer/components/monitoring/CostTracker.tsx` |
| Gemini 어댑터 | `src/main/services/ai/adapters/GeminiAdapter.ts` |
| Ollama 어댑터 | `src/main/services/ai/adapters/OllamaAdapter.ts` |
| 추가 테마 (6개) | `src/renderer/themes/dracula.ts`, `nord.ts`, `github.ts`, `one-dark.ts`, `tokyo-night.ts`, `solarized.ts` |

**Phase 2 완료 기준:**
- 출시된 앱의 실시간 메트릭 (트래픽, 응답시간, 에러율) 표시
- 에러 트래킹 + AI 원인 분석
- Railway/Fly.io 배포 지원
- AI가 보안 이슈를 감지하고 자동 수정 코드를 제안
- Semgrep OWASP 전체 룰 + 의존성 CVE 체크

---

### Phase 3 - +2개월 (Month 6~7)

> **목표:** 팀 협업, PR 자동화, AWS/GCP, 오토스케일링

| 태스크 | 상세 |
|--------|------|
| 팀 협업 기본 | 프로젝트 공유, 역할/권한 관리 |
| PR 리뷰 자동화 | AI가 PR diff를 분석하여 리뷰 코멘트 자동 생성 |
| PR 패널 확장 | `src/renderer/components/git/PullRequestPanel.tsx`, `IssuePanel.tsx`, `ActionsPanel.tsx` |
| Git 히스토리 그래프 뷰 | `src/renderer/components/git/GitHistory.tsx` |
| 머지 충돌 해결 UI | `src/renderer/components/git/MergeConflictView.tsx` |
| AWS 배포 어댑터 | `src/main/services/deploy/adapters/AWSAdapter.ts` |
| GCP 배포 어댑터 | `src/main/services/deploy/adapters/GCPAdapter.ts` |
| Netlify 어댑터 | `src/main/services/deploy/adapters/NetlifyAdapter.ts` |
| 오토스케일링 | `src/renderer/components/monitoring/ScalingControl.tsx` |
| AIOps 기본 | `src/renderer/components/monitoring/AIOpsPanel.tsx` |
| 출시 파이프라인 설정 | `src/main/services/deploy/PipelineManager.ts`, `src/renderer/components/deploy/PipelineConfig.tsx` |
| 도메인/SSL 관리 | `src/renderer/components/deploy/DomainManager.tsx` |
| 롤백 패널 | `src/renderer/components/deploy/RollbackPanel.tsx` |

**Phase 3 완료 기준:**
- PR 생성 시 AI가 자동으로 리뷰 코멘트 작성
- AWS ECS/Lambda, GCP Cloud Run 배포 지원
- 오토스케일링 규칙 설정 및 자동 스케일 업/다운
- 도메인 연결, SSL 자동 발급

---

### Phase 4 - +3개월 (Month 8~10)

> **목표:** 마켓플레이스, B2B, 엔터프라이즈

| 태스크 | 상세 |
|--------|------|
| 프로젝트 매칭 마켓플레이스 | 개발자-비개발자 매칭, 프로젝트 의뢰/수주 |
| AI 설정 동기화 (B2B) | 팀 전체의 AI 설정/프롬프트 동기화 |
| 엔터프라이즈 기능 | SSO (SAML/OIDC), 감사 로그, 관리자 콘솔 |
| 자체 서버 배포 | SSH + Docker Compose로 자체 인프라 배포 |
| Cloudflare/DigitalOcean 어댑터 | 추가 출시 플랫폼 |
| VSCode 테마 임포트 | `src/renderer/themes/vscode-compat.ts` |
| 키바인딩 커스터마이즈 | `src/renderer/components/settings/KeybindingSettings.tsx` |
| 접근성 고도화 | 고대비 모드, 스크린 리더 ARIA 지원 |
| 추가 i18n | 일본어, 중국어, 기타 언어 번역 파일 |
| OpenRouter 어댑터 | `src/main/services/ai/adapters/OpenRouterAdapter.ts` |

---

## 4. API 설계

### 4.1 IPC 채널 목록 (Main ↔ Renderer)

```typescript
// src/shared/types/ipc-channels.ts

export const IPC_CHANNELS = {
  // ── 파일시스템 ──
  FS: {
    READ_FILE:         'fs:read-file',
    WRITE_FILE:        'fs:write-file',
    CREATE_FILE:       'fs:create-file',
    DELETE_FILE:       'fs:delete-file',
    RENAME_FILE:       'fs:rename-file',
    CREATE_DIR:        'fs:create-dir',
    DELETE_DIR:        'fs:delete-dir',
    READ_DIR:          'fs:read-dir',
    WATCH_DIR:         'fs:watch-dir',
    UNWATCH_DIR:       'fs:unwatch-dir',
    GET_FILE_STAT:     'fs:get-file-stat',
    SEARCH_FILES:      'fs:search-files',
    OPEN_FOLDER:       'fs:open-folder',      // dialog.showOpenDialog
    // Events (Main → Renderer)
    FILE_CHANGED:      'fs:file-changed',
    FILE_CREATED:      'fs:file-created',
    FILE_DELETED:       'fs:file-deleted',
  },

  // ── AI ──
  AI: {
    CHAT:              'ai:chat',              // 일반 채팅
    GENERATE_CODE:     'ai:generate-code',     // 코드 생성
    REVIEW_CODE:       'ai:review-code',       // 코드 리뷰
    GENERATE_COMMIT:   'ai:generate-commit',   // 커밋 메시지 생성
    GENERATE_PR:       'ai:generate-pr',       // PR 본문 생성
    SUGGEST_FIX:       'ai:suggest-fix',       // 에러 수정 제안
    CANCEL_REQUEST:    'ai:cancel-request',
    GET_MODELS:        'ai:get-models',
    GET_USAGE:         'ai:get-usage',         // 토큰 사용량 조회
    // Events
    STREAM_CHUNK:      'ai:stream-chunk',      // 스트리밍 청크
    STREAM_END:        'ai:stream-end',
    STREAM_ERROR:      'ai:stream-error',
  },

  // ── AI 프로바이더 ──
  AI_PROVIDER: {
    LIST_PROVIDERS:    'ai-provider:list',
    CONNECT:           'ai-provider:connect',
    DISCONNECT:        'ai-provider:disconnect',
    SET_API_KEY:       'ai-provider:set-api-key',
    GET_STATUS:        'ai-provider:get-status',
    SET_MODEL_ROUTING: 'ai-provider:set-model-routing',
    DETECT_OLLAMA:     'ai-provider:detect-ollama',
  },

  // ── Git ──
  GIT: {
    STATUS:            'git:status',
    STAGE:             'git:stage',
    UNSTAGE:           'git:unstage',
    COMMIT:            'git:commit',
    PUSH:              'git:push',
    PULL:              'git:pull',
    FETCH:             'git:fetch',
    BRANCH_LIST:       'git:branch-list',
    BRANCH_CREATE:     'git:branch-create',
    BRANCH_DELETE:     'git:branch-delete',
    BRANCH_CHECKOUT:   'git:branch-checkout',
    MERGE:             'git:merge',
    STASH:             'git:stash',
    STASH_POP:         'git:stash-pop',
    LOG:               'git:log',
    DIFF:              'git:diff',
    BLAME:             'git:blame',
    CLONE:             'git:clone',
    INIT:              'git:init',
  },

  // ── GitHub ──
  GITHUB: {
    GET_USER:          'github:get-user',
    LIST_REPOS:        'github:list-repos',
    CREATE_REPO:       'github:create-repo',
    CREATE_PR:         'github:create-pr',
    LIST_PRS:          'github:list-prs',
    MERGE_PR:          'github:merge-pr',
    CREATE_ISSUE:      'github:create-issue',
    LIST_ISSUES:       'github:list-issues',
    GET_ACTIONS:       'github:get-actions',
    SETUP_SSH:         'github:setup-ssh',
  },

  // ── 배포 ──
  DEPLOY: {
    ANALYZE_STACK:     'deploy:analyze-stack',
    RECOMMEND_PLATFORM:'deploy:recommend-platform',
    DETECT_ENV_VARS:   'deploy:detect-env-vars',
    VERIFY_PRE_DEPLOY: 'deploy:verify-pre-deploy',
    START_DEPLOY:      'deploy:start-deploy',
    GET_DEPLOY_STATUS: 'deploy:get-status',
    ROLLBACK:          'deploy:rollback',
    GET_HISTORY:       'deploy:get-history',
    SET_ENV_VARS:      'deploy:set-env-vars',
    GET_ENV_VARS:      'deploy:get-env-vars',
    ADD_DOMAIN:        'deploy:add-domain',
    CHECK_DNS:         'deploy:check-dns',
    GET_DOMAINS:       'deploy:get-domains',
    SET_PIPELINE:      'deploy:set-pipeline',
    // Events
    DEPLOY_LOG:        'deploy:log',
    DEPLOY_PROGRESS:   'deploy:progress',
    DEPLOY_COMPLETE:   'deploy:complete',
    DEPLOY_FAILED:     'deploy:failed',
  },

  // ── 보안/검증 ──
  SECURITY: {
    SCAN_FILE:         'security:scan-file',
    SCAN_PROJECT:      'security:scan-project',
    GET_REPORT:        'security:get-report',
    AUTO_FIX:          'security:auto-fix',
    // Events
    SCAN_PROGRESS:     'security:scan-progress',
    SCAN_COMPLETE:     'security:scan-complete',
  },

  // ── 터미널 ──
  TERMINAL: {
    CREATE:            'terminal:create',
    WRITE:             'terminal:write',
    RESIZE:            'terminal:resize',
    CLOSE:             'terminal:close',
    LIST:              'terminal:list',
    // Events
    DATA:              'terminal:data',        // stdout 스트리밍
    EXIT:              'terminal:exit',
  },

  // ── 모니터링 ──
  MONITORING: {
    GET_METRICS:       'monitoring:get-metrics',
    GET_ERRORS:        'monitoring:get-errors',
    GET_LOGS:          'monitoring:get-logs',
    GET_ALERTS:        'monitoring:get-alerts',
    SET_ALERT_RULES:   'monitoring:set-alert-rules',
    GET_COST:          'monitoring:get-cost',
    SET_BUDGET:        'monitoring:set-budget',
    SET_AUTOSCALE:     'monitoring:set-autoscale',
    RESOLVE_ERROR:     'monitoring:resolve-error',
    // Events
    METRICS_UPDATE:    'monitoring:metrics-update',
    NEW_ERROR:         'monitoring:new-error',
    NEW_LOG:           'monitoring:new-log',
    ALERT_FIRED:       'monitoring:alert-fired',
  },

  // ── 인증 ──
  AUTH: {
    START_OAUTH:       'auth:start-oauth',
    SET_TOKEN:         'auth:set-token',
    GET_TOKEN:         'auth:get-token',
    DELETE_TOKEN:       'auth:delete-token',
    LIST_CONNECTIONS:  'auth:list-connections',
    REFRESH_TOKEN:     'auth:refresh-token',
    // Events
    OAUTH_SUCCESS:     'auth:oauth-success',
    OAUTH_FAILURE:     'auth:oauth-failure',
    TOKEN_EXPIRED:     'auth:token-expired',
  },

  // ── 설정 ──
  SETTINGS: {
    GET_ALL:           'settings:get-all',
    GET:               'settings:get',
    SET:               'settings:set',
    RESET:             'settings:reset',
    IMPORT_THEME:      'settings:import-theme',
    EXPORT_THEME:      'settings:export-theme',
    // Events
    CHANGED:           'settings:changed',
  },

  // ── 프로젝트 ──
  PROJECT: {
    LIST:              'project:list',
    CREATE:            'project:create',
    UPDATE:            'project:update',
    DELETE:            'project:delete',
    OPEN:              'project:open',
    LINK_FOLDER:       'project:link-folder',
    GET_ACTIVITY:      'project:get-activity',
  },

  // ── 앱 ──
  APP: {
    GET_VERSION:       'app:get-version',
    CHECK_UPDATE:      'app:check-update',
    INSTALL_UPDATE:    'app:install-update',
    QUIT:              'app:quit',
    MINIMIZE:          'app:minimize',
    MAXIMIZE:          'app:maximize',
    GET_PLATFORM:      'app:get-platform',
  },
} as const;
```

### 4.2 AI 프로바이더 어댑터 인터페이스

```typescript
// src/shared/types/ai.ts

interface AIModel {
  id: string;
  name: string;
  provider: string;
  contextWindow: number;
  inputPricePerMToken: number;
  outputPricePerMToken: number;
  supportsVision: boolean;
  supportsStreaming: boolean;
}

interface ChatParams {
  messages: ChatMessage[];
  model: string;
  temperature?: number;
  maxTokens?: number;
  systemPrompt?: string;
  files?: FileContext[];         // @파일명 태깅된 파일들
  images?: ImageAttachment[];    // 멀티모달 이미지
}

interface ChatStreamChunk {
  type: 'text' | 'code' | 'thinking';
  content: string;
  model: string;
  usage?: { inputTokens: number; outputTokens: number };
}

interface CodeGenParams {
  prompt: string;
  projectContext: ProjectContext;
  existingFiles: FileContext[];
  targetFiles?: string[];
  language?: string;
}

interface CodeStreamChunk {
  type: 'file-start' | 'file-content' | 'file-end' | 'status';
  filePath?: string;
  content?: string;
  status?: 'creating' | 'modifying' | 'complete' | 'error';
}

interface CodeReviewParams {
  files: FileContext[];
  diff?: string;
  focusAreas?: ('security' | 'performance' | 'quality' | 'all')[];
}

interface ReviewResult {
  overallAssessment: string;
  issues: ReviewIssue[];
  suggestions: string[];
}

interface AuthResult {
  success: boolean;
  provider: string;
  userId?: string;
  subscription?: SubscriptionInfo;
  availableModels?: AIModel[];
  error?: string;
}

interface SubscriptionInfo {
  plan: 'free' | 'pro' | 'plus' | 'team' | 'enterprise';
  usageLimits?: {
    requestsPerDay?: number;
    tokensPerDay?: number;
  };
}
```

### 4.3 Git 서비스 인터페이스

```typescript
// src/shared/types/git.ts

interface GitStatus {
  branch: string;
  ahead: number;
  behind: number;
  staged: FileChange[];
  unstaged: FileChange[];
  untracked: string[];
  conflicts: string[];
}

interface FileChange {
  path: string;
  status: 'added' | 'modified' | 'deleted' | 'renamed';
  oldPath?: string;       // renamed인 경우
  insertions?: number;
  deletions?: number;
}

interface CommitInfo {
  hash: string;
  shortHash: string;
  message: string;
  author: string;
  email: string;
  date: Date;
  parentHashes: string[];
}

interface BranchInfo {
  name: string;
  current: boolean;
  remote: boolean;
  tracking?: string;
  ahead?: number;
  behind?: number;
  lastCommit?: CommitInfo;
}

interface PullRequest {
  number: number;
  title: string;
  body: string;
  state: 'open' | 'closed' | 'merged';
  fromBranch: string;
  toBranch: string;
  author: string;
  reviewers: string[];
  labels: string[];
  additions: number;
  deletions: number;
  changedFiles: number;
  createdAt: Date;
  updatedAt: Date;
}

interface MergeConflict {
  file: string;
  currentContent: string;
  incomingContent: string;
  baseContent: string;
}

// Git 서비스 인터페이스 (Main Process)
interface IGitService {
  init(path: string): Promise<void>;
  clone(url: string, path: string): Promise<void>;
  status(path: string): Promise<GitStatus>;
  stage(path: string, files: string[]): Promise<void>;
  unstage(path: string, files: string[]): Promise<void>;
  commit(path: string, message: string): Promise<CommitInfo>;
  push(path: string, remote?: string, branch?: string): Promise<void>;
  pull(path: string, remote?: string, branch?: string): Promise<void>;
  fetch(path: string): Promise<void>;
  getBranches(path: string): Promise<BranchInfo[]>;
  createBranch(path: string, name: string): Promise<void>;
  checkoutBranch(path: string, name: string): Promise<void>;
  deleteBranch(path: string, name: string): Promise<void>;
  merge(path: string, branch: string): Promise<MergeConflict[]>;
  log(path: string, limit?: number): Promise<CommitInfo[]>;
  diff(path: string, file?: string): Promise<string>;
  blame(path: string, file: string): Promise<BlameLine[]>;
  stash(path: string): Promise<void>;
  stashPop(path: string): Promise<void>;
}

// GitHub 서비스 인터페이스
interface IGitHubService {
  getUser(): Promise<GitHubUser>;
  listRepos(): Promise<GitHubRepo[]>;
  createRepo(name: string, options: CreateRepoOptions): Promise<GitHubRepo>;
  createPR(params: CreatePRParams): Promise<PullRequest>;
  listPRs(repo: string): Promise<PullRequest[]>;
  mergePR(repo: string, prNumber: number, method: MergeMethod): Promise<void>;
  createIssue(repo: string, params: CreateIssueParams): Promise<Issue>;
  listIssues(repo: string): Promise<Issue[]>;
  getActions(repo: string): Promise<ActionRun[]>;
  registerSSHKey(publicKey: string): Promise<void>;
}
```

### 4.4 출시 플랫폼 어댑터 인터페이스

```typescript
// src/shared/types/deploy.ts

interface StackInfo {
  framework: string;          // 'nextjs' | 'react' | 'express' | 'flask' 등
  runtime: string;            // 'node20' | 'python3.12' 등
  packageManager: string;     // 'npm' | 'pnpm' | 'yarn' | 'pip'
  database?: string;          // 'supabase' | 'firebase' | 'planetscale'
  auth?: string;              // 'nextauth' | 'firebase-auth'
  payments?: string;          // 'stripe' | 'toss'
  storage?: string;           // 's3' | 'r2'
  detectedEnvVars: string[];
}

interface DeployConfig {
  projectId: string;
  platformId: string;
  environment: 'preview' | 'staging' | 'production';
  branch: string;
  envVars: EnvVar[];
  buildCommand?: string;
  outputDir?: string;
  installCommand?: string;
  rootDir?: string;
}

interface DeployEvent {
  type: 'log' | 'progress' | 'status' | 'error' | 'complete';
  message: string;
  progress?: number;          // 0~100
  timestamp: Date;
  deployId?: string;
  url?: string;               // 완료 시 출시 URL
}

interface DeployRecord {
  id: string;
  version: string;
  commitHash: string;
  commitMessage: string;
  environment: string;
  platform: string;
  status: 'building' | 'deploying' | 'success' | 'failed' | 'rolled-back';
  url?: string;
  createdAt: Date;
  duration?: number;          // 밀리초
}

interface EnvVar {
  key: string;
  value: string;
  isSecret: boolean;
  source: 'auto-detected' | 'env-file' | 'auto-generated' | 'manual';
}

interface CostEstimate {
  platform: string;
  plan: string;
  monthlyCost: number;
  breakdown: { item: string; cost: number }[];
}

interface DomainConfig {
  domain: string;
  type: 'primary' | 'redirect' | 'subdomain';
  sslStatus: 'pending' | 'active' | 'expired';
  dnsRecords: DNSRecord[];
}

interface RollbackPolicy {
  autoRollbackEnabled: boolean;
  errorRateThreshold: number;       // 에러율 (%) - 이 값 초과 시 롤백
  windowMinutes: number;            // 배포 후 모니터링 시간
  maxVersionsToKeep: number;        // 롤백 가능한 최대 버전 수
}

// 어댑터 인터페이스
interface IDeployAdapter {
  readonly platformId: string;
  readonly platformName: string;

  authenticate(token: OAuthToken | APIToken): Promise<void>;
  analyzeCompatibility(stack: StackInfo): CompatibilityResult;
  estimateCost(stack: StackInfo): CostEstimate;

  deploy(config: DeployConfig): AsyncGenerator<DeployEvent>;
  getDeployStatus(deployId: string): Promise<DeployStatus>;
  rollback(deployId: string, targetVersion: string): Promise<void>;
  deleteDeploy(deployId: string): Promise<void>;

  setEnvVars(projectId: string, vars: EnvVar[]): Promise<void>;
  getEnvVars(projectId: string): Promise<EnvVar[]>;

  addDomain(projectId: string, domain: string): Promise<DomainConfig>;
  getDomains(projectId: string): Promise<DomainConfig[]>;
  checkDNS(domain: string): Promise<DNSStatus>;

  getLogs(deployId: string): AsyncGenerator<LogEntry>;
  getDeployHistory(projectId: string): Promise<DeployRecord[]>;
}
```

---

## 5. 데이터 모델

### 5.1 로컬 저장소 (SQLite + electron-store + keytar)

VidEplace는 Electron 앱이므로 대부분의 데이터를 로컬에 저장한다.

| 저장소 | 용도 | 기술 |
|--------|------|------|
| SQLite (better-sqlite3) | 프로젝트, AI 대화 히스토리, 모니터링 데이터, 에러 로그 | better-sqlite3 |
| electron-store | 앱 설정, 테마, 레이아웃 상태 | electron-store |
| keytar (OS 키체인) | OAuth 토큰, API 키, 시크릿 | keytar |

### 5.2 엔티티 정의

```typescript
// ── Project ──
interface Project {
  id: string;                    // UUID
  name: string;
  description?: string;
  localPath: string;             // 로컬 디렉토리 경로
  stack: StackInfo;              // 감지된 기술 스택
  status: 'developing' | 'verifying' | 'deployed' | 'error';
  gitRemoteUrl?: string;
  deployments: DeploymentRef[];
  createdAt: Date;
  updatedAt: Date;
  lastOpenedAt: Date;
}

// ── User (VidEplace 계정 — Phase 4+) ──
interface User {
  id: string;
  email: string;
  name: string;
  plan: 'free' | 'pro' | 'team' | 'enterprise';
  planExpiresAt?: Date;
  preferences: UserPreferences;
  createdAt: Date;
}

interface UserPreferences {
  theme: string;
  language: string;
  editorSettings: EditorSettings;
  keyBindingPreset: 'videplace' | 'vscode' | 'vim' | 'emacs';
  aiResponseLanguage: 'ui-sync' | 'always-en' | 'always-ko' | 'auto';
  codeCommentLanguage: 'ko' | 'en' | 'none';
  commitMessageLanguage: 'ko' | 'en';
}

// ── AIProvider (연결된 AI 프로바이더) ──
interface AIProvider {
  id: string;
  providerId: 'claude' | 'openai' | 'gemini' | 'ollama' | 'openrouter';
  connectionType: 'oauth' | 'api-key' | 'local';
  isConnected: boolean;
  userId?: string;               // 프로바이더 측 사용자 ID
  subscription?: SubscriptionInfo;
  availableModels: AIModel[];
  connectedAt?: Date;
  // 토큰/키는 keytar에 별도 저장 (이 모델에 포함하지 않음)
}

interface ModelRouting {
  codeGeneration: ModelRef;
  codeReview: ModelRef;
  commitMessage: ModelRef;
  debugging: ModelRef;
  prDescription: ModelRef;
  general: ModelRef;
}

interface ModelRef {
  providerId: string;
  modelId: string;
}

// ── AIConversation (AI 대화 히스토리) ──
interface AIConversation {
  id: string;
  projectId: string;
  title: string;
  messages: AIMessage[];
  modelId: string;
  totalTokens: number;
  totalCost: number;
  createdAt: Date;
  updatedAt: Date;
}

interface AIMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  attachments?: Attachment[];
  codeBlocks?: CodeBlock[];
  fileChanges?: FileChange[];
  tokenCount: number;
  model: string;
  timestamp: Date;
  parentId?: string;             // 대화 분기용
}

// ── GitConfig ──
interface GitConfig {
  projectId: string;
  userName: string;
  userEmail: string;
  remoteName: string;            // 'origin'
  remoteUrl: string;
  defaultBranch: string;         // 'main'
  sshKeyPath?: string;
  autoFetchInterval: number;     // 분 (0 = 수동)
  commitFormat: 'conventional' | 'free';
}

// ── DeployConfig ──
interface DeployConfig {
  id: string;
  projectId: string;
  platformId: string;
  platformProjectId?: string;    // 플랫폼 측 프로젝트 ID
  environments: EnvironmentConfig[];
  pipeline: DeployPipeline;
  domains: DomainConfig[];
  createdAt: Date;
  updatedAt: Date;
}

interface EnvironmentConfig {
  name: 'preview' | 'staging' | 'production';
  branch: string;
  autoDeployEnabled: boolean;
  requireApproval: boolean;
  envVars: EnvVar[];             // 환경변수 (값은 keytar에 저장)
  url?: string;
}

// ── MonitoringData ──
interface MonitoringData {
  projectId: string;
  deployId: string;
  metrics: TimeSeriesMetric[];
  errors: TrackedError[];
  logs: LogEntry[];
  alerts: Alert[];
  costData: CostData;
  autoScaleConfig?: AutoScaleConfig;
}

interface TimeSeriesMetric {
  name: string;                  // 'requests_per_second', 'response_time_p95', etc.
  dataPoints: { timestamp: Date; value: number }[];
}

interface Alert {
  id: string;
  ruleId: string;
  severity: 'info' | 'warning' | 'critical';
  message: string;
  status: 'firing' | 'resolved';
  firedAt: Date;
  resolvedAt?: Date;
  channels: string[];            // 'app', 'slack', 'email' 등
}

interface CostData {
  currentMonth: {
    total: number;
    breakdown: { service: string; cost: number }[];
  };
  budget: number;
  history: { month: string; total: number }[];
}

// ── SecurityReport ──
interface SecurityReport {
  id: string;
  projectId: string;
  securityScore: number;
  qualityScore: number;
  overallScore: number;
  issues: VerificationIssue[];
  canDeploy: boolean;
  scanDuration: number;          // 밀리초
  scannedAt: Date;
  tools: string[];               // 사용된 도구 목록
}

// ── EditorSettings ──
interface EditorSettings {
  fontFamily: string;
  fontSize: number;
  lineHeight: number;
  ligatures: boolean;
  tabSize: number;
  insertSpaces: boolean;
  autoSave: boolean;
  autoSaveDelay: number;         // 밀리초
  wordWrap: 'on' | 'off' | 'bounded';
  wordWrapColumn: number;
  minimap: boolean;
  lineNumbers: boolean;
  bracketMatching: boolean;
  autoComplete: boolean;
  formatOnSave: boolean;
  aiInlineSuggestions: boolean;
}

// ── ServiceConnection (인증 허브) ──
interface ServiceConnection {
  serviceId: string;             // 'github', 'claude', 'vercel' 등
  serviceName: string;
  category: 'ai' | 'git' | 'deploy' | 'database' | 'payment' | 'notification';
  isConnected: boolean;
  connectionType: 'oauth' | 'api-key' | 'bot-token' | 'local';
  userId?: string;
  displayName?: string;
  connectedAt?: Date;
  expiresAt?: Date;
  metadata?: Record<string, unknown>;
  // 토큰은 keytar에 저장
}
```

### 5.3 SQLite 스키마 (핵심 테이블)

```sql
-- 프로젝트
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  local_path TEXT NOT NULL,
  stack_json TEXT,              -- JSON: StackInfo
  status TEXT DEFAULT 'developing',
  git_remote_url TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  last_opened_at DATETIME
);

-- AI 대화
CREATE TABLE ai_conversations (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id),
  title TEXT,
  model_id TEXT,
  total_tokens INTEGER DEFAULT 0,
  total_cost REAL DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE ai_messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT REFERENCES ai_conversations(id),
  parent_id TEXT,               -- 대화 분기용
  role TEXT NOT NULL,           -- 'user' | 'assistant' | 'system'
  content TEXT NOT NULL,
  attachments_json TEXT,
  token_count INTEGER DEFAULT 0,
  model TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 보안 리포트
CREATE TABLE security_reports (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id),
  security_score INTEGER,
  quality_score INTEGER,
  overall_score INTEGER,
  issues_json TEXT,             -- JSON: VerificationIssue[]
  can_deploy BOOLEAN,
  scan_duration INTEGER,
  scanned_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 배포 기록
CREATE TABLE deploy_records (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id),
  platform_id TEXT,
  environment TEXT,
  version TEXT,
  commit_hash TEXT,
  commit_message TEXT,
  status TEXT,
  url TEXT,
  duration INTEGER,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 모니터링 메트릭 (시계열)
CREATE TABLE metrics (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT REFERENCES projects(id),
  metric_name TEXT NOT NULL,
  value REAL NOT NULL,
  timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_metrics_project_time ON metrics(project_id, metric_name, timestamp);

-- 에러 트래킹
CREATE TABLE tracked_errors (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id),
  type TEXT,
  message TEXT,
  stack_trace TEXT,
  file TEXT,
  line INTEGER,
  occurrence_count INTEGER DEFAULT 1,
  affected_users INTEGER DEFAULT 0,
  severity TEXT,
  status TEXT DEFAULT 'open',
  ai_analysis TEXT,
  ai_suggested_fix TEXT,
  first_occurrence DATETIME DEFAULT CURRENT_TIMESTAMP,
  last_occurrence DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 서비스 연결 (토큰은 keytar에)
CREATE TABLE service_connections (
  service_id TEXT PRIMARY KEY,
  service_name TEXT,
  category TEXT,
  is_connected BOOLEAN DEFAULT 0,
  connection_type TEXT,
  user_id TEXT,
  display_name TEXT,
  connected_at DATETIME,
  expires_at DATETIME,
  metadata_json TEXT
);

-- 활동 로그
CREATE TABLE activity_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT REFERENCES projects(id),
  type TEXT,                    -- 'code_gen', 'deploy', 'commit', 'error', 'scan'
  message TEXT,
  metadata_json TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

---

## 6. 보안 고려사항

### 6.1 토큰/키 저장 (keytar)

**원칙:** 모든 민감 정보는 OS 키체인에 저장. 디스크에 평문 저장 절대 불가.

| 데이터 | 저장소 | 상세 |
|--------|--------|------|
| OAuth 토큰 (GitHub, Vercel 등) | keytar (OS 키체인) | macOS Keychain, Windows Credential Store, Linux libsecret |
| AI API 키 | keytar | 프로바이더별 개별 저장 |
| AI OAuth 토큰 | keytar | Claude, OpenAI, Google |
| SSH 개인키 | 파일시스템 (~/.ssh/) | 파일 퍼미션 600, 패스프레이즈 옵션 |
| 배포 환경변수 값 | keytar | 프로젝트 + 환경별 네임스페이스 |
| 세션 토큰 | 메모리 (Electron Main Process) | 앱 종료 시 자동 삭제 |

**keytar 네이밍 컨벤션:**

```
서비스명: "videplace"
계정명 형식: "{category}:{serviceId}:{key}"

예시:
- "ai:claude:oauth-token"
- "ai:openai:api-key"
- "git:github:oauth-token"
- "deploy:vercel:oauth-token"
- "deploy:env:project-abc:production:STRIPE_SECRET_KEY"
```

**보안 조치:**

```typescript
class KeychainService {
  // 토큰 저장 시 메타데이터도 함께 관리
  async saveToken(key: string, token: string): Promise<void> {
    // 1. keytar에 암호화 저장
    await keytar.setPassword('videplace', key, token);
    // 2. 만료 시간 메타데이터 저장 (별도 electron-store)
    this.saveTokenMeta(key, { savedAt: Date.now() });
  }

  // 만료된 토큰 자동 정리
  async cleanExpiredTokens(): Promise<void>;

  // 앱 삭제 시 모든 keytar 항목 정리
  async clearAll(): Promise<void>;
}
```

### 6.2 OAuth 플로우 보안

#### BrowserView 기반 OAuth 보안

```
보안 위협                           대응
──────────                         ────
피싱 (가짜 로그인 페이지)             → URL 화이트리스트 검증, SSL 인증서 확인
토큰 가로채기                        → 콜백 URL을 localhost 루프백으로 제한
중간자 공격                          → HTTPS only, certificate pinning
XSS (BrowserView 내부)              → nodeIntegration: false, contextIsolation: true
토큰 노출 (URL fragment)             → 인증 코드 플로우만 사용 (Implicit 금지)
CSRF                                → state 파라미터 + PKCE
```

**OAuth 구현 체크리스트:**

```typescript
class OAuthService {
  async startOAuth(service: OAuthServiceConfig): Promise<OAuthToken> {
    // 1. state 파라미터 생성 (CSRF 방지)
    const state = crypto.randomBytes(32).toString('hex');

    // 2. PKCE code_verifier + code_challenge 생성
    const codeVerifier = crypto.randomBytes(32).toString('base64url');
    const codeChallenge = crypto
      .createHash('sha256')
      .update(codeVerifier)
      .digest('base64url');

    // 3. BrowserView 생성 (보안 설정)
    const view = new BrowserView({
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        sandbox: true,
        // JavaScript 비활성화 불가 (OAuth 페이지 필요)
      }
    });

    // 4. URL 화이트리스트 검증
    view.webContents.on('will-navigate', (event, url) => {
      if (!this.isAllowedUrl(url, service)) {
        event.preventDefault();
      }
    });

    // 5. HTTPS only 강제
    view.webContents.on('certificate-error', (event) => {
      event.preventDefault(); // 잘못된 인증서 차단
    });

    // 6. 인증 코드 수신 → 토큰 교환
    // 7. 토큰을 keytar에 저장
    // 8. BrowserView 파괴
  }

  private isAllowedUrl(url: string, service: OAuthServiceConfig): boolean {
    const allowed = [
      service.authUrl,
      service.tokenUrl,
      service.callbackUrl,
      ...service.allowedDomains, // 예: ['github.com', 'accounts.google.com']
    ];
    return allowed.some(domain => new URL(url).hostname.endsWith(domain));
  }
}
```

#### 토큰 자동 갱신

```typescript
class TokenRefresher {
  private refreshTimers: Map<string, NodeJS.Timeout> = new Map();

  // 토큰 저장 시 갱신 타이머 등록
  scheduleRefresh(serviceId: string, expiresIn: number): void {
    const refreshAt = expiresIn * 0.8; // 만료 20% 전에 갱신
    const timer = setTimeout(() => this.refresh(serviceId), refreshAt * 1000);
    this.refreshTimers.set(serviceId, timer);
  }

  private async refresh(serviceId: string): Promise<void> {
    try {
      const refreshToken = await this.keychain.getToken(`${serviceId}:refresh`);
      if (!refreshToken) throw new Error('No refresh token');

      const newTokens = await this.exchangeRefreshToken(serviceId, refreshToken);
      await this.keychain.saveToken(`${serviceId}:access`, newTokens.accessToken);

      if (newTokens.refreshToken) {
        await this.keychain.saveToken(`${serviceId}:refresh`, newTokens.refreshToken);
      }

      this.scheduleRefresh(serviceId, newTokens.expiresIn);
    } catch (error) {
      // 갱신 실패 → Renderer에 재로그인 요청 이벤트 발송
      this.mainWindow.webContents.send(IPC_CHANNELS.AUTH.TOKEN_EXPIRED, serviceId);
    }
  }
}
```

### 6.3 환경변수 암호화

#### 배포 환경변수 보안

```
환경변수 값 입력
    │
    ├─ 로컬 저장: keytar (OS 키체인)
    │   └─ 키: "deploy:env:{projectId}:{env}:{VAR_NAME}"
    │
    ├─ 배포 시: 출시 플랫폼 API로 암호화 전송
    │   └─ HTTPS only
    │   └─ 플랫폼 측에서 암호화 저장 (Vercel Secrets, Railway Variables 등)
    │
    └─ 코드 노출 검사: SecretDetector가 소스 코드 내 환경변수 값 노출 검사
        └─ .env 파일이 .gitignore에 포함되어 있는지 확인
        └─ 코드에서 하드코딩된 API 키/시크릿 탐지
```

#### SecretDetector 구현

```typescript
class SecretDetector {
  private patterns: SecretPattern[] = [
    { name: 'AWS Access Key',    regex: /AKIA[0-9A-Z]{16}/g,                      severity: 'HIGH' },
    { name: 'AWS Secret Key',    regex: /[0-9a-zA-Z/+]{40}/g,                     severity: 'HIGH' },
    { name: 'GitHub Token',      regex: /gh[ps]_[A-Za-z0-9_]{36,}/g,             severity: 'HIGH' },
    { name: 'Stripe Secret Key', regex: /sk_live_[0-9a-zA-Z]{24,}/g,             severity: 'HIGH' },
    { name: 'Generic API Key',   regex: /['\"]?[a-zA-Z_]*(?:api|secret|key|token|password)['\"]?\s*[:=]\s*['\"][^'\"]{8,}['\"]/gi, severity: 'MEDIUM' },
    { name: '.env in code',      regex: /process\.env\.([A-Z_]+)/g,              severity: 'INFO' },
  ];

  async scanFile(filePath: string, content: string): Promise<SecretFinding[]>;
  async scanProject(projectPath: string): Promise<SecretFinding[]>;
  async checkGitignore(projectPath: string): Promise<GitignoreCheck>;
}
```

### 6.4 Electron 보안 설정

```typescript
// src/main/index.ts — BrowserWindow 보안 설정
const mainWindow = new BrowserWindow({
  webPreferences: {
    nodeIntegration: false,          // Node.js API 접근 차단
    contextIsolation: true,          // Preload 컨텍스트 격리
    sandbox: true,                   // 샌드박스 모드
    webSecurity: true,               // 동일 출처 정책 활성
    allowRunningInsecureContent: false,
    // Preload 스크립트만 IPC 접근 가능
    preload: path.join(__dirname, '../preload/index.js'),
  },
});

// CSP (Content Security Policy) 설정
session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
  callback({
    responseHeaders: {
      ...details.responseHeaders,
      'Content-Security-Policy': [
        "default-src 'self'",
        "script-src 'self'",
        "style-src 'self' 'unsafe-inline'",  // Tailwind CSS 필요
        "connect-src 'self' https://api.anthropic.com https://api.openai.com https://generativelanguage.googleapis.com",
        "img-src 'self' data: blob:",
      ].join('; '),
    },
  });
});
```

### 6.5 보안 체크리스트

| 영역 | 체크 항목 | 상태 |
|------|----------|------|
| **Electron** | nodeIntegration: false | 필수 |
| **Electron** | contextIsolation: true | 필수 |
| **Electron** | sandbox: true | 필수 |
| **Electron** | CSP 헤더 설정 | 필수 |
| **Electron** | 원격 URL 로드 차단 (BrowserView 제외) | 필수 |
| **인증** | OAuth 2.0 + PKCE | 필수 |
| **인증** | state 파라미터 (CSRF 방지) | 필수 |
| **인증** | 토큰 keytar 저장 (OS 키체인) | 필수 |
| **인증** | 토큰 자동 갱신 | 필수 |
| **인증** | BrowserView URL 화이트리스트 | 필수 |
| **데이터** | 환경변수 값 keytar 저장 | 필수 |
| **데이터** | .env 파일 .gitignore 확인 | 필수 |
| **데이터** | 소스 코드 내 시크릿 탐지 | 필수 |
| **데이터** | SQLite 파일 접근 권한 제한 | 권장 |
| **통신** | 모든 외부 API 호출 HTTPS only | 필수 |
| **통신** | AI API 호출 시 민감 데이터 마스킹 | 권장 |
| **빌드** | 앱 서명 (macOS notarize, Windows signing) | 필수 |
| **빌드** | 자동 업데이트 서명 검증 | 필수 |
| **의존성** | npm audit 정기 실행 | 필수 |
| **의존성** | dependabot 또는 renovate 설정 | 권장 |

---

## 부록: 개발 환경 설정

### 빠른 시작

```bash
# 1. 의존성 설치
pnpm install

# 2. 개발 서버 실행 (Electron + Vite HMR)
pnpm dev

# 3. 빌드
pnpm build

# 4. 패키징 (현재 OS용)
pnpm package

# 5. 테스트
pnpm test
pnpm test:e2e
```

### 필수 전역 도구

```bash
# Node.js 20+
# pnpm 9+
# Semgrep (보안 스캔)
pip install semgrep

# macOS: Xcode Command Line Tools (네이티브 모듈 빌드)
xcode-select --install

# Linux: libsecret (keytar 의존성)
sudo apt install libsecret-1-dev
```

### 권장 VSCode 확장

- ESLint
- Prettier
- Tailwind CSS IntelliSense
- TypeScript Importer
