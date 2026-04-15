# VidEplace 개발 로드맵

> **현재 버전: v0.1.0-alpha (Phase 0 완료)**
> UI/디자인 프레임워크 구축 완료. Phase 1부터 실제 기능 구현 시작.

---

## 완료: Phase 0 — UI/디자인 프레임워크 (1일)

### 완료된 항목

| 카테고리 | 항목 | 파일 |
|----------|------|------|
| **페이지** | 로그인 | `pages/LoginPage.tsx` |
| **페이지** | 요금제 선택 | `pages/PricingPage.tsx` |
| **페이지** | 온보딩 (4단계 위자드) | `pages/OnboardingPage.tsx` |
| **페이지** | 대시보드 (내 서비스, 개발환경, 35개 서비스 연결) | `pages/DashboardPage.tsx` |
| **페이지** | IDE (워크플로우 캔버스 / 코드 에디터) | `pages/IDEPage.tsx` |
| **페이지** | 설정 (테마/언어/에디터) | `pages/SettingsPage.tsx` |
| **컴포넌트** | 커스텀 네비게이션바 (뒤로/홈/설정/유저메뉴/윈도우컨트롤) | `common/NavBar.tsx` |
| **컴포넌트** | 워크플로우 캔버스 (노드 시각화, 팬/줌, 노드 인포 패널) | `workflow/WorkflowCanvas.tsx` |
| **컴포넌트** | AI 채팅 위젯 (하단 중앙 오버레이) | `chat/ChatWidget.tsx` |
| **컴포넌트** | AI 채팅 패널 (사이드 패널) | `chat/ChatPanel.tsx` |
| **컴포넌트** | 코드 에디터 (Mock Monaco, 구문 강조, 탭, 웰컴) | `editor/EditorPanel.tsx` |
| **컴포넌트** | 파일 탐색기 (트리뷰) | `sidebar/FileExplorer.tsx` |
| **컴포넌트** | Git 소스 컨트롤 | `sidebar/GitPanel.tsx` |
| **컴포넌트** | 개발 환경 매니저 | `sidebar/DevEnvPanel.tsx` |
| **컴포넌트** | 계정 연결 관리 | `sidebar/AccountsPanel.tsx` |
| **컴포넌트** | 디버그 콘솔 (Console/Network/Problems/Terminal) | `debug/DebugPanel.tsx` |
| **컴포넌트** | 미리보기 패널 | `preview/PreviewPanel.tsx` |
| **컴포넌트** | 내장 인증 모달 | `auth/AuthModal.tsx` |
| **디자인** | 다크/라이트/모노카이 테마 전환 | CSS 변수 오버라이드 |
| **디자인** | CSS 클래스 시스템 (11개 CSS 모듈) | `styles/` |
| **인프라** | Electron + Vite + React + TypeScript + Tailwind | `main/main.ts` |
| **문서** | PRD, 개발 가이드, 디자인 가이드, README | `*.md` |

---

## Phase 1 — 핵심 엔진 (4주)

> **목표:** AI와 대화하면 진짜 코드가 생성되고, 파일이 만들어지고, 미리보기로 볼 수 있다.

### Week 1: 파일시스템 + 에디터

| 작업 | 상세 | 결과물 |
|------|------|--------|
| 로컬 폴더 열기/읽기 | Electron IPC로 `fs` 접근, 디렉토리 스캔 | `src/main/services/fileSystem.ts` |
| 파일 감시 (watch) | `chokidar`로 파일 변경 감지 → UI 자동 반영 | `src/main/services/fileWatcher.ts` |
| 파일 트리 실제 연동 | Mock 트리 → 실제 디렉토리 구조 | `FileExplorer.tsx` 리팩토링 |
| Monaco Editor 연동 | `@monaco-editor/react`로 진짜 에디터, 파일 열기/편집/저장 | `EditorPanel.tsx` 리팩토링 |
| 개발 환경 실제 감지 | `which node`, `python --version` 등 실행해서 실제 설치 상태 확인 | `DevEnvPanel.tsx` 리팩토링 |

### Week 2: AI 프로바이더 연동

| 작업 | 상세 | 결과물 |
|------|------|--------|
| AI 어댑터 인터페이스 | 공통 인터페이스 정의 (generateCode, chat, review) | `src/shared/ai/types.ts` |
| Claude API 연동 | Anthropic SDK, 스트리밍 응답, 토큰 카운팅 | `src/main/services/ai/claude.ts` |
| OpenAI API 연동 | OpenAI SDK, GPT-4o/o3 지원 | `src/main/services/ai/openai.ts` |
| API 키 저장 | `keytar`로 OS 키체인에 암호화 저장 | `src/main/services/keychain.ts` |
| AI 채팅 실제 통신 | 채팅 위젯/패널 → IPC → AI API → 스트리밍 응답 렌더링 | `ChatWidget.tsx`, `ChatPanel.tsx` |
| 토큰 카운터 | 실시간 토큰 사용량 + 예상 비용 표시 | UI 컴포넌트 |

### Week 3: 코드 생성 + 터미널

| 작업 | 상세 | 결과물 |
|------|------|--------|
| AI 코드 생성 파이프라인 | 채팅 입력 → AI 응답 파싱 → 파일 생성/수정 | `src/main/services/codegen.ts` |
| 파일 diff 표시 | AI가 수정한 부분을 diff로 표시, 승인/거절 | `components/editor/DiffView.tsx` |
| 내장 터미널 | `xterm.js` + `node-pty`, npm install/run 실행 | `components/terminal/Terminal.tsx` |
| 미리보기 실제 연동 | `BrowserView`로 localhost 프리뷰, 핫 리로드 | `PreviewPanel.tsx` 리팩토링 |

### Week 4: 워크플로우 연동 + 통합 테스트

| 작업 | 상세 | 결과물 |
|------|------|--------|
| 워크플로우 실시간 상태 | 노드 상태가 실제 파이프라인 진행에 따라 업데이트 | `WorkflowCanvas.tsx` |
| 서비스 생성 위자드 | "새 서비스" 버튼 → 프레임워크 선택 → AI PRD 생성 | `pages/NewServiceWizard.tsx` |
| E2E 테스트 | "쇼핑몰 만들어줘" → 코드 생성 → 파일 저장 → 터미널 실행 → 미리보기 | 수동 QA |

### Phase 1 완료 기준
- [ ] 채팅에서 "쇼핑몰 만들어줘"라고 하면 실제 코드 파일이 로컬에 생성됨
- [ ] 생성된 코드를 Monaco 에디터에서 편집 가능
- [ ] 터미널에서 `npm run dev` 실행 가능
- [ ] 미리보기에서 결과를 바로 확인 가능
- [ ] 워크플로우 캔버스의 노드 상태가 실시간 반영

---

## Phase 2 — Git + 보안 검증 + 출시 (4주)

> **목표:** 만든 코드를 검증하고, GitHub에 올리고, Vercel에 출시한다.

### Week 5: GitHub 연동

| 작업 | 상세 | 결과물 |
|------|------|--------|
| GitHub OAuth | Electron 내장 브라우저로 OAuth → 토큰 저장 | `src/main/services/github/auth.ts` |
| SSH 키 자동 설정 | ed25519 키 생성 → GitHub API로 등록 → ssh config 설정 | `src/main/services/github/ssh.ts` |
| Git 기본 기능 | `simple-git`으로 init/add/commit/push/pull/branch | `src/main/services/git.ts` |
| Git 패널 실제 연동 | Mock → 실제 status/diff/log | `GitPanel.tsx` 리팩토링 |

### Week 6: 보안 검증

| 작업 | 상세 | 결과물 |
|------|------|--------|
| ESLint 통합 | 프로젝트별 ESLint 실행, 결과 Problems 탭에 표시 | `src/main/services/lint.ts` |
| Semgrep 통합 | OWASP Top 10 룰셋, 보안 취약점 스캔 | `src/main/services/security.ts` |
| 시크릿 탐지 | 코드 내 API 키, 비밀번호 하드코딩 감지 | `src/main/services/secretDetector.ts` |
| AI 코드 리뷰 | 생성된 코드를 AI가 보안/품질 관점에서 분석 | `src/main/services/ai/review.ts` |
| 검증 점수 시스템 | 보안 점수 + 품질 점수, 리포트 생성 | `components/security/SecurityReport.tsx` |
| PR 생성/리뷰 | AI 자동 PR 본문 생성, 코드 리뷰 코멘트 | GitHub API 연동 |

### Week 7: 출시 연동

| 작업 | 상세 | 결과물 |
|------|------|--------|
| Vercel 출시 | Vercel API 연동, OAuth, 프로젝트 감지, 출시 실행 | `src/main/services/deploy/vercel.ts` |
| Railway 출시 | Railway API 연동 (풀스택 앱용) | `src/main/services/deploy/railway.ts` |
| 환경변수 관리 | 코드에서 필요한 env var 자동 감지, 설정 UI | `components/deploy/EnvVarManager.tsx` |
| 출시 빌드 로그 | 실시간 빌드 로그 스트리밍 | `components/deploy/BuildLog.tsx` |

### Week 8: 출시 파이프라인

| 작업 | 상세 | 결과물 |
|------|------|--------|
| 검증 게이트 | HIGH 보안 이슈 있으면 출시 차단 | 파이프라인 로직 |
| 멀티 환경 | Preview / Staging / Production 분리 | 출시 설정 UI |
| 롤백 | 이전 버전으로 원클릭 롤백 | `components/deploy/RollbackPanel.tsx` |
| 도메인/SSL | 커스텀 도메인 연결, SSL 자동 발급 | `components/deploy/DomainManager.tsx` |

### Phase 2 완료 기준
- [ ] AI가 만든 코드를 ESLint + Semgrep으로 자동 검증
- [ ] 보안 점수가 표시되고, HIGH 이슈 있으면 출시 차단
- [ ] GitHub에 커밋/푸시, PR 생성 가능
- [ ] Vercel에 원클릭 출시, 빌드 로그 실시간 확인
- [ ] 롤백 가능

---

## Phase 3 — 모니터링 + AIOps (4주)

> **목표:** 출시한 서비스를 실시간으로 모니터링하고, AI가 자동 대응한다.

### Week 9-10: 모니터링 기본

| 작업 | 상세 |
|------|------|
| 워치독 대시보드 | 업타임, 트래픽, 응답시간, 에러율 실시간 그래프 (Recharts) |
| 모니터링 에이전트 | 출시 앱에 경량 메트릭 수집 미들웨어 자동 삽입 |
| 에러 트래킹 | Sentry급 에러 수집, 그룹핑, 스택트레이스, 영향 사용자 수 |
| 실시간 로그 뷰어 | WebSocket으로 로그 스트리밍, 필터, 검색 |

### Week 11-12: AIOps + 알림

| 작업 | 상세 |
|------|------|
| 알림 시스템 | Slack/Discord/이메일/Telegram 웹훅, 커스텀 규칙 |
| 비용 트래커 | 서비스별 인프라 비용 추적, 예산 알림 |
| AI 에러 분석 | 에러 발생 → AI가 원인 분석 + 수정 코드 제안 |
| 트래픽 예측 | 과거 패턴 학습 → 선제적 스케일링 제안 |
| 스케일링 컨트롤 | 수동/자동 스케일링, 예산 제한 |
| 주간 AI 리포트 | 운영 요약 + 핵심 제안사항 자동 생성 |

### Phase 3 완료 기준
- [ ] 출시된 서비스의 실시간 상태를 워치독에서 확인
- [ ] 에러 발생 시 Slack/Discord로 알림
- [ ] AI가 에러 원인을 분석하고 수정 제안
- [ ] 비용 추적 및 스케일링 제어

---

## Phase 4 — B2B + 정식 출시 (4주)

> **목표:** 결제, 팀 협업, 멀티 플랫폼, 정식 베타 출시.

### Week 13-14: 인증 + 결제 + 팀

| 작업 | 상세 |
|------|------|
| 백엔드 서버 | Fastify + PostgreSQL + Redis |
| 실제 인증 | JWT, 이메일/비밀번호, OAuth (Google, GitHub) |
| Stripe 결제 | 구독 관리, 플랜 업/다운그레이드 |
| 팀 협업 | 팀 생성, 멤버 초대, 권한 관리, 서비스 공유 |

### Week 15-16: 앱 빌드 + 출시

| 작업 | 상세 |
|------|------|
| 추가 출시 플랫폼 | Netlify, Fly.io, Cloudflare Pages, AWS ECS |
| Electron 패키징 | electron-builder로 Mac/Win/Linux (.dmg, .exe, .AppImage) |
| 자동 업데이트 | electron-updater로 앱 자동 업데이트 |
| 엔터프라이즈 | SSO, 감사 로그, 온프레미스 옵션 |
| 베타 출시 | 랜딩 페이지, 대기 리스트, 베타 사용자 모집 |

### Phase 4 완료 기준
- [ ] Stripe으로 Pro/Team 결제 가능
- [ ] 팀 멤버 초대 및 협업 동작
- [ ] Mac/Win/Linux용 설치 파일 배포
- [ ] 자동 업데이트 동작
- [ ] 정식 베타 출시

---

## 전체 타임라인

```
Phase 0 (완료)  ████████████████████████████████  UI/디자인
Phase 1         ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  핵심 엔진 (4주)
Phase 2         ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  Git + 검증 + 출시 (4주)
Phase 3         ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  모니터링 + AIOps (4주)
Phase 4         ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  B2B + 정식 출시 (4주)
```

**총 예상 기간: 약 4개월 (16주)**
