# VidEplace 디자인 가이드

> **VidEplace** — Vibe + Dev + Place. PRD를 작성하면 AI가 코드를 생성하고, 자동 보안/품질 검증 후 원클릭 출시까지 해주는 올인원 Electron IDE.

---

## 1. 디자인 철학

### 핵심 원칙

1. **모던 IDE + 친근한 메신저**
   - VSCode의 생산성과 Discord/Slack의 접근성을 결합한다. 좌측 사이드바와 에디터 영역은 VSCode에서 영감을 받되, AI 채팅 패널은 메신저 앱처럼 자연스러운 대화형 UX를 제공한다.

2. **비개발자도 편한 UI**
   - 타겟 유저의 상당수가 바이브코더, 1인 창업자, 비개발 부서 인력이다. 복잡한 기능을 숨기되 필요할 때 바로 꺼낼 수 있는 **점진적 노출(Progressive Disclosure)** 패턴을 따른다. 초기 화면은 깔끔하게, 고급 기능은 컨텍스트 메뉴 또는 설정에서 노출한다.

3. **다크 모드 퍼스트**
   - 기본 테마는 다크 모드다. 장시간 코딩 환경에 최적화하고, 라이트 모드와 Monokai는 대안으로 제공한다. 모든 컴포넌트는 다크 모드를 기준으로 먼저 설계한 뒤 라이트/Monokai에 매핑한다.

4. **일관된 컴포넌트 시스템**
   - 모든 UI 요소는 디자인 토큰(색상, 타이포그래피, 간격, 그림자)으로 정의한다. 하나의 토큰을 바꾸면 전체 앱에 반영되는 구조를 유지한다.

5. **정보 밀도 조절**
   - IDE는 정보가 많다. 패널별로 명확한 시각적 경계를 두고, 색상 대비와 여백으로 가독성을 확보한다. 중요한 상태(에러, 성공, 경고)는 색상 + 아이콘으로 이중 전달한다.

6. **빠른 피드백**
   - 모든 사용자 인터랙션에 200ms 이내의 시각적 피드백을 제공한다. AI 응답 대기 시에는 타이핑 인디케이터와 스켈레톤으로 체감 대기 시간을 줄인다.

7. **넉넉한 여백, 큰 카드**
   - 모든 카드/박스는 내부 여백이 충분해야 한다 (최소 28px, 권장 36px). 글자가 벽에 붙어보이면 안 된다.
   - border-radius는 최소 16px, 카드는 24px을 기준으로 한다.
   - 카드 간 간격은 24~28px, 섹션 간 간격은 56px 이상.
   - 그림자는 부드럽고 넓게 (hover 시 `0 20px 40px rgba(0,0,0,0.25)`).
   - 반응형 단위(vw, vh, %) 우선. 고정 px은 최소한으로.

---

## 1.1 카드 & 박스 디자인 규칙

> **VidEplace 자체 디자인 시스템**
> 깔끔한 카드형, 넉넉한 여백, 둥근 모서리, 부드러운 그림자.

### 카드 사이즈 가이드

| 타입 | padding | border-radius | min-height | 용도 |
|------|---------|---------------|------------|------|
| **카드 (대)** | 36px | 24px | 300px+ | 서비스 카드, 메인 컨텐츠 카드 |
| **카드 (중)** | 28px 32px | 24px | - | 서비스 카드, 설정 섹션 카드 |
| **카드 (소)** | 20px 24px | 20px | - | 인라인 카드, 리스트 내 카드 |
| **리스트 행** | 24px 36px | - (부모 카드가 24px) | - | 최근 활동, 에러 목록 행 |

### 여백 규칙

```
■ 카드 내부:
  - 패딩: 최소 28px, 큰 카드는 36px
  - 아이콘↔제목: 28px
  - 제목↔설명: 12px
  - 요소 간: 16~20px
  - 카드 내 구분선 위아래: 24px

■ 카드 외부:
  - 카드 간 간격 (gap): 24~28px
  - 섹션 간 간격: 56px
  - 섹션 제목↔카드: 24px

■ 페이지:
  - 상하 여백: 7vh (뷰포트 높이 비례)
  - 좌우 여백: 7.5% (컨테이너 w-[85%])
  - max-width: 1300px
```

### border-radius 체계

| 요소 | radius | CSS 변수/값 |
|------|--------|-----------|
| 카드 | 24px | `border-radius: 24px` |
| 아이콘 박스 | 18~20px | `border-radius: 20px` |
| 버튼 (큰) | 16px | `border-radius: 16px` |
| 뱃지/태그 | 14px | `border-radius: 14px` |
| 입력 필드 | 12px | `border-radius: 12px` |
| 아이콘 버튼 | 12px | `border-radius: 12px` |

### 그림자 체계

| 상태 | box-shadow | 용도 |
|------|-----------|------|
| 기본 | 없음 (border만) | 대부분의 카드 |
| Hover | `0 12px 32px rgba(0,0,0,0.15)` | 서비스 카드 등 가벼운 hover |
| Hover (강조) | `0 20px 40px rgba(0,0,0,0.25)` | 서비스 카드 등 인터랙티브 |
| 버튼 | `0 4px 16px rgba(88,166,255,0.25)` | Primary 버튼 |
| 버튼 Hover | `0 8px 24px rgba(88,166,255,0.35)` | Primary 버튼 hover |
| 모달 | `0 24px 48px rgba(0,0,0,0.4)` | 모달/팝오버 |

### 아이콘 박스

| 크기 | width × height | radius | 아이콘 size | 용도 |
|------|---------------|--------|-----------|------|
| Medium | 56 × 56px | 18px | 22~24px | 서비스 카드, 설정 항목 |
| Large | 64 × 64px | 20px | 28~32px | 서비스 카드 메인 아이콘 |

아이콘 박스 배경색:
- 연결됨/성공: `bg-accent-success/10`
- 기본/액센트: `bg-accent-primary/10`
- 미연결/비활성: `bg-bg-elevated`

### 트랜지션

| 속성 | duration | easing | 대상 |
|------|----------|--------|------|
| 배경/색상 변경 | 250ms | ease | 카드 hover, 버튼 hover |
| transform | 250ms | ease | 카드 hover (translateY) |
| box-shadow | 250ms | ease | 카드/버튼 hover |
| scale | 200ms | ease | 버튼 클릭 |
| opacity | 200ms | ease | 화살표, 토스트 |

---

## 1.2 CSS 클래스 시스템

> 모든 공통 스타일은 `src/renderer/styles/components.css`에 정의.
> 페이지별 스타일은 `src/renderer/styles/pages/*.css`에 정의.

### 파일 구조

```
src/renderer/styles/
├── globals.css           # Tailwind import + 디자인 토큰 + 글로벌 스타일
├── components.css        # 공통 컴포넌트 클래스 (카드, 버튼, 뱃지 등)
├── navbar.css            # 커스텀 네비게이션바 스타일
└── pages/
    ├── login.css         # 로그인 페이지 전용 스타일
    ├── pricing.css       # 요금제 선택 페이지 전용 스타일
    ├── onboarding.css    # 온보딩 페이지 전용 스타일
    ├── dashboard.css     # 대시보드 페이지 전용 스타일
    ├── ide.css           # IDE 페이지 전용 스타일
    ├── settings.css      # 설정 페이지 전용 스타일
    └── workflow.css      # 워크플로우 캔버스 전용 스타일
```

globals.css에서 모든 CSS를 import하는 계층 구조:
```css
@import "tailwindcss";
@import "./components.css";
@import "./navbar.css";
@import "./pages/login.css";
@import "./pages/pricing.css";
@import "./pages/onboarding.css";
@import "./pages/dashboard.css";
@import "./pages/ide.css";
@import "./pages/settings.css";
@import "./pages/workflow.css";
```

### 공통 클래스 목록

#### 레이아웃
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.page-container` | 페이지 최외곽 | `flex h-screen flex-col` |
| `.page-content` | 스크롤 영역 | `flex-1 overflow-y-auto` |
| `.page-content-inner` | 콘텐츠 중앙 정렬 | `w-[85%] max-w-[1300px] py-[7vh]` |

#### 카드
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.card` | 기본 카드 | `radius: 24px, border, bg-secondary` |
| `.card-interactive` | 클릭 가능 카드 | `card + hover 효과 + translateY(-4px)` |
| `.card-md` | 중간 카드 | `padding: 28px 32px, radius: 24px` |

#### 버튼
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.btn-primary` | 메인 액션 버튼 | `accent-primary, radius: 16px, shadow` |
| `.btn-ghost` | 보조 버튼 | `bg-elevated, hover→accent` |
| `.btn-icon-md` | 아이콘 버튼 | `36×36px, radius: 12px` |

#### 텍스트
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.heading-1` | 페이지 제목 | `32px, bold, tracking-tight` |
| `.heading-2` | 섹션 제목 | `22px, bold` |
| `.heading-3` | 카드 제목 | `26px, bold` |
| `.subtitle` | 서브타이틀 | `16px, text-secondary` |
| `.text-body` | 본문 | `16px, semibold, text-primary` |
| `.text-caption` | 캡션 | `14px, medium, text-tertiary` |

#### 기타
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.badge` | 뱃지/태그 | `radius: 14px, padding: 8px 16px` |
| `.badge-success` | 성공 뱃지 | `bg-success/10, text-success` |
| `.icon-box-md` | 아이콘 박스 중 | `56×56px, radius: 18px` |
| `.icon-box-lg` | 아이콘 박스 대 | `64×64px, radius: 20px` |
| `.list-card` | 리스트 카드 | `radius: 24px, overflow-hidden` |
| `.list-card-row` | 리스트 행 | `padding: 24px 36px` |
| `.section` | 섹션 간격 | `margin-bottom: 56px` |
| `.divider` | 구분선 | `border-t border-border-primary` |
| `.avatar-md` | 아바타 | `36×36px, rounded-full` |

#### 네비게이션바 (navbar.css)
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.navbar` | 커스텀 네비바 | `height: 44px, bg-secondary, border-b, -webkit-app-region: drag` |
| `.navbar-left` | 좌측 영역 (네비버튼) | `flex, gap-1, no-drag` |
| `.navbar-center` | 중앙 드래그 영역 | `flex-1` |
| `.navbar-right` | 우측 영역 (액션+윈도우컨트롤) | `flex, gap-1, no-drag` |
| `.navbar-btn` | 네비 버튼 | `32×32px, radius: 10px` |
| `.navbar-btn-active` | 활성 네비 버튼 | `text-accent-primary` |
| `.navbar-separator` | 구분선 | `1px × 20px` |
| `.navbar-breadcrumb` | 브레드크럼 | `flex, gap-0.5` |
| `.navbar-breadcrumb-current` | 현재 위치 | `text-primary, font-semibold` |
| `.navbar-window-btn` | 윈도우 컨트롤 버튼 | `40×32px` |
| `.navbar-window-btn-close` | 닫기 버튼 hover | `bg: accent-error, color: white` |

#### 로그인 페이지 (login.css)
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.login-wrapper` | 전체 배경 | `flex, center, h-screen` |
| `.login-container` | 중앙 정렬 컨테이너 | `flex-col, center` |
| `.login-card` | 로그인 카드 | `radius: 24px, padding: 36px+` |
| `.login-input` | 입력 필드 | `radius: 12px, bg-tertiary` |
| `.login-btn-primary` | 로그인 버튼 | `accent-primary, radius: 16px` |
| `.login-btn-social` | 소셜 로그인 버튼 | `bg-elevated, radius: 14px` |

#### 요금제 페이지 (pricing.css)
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.pricing-wrapper` | 전체 배경 | `flex, center, h-screen` |
| `.pricing-grid` | 요금제 카드 그리드 | `grid, 4컬럼` |
| `.pricing-card` | 요금제 카드 | `radius: 24px, flex-col` |
| `.pricing-card-recommended` | 추천 카드 강조 | `border-accent-primary` |
| `.pricing-btn-primary` | 추천 플랜 버튼 | `accent-primary` |
| `.pricing-btn-ghost` | 일반 플랜 버튼 | `bg-elevated` |

#### 워크플로우 캔버스 (workflow.css)
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.workflow-canvas` | 캔버스 전체 | `relative, overflow-hidden` |
| `.workflow-grid-bg` | 그리드 배경 | `absolute, dot-pattern` |
| `.workflow-node` | 워크플로우 노드 | `absolute, radius: 16px, 180×72px` |
| `.workflow-node-active` | 진행 중 노드 | `border-accent-primary, glow` |
| `.workflow-node-completed` | 완료 노드 | `border-accent-success` |
| `.workflow-node-selected` | 선택된 노드 | `ring-accent-primary` |
| `.workflow-zoom-controls` | 줌 컨트롤 | `absolute, bottom-left` |
| `.workflow-float` | 플로팅 애니메이션 | `@keyframes float` |

#### 채팅 위젯 (workflow.css / chat-widget)
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.chat-widget` | 채팅 위젯 컨테이너 | `absolute, bottom-center, radius: 24px` |
| `.chat-widget-trigger` | 채팅 열기 버튼 | `pill-shape, accent-primary` |
| `.chat-widget-header` | 위젯 헤더 | `flex, justify-between` |
| `.chat-widget-messages` | 메시지 영역 | `overflow-y-auto` |
| `.chat-widget-bubble-user` | 사용자 메시지 | `bg-accent-primary/20, radius: 16px` |
| `.chat-widget-bubble-ai` | AI 메시지 | `bg-bg-secondary, radius: 16px` |
| `.chat-widget-input-area` | 입력 영역 | `border-t, padding` |

#### 노드 상태 패널 (workflow.css / node-info)
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.node-info-panel` | 정보 패널 | `absolute, right, radius: 20px, shadow` |
| `.node-info-header` | 패널 헤더 | `flex, justify-between` |
| `.node-info-summary` | AI 요약 | `bg-accent-primary/5, radius: 12px` |
| `.node-info-details` | 상세 정보 | `flex-col, gap` |
| `.node-info-ai-note` | AI 분석 노트 | `bg-accent-purple/5, radius: 12px` |

#### 설정 페이지 (settings.css)
| 클래스 | 용도 | 핵심 속성 |
|--------|------|----------|
| `.settings-layout` | 설정 전체 레이아웃 | `flex, h-full` |
| `.settings-sidebar` | 좌측 사이드바 | `width: ~200px, flex-col` |
| `.settings-sidebar-item` | 사이드바 항목 | `padding, radius: 10px` |
| `.settings-sidebar-item-active` | 활성 항목 | `bg-bg-hover, text-primary` |
| `.settings-content` | 설정 본문 | `flex-1, overflow-y-auto` |
| `.settings-card` | 설정 카드 | `radius: 16px, padding: 24px` |
| `.settings-toggle` | 토글 스위치 | `width: 44px, height: 24px` |
| `.settings-theme-grid` | 테마 선택 그리드 | `grid, gap` |
| `.settings-theme-card` | 테마 카드 | `radius: 12px, cursor-pointer` |

### 사용 예시

```tsx
// TSX에서 CSS 클래스 사용
<div className="page-container">
  <div className="titlebar">...</div>
  <div className="page-content">
    <div className="page-content-inner">
      <div className="section">
        <h2 className="section-header heading-2">섹션 제목</h2>
        <div className="project-grid">
          <button className="project-card">
            <div className="project-card-icon">...</div>
            <h3 className="project-card-title">서비스명</h3>
            <span className="badge">Next.js</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</div>
```

### 새 컴포넌트 스타일 추가 규칙

1. **공통 컴포넌트** → `components.css`에 추가
2. **특정 페이지 전용** → `pages/해당페이지.css`에 추가
3. **인라인 Tailwind는 일회성 스타일에만** 사용 (gap 미세조정, 색상 오버라이드 등)
4. **px 값보다 디자인 토큰 우선** — 하드코딩 줄이기
5. **호버/트랜지션은 반드시 CSS 클래스로** — TSX에서 hover: 남발 금지

---

## 2. 컬러 시스템

### 2.1 Dark 테마 (기본)

VidEplace의 기본 팔레트. GitHub Dark Dimmed에서 영감을 받되, 채도를 약간 높여 생동감을 더한다.

#### Background (배경)

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--bg-canvas` | `#0d1117` | 최하위 배경 (윈도우 타이틀바, 상태바) |
| `--bg-sidebar` | `#161b22` | 사이드바, 액티비티바 |
| `--bg-editor` | `#1c2128` | 에디터 영역, 채팅 패널 배경 |
| `--bg-surface` | `#21262d` | 카드, 드롭다운, 팝오버 배경 |
| `--bg-elevated` | `#2d333b` | 모달, 토스트, 부유 패널 |
| `--bg-overlay` | `rgba(1, 4, 9, 0.8)` | 모달 오버레이 |
| `--bg-input` | `#0d1117` | 입력 필드 배경 |
| `--bg-hover` | `rgba(177, 186, 196, 0.12)` | 항목 호버 |
| `--bg-active` | `rgba(177, 186, 196, 0.2)` | 항목 활성(선택됨) |

#### Text (텍스트)

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--text-primary` | `#e6edf3` | 기본 텍스트 |
| `--text-secondary` | `#8b949e` | 보조 텍스트, 설명, 타임스탬프 |
| `--text-tertiary` | `#6e7681` | 힌트, 플레이스홀더 |
| `--text-disabled` | `#484f58` | 비활성 텍스트 |
| `--text-link` | `#58a6ff` | 링크 텍스트 |
| `--text-on-accent` | `#ffffff` | 액센트 배경 위 텍스트 |

#### Accent (강조색)

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--accent-primary` | `#58a6ff` | 메인 액센트 (버튼, 링크, 포커스 링) |
| `--accent-primary-hover` | `#79c0ff` | 메인 액센트 호버 |
| `--accent-primary-muted` | `rgba(56, 139, 253, 0.4)` | 배지 배경, 선택 하이라이트 |
| `--accent-success` | `#3fb950` | 성공, 연결됨, 출시 완료 |
| `--accent-success-muted` | `rgba(63, 185, 80, 0.15)` | 성공 배경 |
| `--accent-danger` | `#f85149` | 에러, 삭제, 위험 |
| `--accent-danger-muted` | `rgba(248, 81, 73, 0.15)` | 에러 배경 |
| `--accent-warning` | `#d29922` | 경고, 주의 |
| `--accent-warning-muted` | `rgba(210, 153, 34, 0.15)` | 경고 배경 |
| `--accent-info` | `#58a6ff` | 정보성 알림 |
| `--accent-purple` | `#bc8cff` | Anthropic/Claude 브랜드 |
| `--accent-pink` | `#f778ba` | 특수 강조 (배지, 태그) |

#### Border (테두리)

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--border-default` | `#30363d` | 기본 테두리 |
| `--border-muted` | `#21262d` | 약한 테두리 (구분선) |
| `--border-strong` | `#484f58` | 강한 테두리 (입력 필드 포커스 전) |
| `--border-accent` | `#58a6ff` | 포커스 링, 활성 테두리 |

#### Syntax Highlighting (코드 구문 강조 - Dark)

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--syntax-keyword` | `#ff7b72` | `if`, `const`, `return` 등 |
| `--syntax-string` | `#a5d6ff` | 문자열 리터럴 |
| `--syntax-function` | `#d2a8ff` | 함수명 |
| `--syntax-variable` | `#ffa657` | 변수명 |
| `--syntax-comment` | `#8b949e` | 주석 |
| `--syntax-number` | `#79c0ff` | 숫자 리터럴 |
| `--syntax-type` | `#ff7b72` | 타입, 클래스명 |
| `--syntax-operator` | `#79c0ff` | 연산자 |
| `--syntax-tag` | `#7ee787` | HTML/JSX 태그 |
| `--syntax-attribute` | `#79c0ff` | HTML 속성 |

---

### 2.2 Light 테마

#### Background

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--bg-canvas` | `#ffffff` | 최하위 배경 |
| `--bg-sidebar` | `#f6f8fa` | 사이드바 |
| `--bg-editor` | `#ffffff` | 에디터 영역 |
| `--bg-surface` | `#f6f8fa` | 카드, 드롭다운 |
| `--bg-elevated` | `#ffffff` | 모달, 토스트 |
| `--bg-overlay` | `rgba(27, 31, 36, 0.5)` | 모달 오버레이 |
| `--bg-input` | `#ffffff` | 입력 필드 |
| `--bg-hover` | `rgba(208, 215, 222, 0.32)` | 호버 |
| `--bg-active` | `rgba(208, 215, 222, 0.48)` | 활성 |

#### Text

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--text-primary` | `#1f2328` | 기본 텍스트 |
| `--text-secondary` | `#656d76` | 보조 텍스트 |
| `--text-tertiary` | `#8b949e` | 힌트 |
| `--text-disabled` | `#b1bac4` | 비활성 |
| `--text-link` | `#0969da` | 링크 |

#### Accent

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--accent-primary` | `#0969da` | 메인 액센트 |
| `--accent-primary-hover` | `#0550ae` | 호버 |
| `--accent-success` | `#1a7f37` | 성공 |
| `--accent-danger` | `#cf222e` | 에러 |
| `--accent-warning` | `#9a6700` | 경고 |

#### Border

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--border-default` | `#d0d7de` | 기본 테두리 |
| `--border-muted` | `#e6edf3` | 약한 테두리 |
| `--border-strong` | `#8b949e` | 강한 테두리 |
| `--border-accent` | `#0969da` | 포커스 링 |

#### Syntax Highlighting (Light)

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--syntax-keyword` | `#cf222e` | 키워드 |
| `--syntax-string` | `#0a3069` | 문자열 |
| `--syntax-function` | `#8250df` | 함수 |
| `--syntax-variable` | `#953800` | 변수 |
| `--syntax-comment` | `#6e7781` | 주석 |
| `--syntax-number` | `#0550ae` | 숫자 |
| `--syntax-tag` | `#116329` | 태그 |

---

### 2.3 Monokai 테마

#### Background

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--bg-canvas` | `#1e1f1c` | 최하위 배경 |
| `--bg-sidebar` | `#252620` | 사이드바 |
| `--bg-editor` | `#272822` | 에디터 영역 |
| `--bg-surface` | `#2d2e27` | 카드 |
| `--bg-elevated` | `#3e3d32` | 모달 |
| `--bg-overlay` | `rgba(0, 0, 0, 0.7)` | 오버레이 |
| `--bg-input` | `#1e1f1c` | 입력 필드 |
| `--bg-hover` | `rgba(255, 255, 255, 0.08)` | 호버 |
| `--bg-active` | `rgba(255, 255, 255, 0.15)` | 활성 |

#### Text

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--text-primary` | `#f8f8f2` | 기본 텍스트 |
| `--text-secondary` | `#a6a18a` | 보조 텍스트 |
| `--text-tertiary` | `#75715e` | 힌트 |
| `--text-disabled` | `#5b5a4f` | 비활성 |
| `--text-link` | `#66d9ef` | 링크 |

#### Accent

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--accent-primary` | `#66d9ef` | 메인 액센트 (시안) |
| `--accent-primary-hover` | `#89e3f5` | 호버 |
| `--accent-success` | `#a6e22e` | 성공 (라임 그린) |
| `--accent-danger` | `#f92672` | 에러 (핑크) |
| `--accent-warning` | `#e6db74` | 경고 (옐로) |
| `--accent-purple` | `#ae81ff` | 보라 |
| `--accent-orange` | `#fd971f` | 오렌지 |

#### Border

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--border-default` | `#3e3d32` | 기본 테두리 |
| `--border-muted` | `#2d2e27` | 약한 테두리 |
| `--border-accent` | `#66d9ef` | 포커스 링 |

#### Syntax Highlighting (Monokai)

| 토큰 | HEX | 용도 |
|------|-----|------|
| `--syntax-keyword` | `#f92672` | 키워드 (핑크) |
| `--syntax-string` | `#e6db74` | 문자열 (옐로) |
| `--syntax-function` | `#a6e22e` | 함수 (라임) |
| `--syntax-variable` | `#fd971f` | 변수 (오렌지) |
| `--syntax-comment` | `#75715e` | 주석 |
| `--syntax-number` | `#ae81ff` | 숫자 (퍼플) |
| `--syntax-type` | `#66d9ef` | 타입 (시안) |
| `--syntax-tag` | `#f92672` | 태그 |
| `--syntax-attribute` | `#a6e22e` | 속성 |

---

## 3. 타이포그래피

### 3.1 폰트 패밀리

| 용도 | 폰트 | 폴백 |
|------|------|------|
| **UI (영문)** | `Inter` | `-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif` |
| **UI (한글)** | `Pretendard` | `'Noto Sans KR', sans-serif` |
| **코드** | `JetBrains Mono` | `'Fira Code', 'Cascadia Code', 'Consolas', monospace` |
| **터미널** | `JetBrains Mono` | `'Cascadia Mono', 'Menlo', monospace` |

CSS 폰트 스택:
```css
--font-ui: 'Inter', 'Pretendard', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans KR', sans-serif;
--font-code: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace;
--font-terminal: 'JetBrains Mono', 'Cascadia Mono', 'Menlo', monospace;
```

### 3.2 크기 체계 (Type Scale)

> **규칙**: 대시보드 등 넓은 화면에서는 큰 텍스트, IDE 에디터 내부에서는 작은 텍스트 사용.
> 글자가 카드에 비해 작아보이면 안 된다.

| 토큰 | 크기 | 줄 간격 | CSS 클래스 | 용도 |
|------|------|---------|-----------|------|
| `--text-xs` | 11px | 16px (1.45) | - | 상태바, 토큰 카운터 |
| `--text-sm` | 12px | 18px (1.5) | - | 사이드바 항목, 파일 트리, 탭 라벨 |
| `--text-base` | 14px | 21px (1.5) | `.text-caption` | 캡션, 부가 정보 |
| `--text-md` | 16px | 24px (1.5) | `.text-body`, `.subtitle` | **기본 본문**, 서브타이틀 |
| `--text-lg` | 18px | 27px (1.5) | - | AI 채팅 메시지, 설정 라벨 |
| `--text-xl` | 22px | 31px (1.4) | `.heading-2` | 섹션 제목 |
| `--text-2xl` | 26px | 34px (1.3) | `.heading-3` | 카드 내 제목 |
| `--text-3xl` | 32px | 42px (1.3) | `.heading-1` | 페이지 제목, 히어로 |

### 3.3 폰트 웨이트

| 토큰 | 값 | 용도 |
|------|-----|------|
| `--font-regular` | 400 | 기본 본문 텍스트 |
| `--font-medium` | 500 | 사이드바 활성 항목, 탭 라벨, 뱃지 |
| `--font-semibold` | 600 | 버튼 텍스트, 섹션 헤더, 패널 타이틀 |
| `--font-bold` | 700 | 대시보드 메트릭 값, 헤드라인 |

### 3.4 코드 폰트 설정

```css
--code-font-size: 13px;
--code-line-height: 20px;    /* 1.54 */
--code-letter-spacing: 0px;
--code-tab-size: 2;

/* 리거처(합자) 지원 */
font-feature-settings: 'liga' 1, 'calt' 1;
```

### 3.5 줄 간격 가이드

- **제목/헤드라인**: line-height 1.3 (시각적 밀도, `.heading-1~3`)
- **본문 텍스트**: line-height 1.5 (가독성 우선, `.text-body`)
- **서브타이틀**: line-height 1.6 (여유로운 읽기, `.subtitle`)
- **코드 에디터**: line-height 1.5 ~ 1.6 (라인 간 명확한 구분)
- **채팅 메시지**: line-height 1.5 (대화 가독성)
- **캡션**: line-height 1.5 (`.text-caption`)

---

## 4. 레이아웃 시스템

### 4.1 전체 레이아웃 구조

```
┌──────────────────────────────────────────────────────────────────┐
│  네비게이션바 (44px) - 뒤로/홈/브레드크럼 + 뷰토글/설정/유저 + 윈도우컨트롤 │
├────┬─────────┬──────────────────────────┬────────────────────────┤
│ A  │    B    │           C              │          D             │
│ c  │ 사이드바 │      메인 에디터 영역       │      AI 채팅 패널       │
│ t  │(240px)  │                          │      (360px)           │
│ i  │         │                          │                        │
│ v  │ 파일    ├────────────┬─────────────┤  모델 선택 드롭다운       │
│ i  │ 탐색기  │   에디터    │  미리보기    │                        │
│ t  │         │   (탭바 +  │  (BrowserV) │  메시지 히스토리          │
│ y  │─────────│   Monaco)  │             │  (스크롤)               │
│    │ 서비스   │            │             │                        │
│ B  │ 패널    ├────────────┴─────────────┤  코드블록 + diff         │
│ a  │ (AI,   │    하단 패널 (디버그)       │  승인/거절 버튼          │
│ r  │  Git,  │    (200px)                │                        │
│    │  배포)  │ Console|Network|Problems  │  입력 영역              │
│48px│         │         |Terminal         │  토큰 카운터             │
├────┴─────────┴──────────────────────────┴────────────────────────┤
│  상태바 (22px) - 브랜치, 에러카운트, AI상태, 언어, 인코딩              │
└──────────────────────────────────────────────────────────────────┘
```

### 4.2 패널 크기 명세

| 패널 | 기본 크기 | 최소 크기 | 최대 크기 | 방향 |
|------|----------|----------|----------|------|
| **액티비티바** | 48px | 48px (고정) | 48px (고정) | 수직 |
| **사이드바** | 240px | 170px | 480px | 수직 |
| **AI 채팅 패널** | 360px | 280px | 600px | 수직 |
| **하단 패널 (디버그)** | 200px | 100px | 전체 높이의 60% | 수평 |
| **미리보기 패널** | 에디터의 50% | 300px | 에디터의 80% | 수직 (에디터 내 분할) |
| **타이틀바** | 30px | 30px (고정) | 30px (고정) | 수평 |
| **상태바** | 22px | 22px (고정) | 22px (고정) | 수평 |
| **탭바** | 35px | 35px (고정) | 35px (고정) | 수평 |

### 4.3 리사이즈 핸들

```css
/* 리사이즈 핸들 스타일 */
.resize-handle {
  width: 4px;              /* 수직 분할선 */
  height: 4px;             /* 수평 분할선 */
  background: transparent;
  cursor: col-resize;      /* 수직: col-resize, 수평: row-resize */
  transition: background 150ms ease;
}

.resize-handle:hover {
  background: var(--accent-primary);  /* #58a6ff */
}

.resize-handle:active {
  background: var(--accent-primary);
  width: 2px;  /* 드래그 중 얇아짐 */
}
```

- 핸들 히트 영역: 실제 시각 폭 4px, 클릭 가능 영역 8px (터치 친화)
- 호버 시 파란색 하이라이트로 드래그 가능함을 명시
- 더블클릭 시 패널을 기본 크기로 리셋

### 4.4 패널 접기/펼치기

| 동작 | 방법 | 애니메이션 |
|------|------|-----------|
| 사이드바 접기 | 액티비티바 아이콘 클릭 또는 `Ctrl+B` | 200ms ease-out, 너비 240px -> 0px |
| AI 채팅 접기 | 패널 헤더 토글 또는 `Ctrl+Shift+I` | 200ms ease-out, 너비 360px -> 0px |
| 하단 패널 접기 | 패널 헤더 토글 또는 `Ctrl+`` ` | 200ms ease-out, 높이 200px -> 0px |
| 미리보기 접기 | 에디터 분할 토글 | 200ms ease-out |

접힌 패널은 완전히 사라지며, 메인 에디터 영역이 나머지 공간을 채운다.

### 4.5 반응형 브레이크포인트

Electron 윈도우 크기에 따른 레이아웃 적응:

| 브레이크포인트 | 윈도우 너비 | 레이아웃 변화 |
|--------------|-----------|-------------|
| `xl` | >= 1600px | 4패널 모두 표시 (기본) |
| `lg` | 1200px ~ 1599px | 미리보기 패널 탭으로 전환 (에디터/미리보기 전환) |
| `md` | 900px ~ 1199px | AI 채팅 오버레이 모드 (에디터 위에 슬라이드) |
| `sm` | < 900px | 사이드바 오버레이 + AI 채팅 오버레이, 싱글 패널 모드 |

### 4.6 간격(Spacing) 시스템

8px 그리드 기반:

| 토큰 | 값 | 용도 |
|------|-----|------|
| `--space-0` | 0px | 없음 |
| `--space-1` | 2px | 인라인 아이콘 간격 |
| `--space-2` | 4px | 아이콘-텍스트 간격, 태그 내부 패딩 |
| `--space-3` | 8px | 컴포넌트 내부 패딩 (작은), 리스트 항목 간 |
| `--space-4` | 12px | 입력 필드 패딩, 버튼 수평 패딩 (sm) |
| `--space-5` | 16px | 카드 내부 패딩, 섹션 간 간격 |
| `--space-6` | 20px | 패널 내부 패딩 |
| `--space-7` | 24px | 모달 내부 패딩 |
| `--space-8` | 32px | 섹션 간 큰 간격 |
| `--space-9` | 40px | 페이지 레벨 패딩 |
| `--space-10` | 48px | 온보딩 화면 큰 간격 |
| `--space-12` | 64px | 대시보드 섹션 간격 |

### 4.7 각 화면별 레이아웃 상세

#### 4.7.0 로그인 화면

```
┌──────────────────────────────────────────────┐
│                                              │
│              VidEplace 로고 (중앙)             │
│              "AI 기반 올인원 IDE"              │
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │              로그인 카드                 │  │
│  │  이메일 입력                            │  │
│  │  비밀번호 입력                          │  │
│  │  [로그인]                              │  │
│  │  ── 또는 ──                            │  │
│  │  [Google로 계속하기]                    │  │
│  │  [GitHub로 계속하기]                    │  │
│  │  회원가입 링크                          │  │
│  └────────────────────────────────────────┘  │
│                                              │
└──────────────────────────────────────────────┘
```

- 전체 화면, 중앙 정렬 레이아웃
- 카드 최대 너비: 420px
- NavBar 표시하지 않음
- 로그인 성공 후 → 요금제 미선택 시 Pricing, 선택 완료 시 Dashboard로 이동

#### 4.7.0-1 요금제 선택 화면

```
┌──────────────────────────────────────────────┐
│              요금제를 선택하세요               │
│                                              │
│  ┌────┐  ┌────┐  ┌────┐  ┌────┐            │
│  │무료 │  │Pro │  │Team│  │기업│            │
│  │$0  │  │$12 │  │$29 │  │커스│            │
│  │    │  │추천 │  │    │  │텀  │            │
│  └────┘  └────┘  └────┘  └────┘            │
│                                              │
└──────────────────────────────────────────────┘
```

- 전체 화면, 중앙 정렬 레이아웃
- 4열 그리드 카드
- 추천 카드에 border-accent-primary 강조 + "추천" 뱃지
- NavBar 표시하지 않음
- 요금제 선택 후 → Onboarding으로 이동

#### 4.7.1 온보딩 화면

```
┌──────────────────────────────────────────────┐
│              VidEplace 로고 (중앙)             │
│                                              │
│         스텝 인디케이터 (1/4, 2/4...)          │
│         ● ─── ○ ─── ○ ─── ○                 │
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │                                        │  │
│  │         스텝별 콘텐츠 영역               │  │
│  │         (최대 너비 520px, 중앙 정렬)     │  │
│  │                                        │  │
│  └────────────────────────────────────────┘  │
│                                              │
│         [이전]                    [다음]       │
│                                              │
└──────────────────────────────────────────────┘
```

- 전체 화면, 중앙 정렬 레이아웃
- 콘텐츠 최대 너비: 520px
- 스텝: (1) 웰컴 (2) AI 프로바이더 연결 (3) GitHub 연결 (4) 테마 선택
- 배경: `--bg-canvas` 위에 미세한 그라디언트 또는 패턴

#### 4.7.2 서비스 대시보드

```
┌──────────────────────────────────────────────────────┐
│  상단 네비게이션 바 (56px)                              │
│  로고  |  내 서비스  |  설정  |  계정        아바타   │
├──────────────────────────────────────────────────────┤
│                                                      │
│  내 서비스                            [+ 새 서비스]     │
│                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │ 서비스    │  │ 서비스    │  │ 서비스    │           │
│  │ 카드 1    │  │ 카드 2    │  │ 카드 3    │           │
│  │ (280px)  │  │ (280px)  │  │ (280px)  │           │
│  └──────────┘  └──────────┘  └──────────┘           │
│                                                      │
│  최근 활동                                            │
│  ─────────────────────────────────────────           │
│  활동 항목 리스트                                      │
│                                                      │
└──────────────────────────────────────────────────────┘
```

- 서비스 카드 그리드: `grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))`
- 카드 간 간격: 16px
- 최대 콘텐츠 너비: 1200px, 중앙 정렬
- 좌우 패딩: 32px

#### 4.7.3 IDE 메인 (두 가지 모드)

IDE 페이지는 `workflowView` 상태에 따라 두 가지 모드를 지원한다. NavBar의 토글 버튼으로 전환 가능.

**모드 1: 워크플로우 캔버스 (기본)**
- 전체 화면 캔버스에 DAG 형태의 워크플로우 노드 표시
- 팬/줌 지원 (마우스 휠 줌, 미들 버튼 팬)
- 노드 클릭 시 우측에 NodeInfoPanel 오버레이 표시
- 하단 중앙에 ChatWidget 오버레이 (접기/펼치기)
- 노드: PRD 입력 → AI 코드 생성 → 보안 검증 → 출시 → 미리보기

**모드 2: 코드 에디터**
- 기존 4패널 구조: 액티비티바(48px) + 사이드바(240px) + 에디터(유동) + AI채팅(360px)
- 에디터 영역 내에서 에디터:미리보기 = 50:50 (수평 분할)
- 에디터 영역 하단 디버그패널

#### 4.7.4 설정 화면

```
┌──────────────────────────────────────────────────────┐
│  설정                                         [닫기]  │
├─────────────┬────────────────────────────────────────┤
│ 사이드 내비   │  설정 콘텐츠 영역                        │
│ (200px)     │  (최대 너비 680px)                       │
│             │                                        │
│ 일반         │  일반 설정                               │
│ 에디터       │  ────────────────────                   │
│ AI 프로바이더 │  테마: [Dark ▼]                         │
│ GitHub      │  언어: [한국어 ▼]                        │
│ 배포         │  ...                                   │
│ 보안         │                                        │
│ 키바인딩     │                                        │
│ 정보         │                                        │
│             │                                        │
└─────────────┴────────────────────────────────────────┘
```

- 모달 또는 전체 화면 탭으로 열림
- 사이드 내비: 200px 고정
- 설정 폼 최대 너비: 680px
- 섹션 간 간격: 32px

#### 4.7.5 출시 위자드

```
┌──────────────────────────────────────────────────────┐
│  출시 위자드                                   [닫기]  │
│                                                      │
│  Step 1    Step 2    Step 3    Step 4    Step 5      │
│  분석 ✅   플랫폼 ✅  계정 ●    환경변수 ○   배포 ○    │
│  ─────●─────●─────●─────○─────○────                  │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │                                                │  │
│  │          현재 스텝 콘텐츠                        │  │
│  │          (최대 너비 640px, 중앙 정렬)            │  │
│  │                                                │  │
│  └────────────────────────────────────────────────┘  │
│                                                      │
│         [이전]                         [다음 / 배포]   │
└──────────────────────────────────────────────────────┘
```

- 전체 화면 또는 큰 모달 (최소 800x600)
- 스텝 인디케이터 상단 고정
- 콘텐츠 최대 너비: 640px

#### 4.7.6 워치독 대시보드

```
┌──────────────────────────────────────────────────────┐
│  워치독: 쇼핑몰                    [실시간] [1h] [24h] │
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐                        │
│  │상태│ │업타│ │응답│ │에러│   메트릭 카드 (4열)       │
│  │    │ │임  │ │시간│ │율  │                          │
│  └────┘ └────┘ └────┘ └────┘                        │
│                                                      │
│  ┌────────────────────────────────────────────┐      │
│  │            트래픽 그래프 (전체 너비)          │      │
│  │                 (높이 240px)               │      │
│  └────────────────────────────────────────────┘      │
│                                                      │
│  ┌─────────────────┐  ┌─────────────────────┐       │
│  │  응답시간 분포    │  │  상태코드 분포        │       │
│  │  (2열 그리드)    │  │                     │       │
│  └─────────────────┘  └─────────────────────┘       │
│                                                      │
│  에러 목록 / 로그 스트림 (탭 전환)                      │
│                                                      │
└──────────────────────────────────────────────────────┘
```

- IDE 메인 내 탭 또는 별도 뷰로 열림
- 메트릭 카드: 4열 그리드 (각 최소 160px)
- 그래프 높이: 240px
- 전체 패딩: 24px

#### 4.7.7 계정 & 연결 화면

```
┌──────────────────────────────────────────────────────┐
│  계정 & 연결                                          │
├─────────────┬────────────────────────────────────────┤
│ 사이드 내비   │                                        │
│ (200px)     │  AI 프로바이더                           │
│             │  ─────────────────                     │
│ AI 프로바이더 │  ┌──────────────────────────────┐      │
│ GitHub      │  │ Claude    ● 연결됨  [연결 해제] │      │
│ 출시 플랫폼  │  │ OpenAI    ○ 미연결  [연결하기]  │      │
│ 계정 정보    │  │ Gemini    ○ 미연결  [연결하기]  │      │
│             │  │ Ollama    ○ 감지됨  [설정]     │      │
│             │  └──────────────────────────────┘      │
│             │                                        │
│             │  모델별 역할 지정                         │
│             │  코드 생성:  [Claude Opus ▼]             │
│             │  코드 검증:  [Claude Sonnet ▼]           │
│             │  디버깅:    [Claude Sonnet ▼]            │
│             │                                        │
└─────────────┴────────────────────────────────────────┘
```

- 설정 화면과 동일한 좌측 내비 + 우측 콘텐츠 레이아웃
- 연결 상태 카드: 전체 너비, 높이 64px
- 프로바이더별 브랜드 컬러 아이콘 사용

---

## 5. 컴포넌트 라이브러리

### 5.1 Button

4가지 변형(variant) x 3가지 크기(size):

| 변형 | 배경 | 텍스트 | 테두리 | 호버 |
|------|------|--------|--------|------|
| **Primary** | `--accent-primary` (#58a6ff) | `#ffffff` | 없음 | `--accent-primary-hover` (#79c0ff) |
| **Secondary** | `--bg-elevated` (#2d333b) | `--text-primary` | `--border-default` | `--bg-hover` |
| **Ghost** | `transparent` | `--text-secondary` | 없음 | `--bg-hover` |
| **Danger** | `--accent-danger` (#f85149) | `#ffffff` | 없음 | `#ff6e67` |

| 크기 | 높이 | 패딩 (좌우) | 폰트 크기 | border-radius |
|------|------|-------------|----------|---------------|
| **sm** | 28px | 12px | 12px | 6px |
| **md** | 32px | 16px | 13px | 6px |
| **lg** | 36px | 20px | 14px | 8px |

```css
/* 공통 버튼 스타일 */
.btn {
  font-weight: 600;
  border-radius: 6px;
  transition: background 150ms ease, box-shadow 150ms ease;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.btn:focus-visible {
  outline: 2px solid var(--accent-primary);
  outline-offset: 2px;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

아이콘 버튼: 정사각형 (sm: 28x28, md: 32x32, lg: 36x36), 패딩 없이 아이콘만 중앙 정렬.

### 5.2 Input

| 상태 | 테두리 | 배경 | 그림자 |
|------|--------|------|--------|
| **기본** | `--border-default` | `--bg-input` | 없음 |
| **호버** | `--border-strong` | `--bg-input` | 없음 |
| **포커스** | `--border-accent` | `--bg-input` | `0 0 0 3px rgba(56, 139, 253, 0.3)` |
| **에러** | `--accent-danger` | `--bg-input` | `0 0 0 3px rgba(248, 81, 73, 0.3)` |
| **비활성** | `--border-muted` | `--bg-surface` | 없음 |

```css
.input {
  height: 32px;
  padding: 0 12px;
  font-size: 13px;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--bg-input);
  color: var(--text-primary);
  transition: border-color 150ms ease, box-shadow 150ms ease;
}

.input::placeholder {
  color: var(--text-tertiary);
}
```

변형:
- **Text**: 기본 텍스트 입력
- **Password**: 눈 아이콘으로 표시/숨김 토글
- **Search**: 좌측 돋보기 아이콘, 우측 X(지우기) 버튼, border-radius: 20px

크기:
- **sm**: 높이 28px, 폰트 12px
- **md**: 높이 32px, 폰트 13px (기본)
- **lg**: 높이 36px, 폰트 14px

### 5.3 Select / Dropdown

```css
.select-trigger {
  height: 32px;
  padding: 0 12px;
  padding-right: 32px;  /* 화살표 공간 */
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--bg-input);
  font-size: 13px;
}

.select-dropdown {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  padding: 4px;
  max-height: 320px;
  overflow-y: auto;
}

.select-option {
  height: 32px;
  padding: 0 12px;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
}

.select-option:hover {
  background: var(--bg-hover);
}

.select-option--selected {
  background: var(--accent-primary-muted);
  color: var(--accent-primary);
}
```

- 우측에 ChevronDown 아이콘 (16px)
- 드롭다운 최대 높이: 320px, 스크롤
- 옵션 그룹 지원: 그룹 라벨은 `--text-tertiary`, 12px, 대문자

### 5.4 Modal

```css
.modal-overlay {
  background: var(--bg-overlay);  /* rgba(1, 4, 9, 0.8) */
  backdrop-filter: blur(4px);
}

.modal {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  padding: 24px;
  max-width: 480px;          /* sm */
  /* max-width: 640px;       md */
  /* max-width: 800px;       lg */
  width: 90vw;
  max-height: 85vh;
  overflow-y: auto;
}

.modal-header {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
  padding-right: 32px;       /* 닫기 버튼 공간 */
}

.modal-close {
  position: absolute;
  top: 16px;
  right: 16px;
  width: 32px;
  height: 32px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--border-muted);
}
```

크기:
- **sm**: 480px (확인/취소 다이얼로그)
- **md**: 640px (로그인 모달, 설정 모달)
- **lg**: 800px (출시 위자드, 상세 보기)
- **full**: 90vw x 85vh (코드 diff, 미리보기)

로그인 모달(내장 BrowserView): Electron의 BrowserView를 모달 내부에 임베드. 모달 크기 md(640x500), BrowserView가 모달 콘텐츠 영역을 채운다.

### 5.5 Toast / Notification

```css
.toast {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  padding: 12px 16px;
  min-width: 320px;
  max-width: 480px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
}
```

| 타입 | 아이콘 | 좌측 테두리 색상 |
|------|--------|----------------|
| **Info** | `Info` (Lucide) | `--accent-primary` (#58a6ff) |
| **Success** | `CheckCircle` | `--accent-success` (#3fb950) |
| **Warning** | `AlertTriangle` | `--accent-warning` (#d29922) |
| **Error** | `XCircle` | `--accent-danger` (#f85149) |

- 위치: 우측 하단, 상태바 위 (bottom: 40px, right: 16px)
- 스택: 최대 3개, 아래에서 위로 쌓임 (간격 8px)
- 자동 닫힘: 5초 (에러는 수동 닫기만)
- 좌측에 4px 두께의 컬러 바 (border-left)

### 5.6 Tabs

```css
.tabs {
  display: flex;
  border-bottom: 1px solid var(--border-muted);
  gap: 0;
}

.tab {
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: color 150ms ease, border-color 150ms ease;
}

.tab:hover {
  color: var(--text-primary);
}

.tab--active {
  color: var(--text-primary);
  border-bottom-color: var(--accent-primary);
}
```

에디터 탭 (파일 탭):
- 높이: 35px
- 배경: `--bg-sidebar` (비활성), `--bg-editor` (활성)
- 닫기(X) 버튼: 호버 시 표시, 12px 아이콘
- 수정 표시: 닫기 버튼 자리에 원형 도트 (8px, `--text-secondary`)
- 드래그로 순서 변경 가능

하단 패널 탭 (Console, Network, Problems, Terminal):
- 높이: 32px
- 뱃지: 탭 옆에 카운트 표시 (Problems: 에러/경고 수)

### 5.7 Card

```css
.card {
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  padding: 16px;
  transition: border-color 200ms ease, box-shadow 200ms ease;
}

.card:hover {
  border-color: var(--border-strong);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.card--clickable {
  cursor: pointer;
}
```

서비스 카드 명세:
- 너비: 280px ~ 1fr (그리드)
- 높이: 자동 (콘텐츠 기반, 최소 180px)
- 상단: 서비스명(16px semibold) + 프레임워크 뱃지
- 중단: 상태 인디케이터 + 마지막 활동 시간
- 하단: 출시 URL, 스택 태그

메트릭 카드 (워치독):
- 높이: 100px
- 상단: 라벨 (12px, `--text-secondary`)
- 중앙: 값 (24px ~ 32px, bold)
- 하단: 변화율 (12px, 초록=상승/빨강=하락)

### 5.8 Badge / Tag

```css
.badge {
  display: inline-flex;
  align-items: center;
  height: 20px;
  padding: 0 8px;
  font-size: 11px;
  font-weight: 500;
  border-radius: 10px;  /* pill 형태 */
  white-space: nowrap;
}
```

| 변형 | 배경 | 텍스트 |
|------|------|--------|
| **Default** | `--bg-surface` | `--text-secondary` |
| **Primary** | `--accent-primary-muted` | `--accent-primary` |
| **Success** | `--accent-success-muted` | `--accent-success` |
| **Warning** | `--accent-warning-muted` | `--accent-warning` |
| **Danger** | `--accent-danger-muted` | `--accent-danger` |
| **Purple** | `rgba(188, 140, 255, 0.15)` | `--accent-purple` |

Tag (제거 가능한 뱃지):
- 뱃지와 동일 + 우측에 X(제거) 아이콘 (10px)
- X 호버 시 배경 밝아짐

### 5.9 Tooltip

```css
.tooltip {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 12px;
  color: var(--text-primary);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  max-width: 240px;
  z-index: 1000;
}
```

- 표시 딜레이: 500ms (호버 시작 후)
- 사라짐 딜레이: 100ms
- 위치: 기본 상단, 공간 부족 시 자동 전환
- 화살표: 6px 삼각형

### 5.10 Toggle / Switch

```css
.toggle {
  width: 36px;
  height: 20px;
  border-radius: 10px;
  background: var(--bg-active);     /* OFF 상태 */
  transition: background 150ms ease;
  cursor: pointer;
  position: relative;
}

.toggle--on {
  background: var(--accent-primary);  /* ON 상태 */
}

.toggle-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #ffffff;
  position: absolute;
  top: 2px;
  left: 2px;                          /* OFF */
  transition: transform 150ms ease;
}

.toggle--on .toggle-thumb {
  transform: translateX(16px);         /* ON */
}
```

- 라벨: 토글 좌측, 13px
- 비활성 시: opacity 0.5

### 5.11 Progress Bar

```css
.progress-bar {
  height: 8px;
  border-radius: 4px;
  background: var(--bg-active);
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  border-radius: 4px;
  background: var(--accent-primary);
  transition: width 300ms ease;
}

/* 얇은 버전 (상태바, 탭) */
.progress-bar--thin {
  height: 2px;
  border-radius: 1px;
}
```

| 타입 | 색상 |
|------|------|
| **기본** | `--accent-primary` |
| **성공** | `--accent-success` |
| **경고** | `--accent-warning` |
| **위험** | `--accent-danger` |
| **Indeterminate** | 좌우 반복 애니메이션 (shimmer) |

퍼센트 라벨: 우측 상단, 12px, `--text-secondary`

### 5.12 Sidebar Navigation

```css
.sidebar-nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 32px;
  padding: 0 12px;
  border-radius: 6px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 100ms ease, color 100ms ease;
}

.sidebar-nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.sidebar-nav-item--active {
  background: var(--bg-active);
  color: var(--text-primary);
  font-weight: 500;
}

.sidebar-nav-item__icon {
  width: 16px;
  height: 16px;
  color: inherit;
}

.sidebar-nav-item__badge {
  margin-left: auto;
  /* Badge 컴포넌트 사용 */
}
```

액티비티바 아이콘:
- 크기: 48px x 48px (히트영역), 아이콘 20px
- 활성 시: 좌측에 2px 흰색 바, 아이콘 `--text-primary`
- 비활성: 아이콘 `--text-tertiary`

### 5.13 Tree View (파일 탐색기)

```css
.tree-item {
  display: flex;
  align-items: center;
  height: 22px;
  padding-right: 8px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
}

.tree-item:hover {
  background: var(--bg-hover);
}

.tree-item--selected {
  background: var(--bg-active);
  color: var(--text-primary);
}

.tree-item--focused {
  outline: 1px solid var(--accent-primary);
  outline-offset: -1px;
}

.tree-item__indent {
  width: 16px;           /* 들여쓰기 단위: 레벨당 16px */
  flex-shrink: 0;
}

.tree-item__icon {
  width: 16px;
  height: 16px;
  margin-right: 6px;
  flex-shrink: 0;
}

.tree-item__chevron {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  transition: transform 100ms ease;
}

.tree-item__chevron--expanded {
  transform: rotate(90deg);
}
```

- 들여쓰기: 레벨당 16px (가이드라인 표시: 1px `--border-muted` 세로선)
- 폴더 아이콘: 열림/닫힘 상태
- 파일 아이콘: 파일 확장자별 색상 (5.13 아이콘 시스템 참조)
- 컨텍스트 메뉴: 우클릭으로 이름 변경, 삭제, 복사 경로 등

### 5.14 Chat Bubble (AI 채팅)

```css
/* 사용자 메시지 */
.chat-bubble--user {
  background: var(--accent-primary-muted);
  border-radius: 12px 12px 4px 12px;
  padding: 10px 14px;
  margin-left: 48px;        /* 좌측 여백 (우측 정렬 효과) */
  font-size: 14px;
  line-height: 22px;
  color: var(--text-primary);
}

/* AI 메시지 */
.chat-bubble--ai {
  background: var(--bg-surface);
  border: 1px solid var(--border-muted);
  border-radius: 12px 12px 12px 4px;
  padding: 10px 14px;
  margin-right: 24px;
  font-size: 14px;
  line-height: 22px;
  color: var(--text-primary);
}

/* 시스템 메시지 */
.chat-bubble--system {
  background: var(--bg-surface);
  border-radius: 8px;
  padding: 8px 12px;
  margin: 0 24px;
  font-size: 12px;
  color: var(--text-secondary);
  text-align: center;
}
```

- 아바타: 사용자(우측 상단, 24px 원형), AI(좌측 상단, 24px 원형 + 모델 아이콘)
- 타임스탬프: 메시지 하단 우측, 11px, `--text-tertiary`
- 코드블록: 메시지 내 인라인, 어두운 배경 (#0d1117), 상단에 언어 라벨 + 복사 버튼
- 파일 diff: 인라인 diff 뷰 (추가=초록 배경, 삭제=빨강 배경)
- 승인 버튼: 코드 제안 아래 `[승인하고 적용]` (Primary) + `[수정 요청]` (Secondary)
- 토큰 카운터: 입력 영역 우측 하단, 11px, `--text-tertiary`

채팅 입력 영역:
```css
.chat-input {
  min-height: 40px;
  max-height: 200px;
  padding: 10px 14px;
  padding-right: 44px;     /* 전송 버튼 공간 */
  border: 1px solid var(--border-default);
  border-radius: 12px;
  background: var(--bg-input);
  font-size: 14px;
  line-height: 22px;
  resize: none;
}

.chat-input:focus {
  border-color: var(--border-accent);
  box-shadow: 0 0 0 3px rgba(56, 139, 253, 0.3);
}

.chat-send-btn {
  position: absolute;
  bottom: 8px;
  right: 8px;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: var(--accent-primary);
  color: #ffffff;
}
```

### 5.15 Code Block (구문 강조)

```css
.code-block {
  background: #0d1117;        /* 항상 어두운 배경 (라이트 모드에서도) */
  border: 1px solid var(--border-default);
  border-radius: 8px;
  overflow: hidden;
  font-family: var(--font-code);
  font-size: 13px;
  line-height: 20px;
}

.code-block__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.05);
  border-bottom: 1px solid var(--border-muted);
  font-size: 12px;
  color: var(--text-secondary);
}

.code-block__body {
  padding: 12px 16px;
  overflow-x: auto;
}

.code-block__line-numbers {
  color: var(--text-disabled);
  text-align: right;
  padding-right: 12px;
  user-select: none;
  min-width: 32px;
}
```

- 헤더: 언어 라벨(좌) + 복사/에디터에서 열기 버튼(우)
- 구문 강조: 해당 테마의 Syntax Highlighting 토큰 사용
- 줄 번호: 선택적 표시 (채팅 내에서는 숨김, diff에서는 표시)
- 가로 스크롤: 긴 코드 줄 대응

### 5.16 Diff View

```css
.diff-line--added {
  background: rgba(63, 185, 80, 0.15);
}

.diff-line--added .diff-gutter {
  background: rgba(63, 185, 80, 0.3);
  color: var(--accent-success);
}

.diff-line--removed {
  background: rgba(248, 81, 73, 0.15);
}

.diff-line--removed .diff-gutter {
  background: rgba(248, 81, 73, 0.3);
  color: var(--accent-danger);
}

.diff-line--modified {
  background: rgba(210, 153, 34, 0.1);
}

.diff-gutter {
  width: 44px;
  padding: 0 8px;
  text-align: right;
  font-size: 12px;
  user-select: none;
}

.diff-header {
  padding: 8px 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-default);
  font-size: 12px;
  font-family: var(--font-code);
  color: var(--text-secondary);
}
```

뷰 모드:
- **Inline (통합)**: 한 컬럼에 추가/삭제 교차 표시 (기본)
- **Side-by-side (분할)**: 좌측 이전, 우측 변경 후 (넓은 화면에서)
- 헤더: `--- a/파일명` / `+++ b/파일명`
- 축소된 동일 코드: "... 5줄 동일 ..." 접기

### 5.17 Status Indicator (dot)

```css
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot--online    { background: var(--accent-success); }
.status-dot--offline   { background: var(--text-disabled); }
.status-dot--error     { background: var(--accent-danger); }
.status-dot--warning   { background: var(--accent-warning); }
.status-dot--building  { background: var(--accent-warning); animation: pulse 1.5s ease infinite; }

/* 큰 버전 (대시보드 상태) */
.status-dot--lg {
  width: 10px;
  height: 10px;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
```

서비스 상태 매핑:
- `개발중` -> `--accent-primary` (파란 도트)
- `검증중` -> `--accent-warning` (노란 도트, pulse)
- `출시완료` / `운영중` -> `--accent-success` (초록 도트)
- `오류` -> `--accent-danger` (빨간 도트)
- `중지됨` -> `--text-disabled` (회색 도트)

### 5.18 Avatar

```css
.avatar {
  border-radius: 50%;
  object-fit: cover;
  background: var(--bg-active);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-weight: 600;
  flex-shrink: 0;
}

.avatar--xs { width: 20px; height: 20px; font-size: 10px; }
.avatar--sm { width: 24px; height: 24px; font-size: 11px; }
.avatar--md { width: 32px; height: 32px; font-size: 13px; }
.avatar--lg { width: 40px; height: 40px; font-size: 16px; }
.avatar--xl { width: 64px; height: 64px; font-size: 24px; }
```

- 이미지가 없을 때: 이니셜 표시 (배경 `--accent-primary-muted`, 텍스트 `--accent-primary`)
- GitHub 아바타: GitHub API에서 가져온 프로필 이미지
- AI 아바타: 모델별 브랜드 아이콘 (Claude=보라, GPT=초록, Gemini=파랑)

### 5.19 Breadcrumb

```css
.breadcrumb {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
}

.breadcrumb__item {
  color: var(--text-secondary);
  cursor: pointer;
}

.breadcrumb__item:hover {
  color: var(--text-primary);
  text-decoration: underline;
}

.breadcrumb__item--current {
  color: var(--text-primary);
  cursor: default;
}

.breadcrumb__separator {
  color: var(--text-disabled);
  font-size: 10px;
  /* ChevronRight 아이콘 또는 '/' 문자 */
}
```

- 에디터 상단에 파일 경로 표시: `src / app / cart / page.tsx`
- 각 세그먼트 클릭으로 해당 디렉토리 이동

### 5.20 Context Menu (우클릭)

```css
.context-menu {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  padding: 4px;
  min-width: 200px;
  max-width: 300px;
  z-index: 1100;
}

.context-menu__item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 30px;
  padding: 0 12px;
  border-radius: 4px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
}

.context-menu__item:hover {
  background: var(--accent-primary);
  color: #ffffff;
}

.context-menu__item--danger:hover {
  background: var(--accent-danger);
}

.context-menu__shortcut {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-tertiary);
}

.context-menu__separator {
  height: 1px;
  background: var(--border-muted);
  margin: 4px 8px;
}

.context-menu__submenu-arrow {
  margin-left: auto;
  /* ChevronRight 아이콘 12px */
}
```

- 호버 시 전체 행이 `--accent-primary` 배경 (텍스트 흰색)
- 단축키: 우측 정렬, 작은 텍스트
- 구분선: 메뉴 그룹 사이
- 서브메뉴: 우측 화살표, 호버 시 서브메뉴 표시 (200ms 딜레이)
- 비활성 항목: `--text-disabled`, 호버 없음

---

## 6. 아이콘 시스템

### 6.1 기본 아이콘: Lucide Icons

Lucide Icons (https://lucide.dev)를 기본 아이콘 라이브러리로 사용한다.

크기 규격:
- **xs**: 12px (배지 내, 인라인)
- **sm**: 14px (버튼 내, 사이드바 항목)
- **md**: 16px (기본 UI 아이콘)
- **lg**: 20px (액티비티바, 헤더)
- **xl**: 24px (빈 상태, 온보딩)

주요 매핑:

| 용도 | 아이콘 이름 | 설명 |
|------|-----------|------|
| 파일 탐색기 | `Files` | 사이드바 탭 |
| 검색 | `Search` | 전체 검색 |
| Git/소스 컨트롤 | `GitBranch` | 소스 컨트롤 탭 |
| 출시 | `Rocket` | 출시 센터 |
| 워치독 | `Activity` | 모니터링 |
| 설정 | `Settings` | 설정 |
| AI 채팅 | `MessageSquare` | 채팅 패널 |
| 터미널 | `Terminal` | 터미널 탭 |
| 닫기 | `X` | 모달/탭 닫기 |
| 추가 | `Plus` | 새 서비스/파일 생성 |
| 삭제 | `Trash2` | 삭제 액션 |
| 편집 | `Pencil` | 이름 변경 |
| 복사 | `Copy` | 클립보드 복사 |
| 새로고침 | `RefreshCw` | 새로고침 |
| 링크 | `ExternalLink` | 외부 링크 열기 |
| 다운로드 | `Download` | 파일 다운로드 |
| 업로드 | `Upload` | 파일 업로드 |
| 접기 | `ChevronDown` / `ChevronRight` | 트리 접기/펼치기 |
| 정렬 | `ArrowUpDown` | 정렬 변경 |
| 필터 | `Filter` | 필터 적용 |
| 알림 | `Bell` | 알림 센터 |
| 사용자 | `User` | 프로필 |
| 로그아웃 | `LogOut` | 로그아웃 |
| 전송 | `Send` | 채팅 메시지 전송 |
| 잠금 | `Lock` | 보안/비밀번호 |
| 눈 | `Eye` / `EyeOff` | 패스워드 표시/숨김 |

### 6.2 파일 타입 아이콘 (Material File Icons 스타일)

파일 확장자별 고유 색상 아이콘을 사용한다. Material File Icons 컬러 팔레트 참고:

| 파일 타입 | 색상 | 아이콘 형태 |
|----------|------|-----------|
| `.ts` / `.tsx` | `#3178c6` (TypeScript Blue) | TS 로고 |
| `.js` / `.jsx` | `#f7df1e` (JavaScript Yellow) | JS 로고 |
| `.py` | `#3776ab` (Python Blue) | 뱀 아이콘 |
| `.html` | `#e34f26` (HTML Orange) | HTML 태그 |
| `.css` / `.scss` | `#1572b6` (CSS Blue) | CSS 아이콘 |
| `.json` | `#cbcb41` (Yellow) | 중괄호 |
| `.md` | `#519aba` (Markdown Blue) | M 아이콘 |
| `.env` | `#ecd53f` (Yellow) | 열쇠 |
| `.gitignore` | `#f05032` (Git Red) | Git 아이콘 |
| `.svg` | `#ffb13b` (Orange) | 이미지 |
| `.png` / `.jpg` | `#a074c4` (Purple) | 이미지 |
| `package.json` | `#e8274b` (npm Red) | npm 아이콘 |
| `tsconfig.json` | `#3178c6` | TS 설정 |
| `Dockerfile` | `#2496ed` (Docker Blue) | Docker 아이콘 |
| `.yaml` / `.yml` | `#cb171e` (Red) | 설정 아이콘 |
| 기본(알 수 없음) | `--text-tertiary` | 일반 문서 아이콘 |

폴더 아이콘:
- 기본 폴더: `--accent-primary` (#58a6ff)
- `src/`: 코드 아이콘 (초록)
- `public/`: 지구본 아이콘 (파랑)
- `node_modules/`: npm 아이콘 (빨강, 반투명)
- `.git/`: Git 아이콘 (빨강)
- `components/`: 블록 아이콘 (보라)
- `tests/` / `__tests__/`: 체크 아이콘 (초록)
- `lib/` / `utils/`: 책 아이콘 (노랑)

### 6.3 상태 아이콘

| 상태 | 아이콘 | 색상 | 애니메이션 |
|------|--------|------|-----------|
| 성공 | `CheckCircle` | `--accent-success` | 없음 |
| 에러 | `XCircle` | `--accent-danger` | 없음 |
| 경고 | `AlertTriangle` | `--accent-warning` | 없음 |
| 정보 | `Info` | `--accent-info` | 없음 |
| 로딩 | `Loader2` | `--text-secondary` | `spin 1s linear infinite` |
| 진행중 | `Circle` (half-filled) | `--accent-primary` | `pulse 1.5s ease infinite` |
| 대기 | `Circle` (outline) | `--text-disabled` | 없음 |

### 6.4 내비게이션 아이콘 (액티비티바)

좌측 액티비티바에 세로로 배열되는 아이콘:

| 순서 | 아이콘 | 라벨 (툴팁) | 기능 |
|------|--------|------------|------|
| 1 | `Files` | 파일 탐색기 | 파일 트리 사이드바 |
| 2 | `Search` | 검색 | 전체 검색 사이드바 |
| 3 | `GitBranch` | 소스 컨트롤 | Git 패널 |
| 4 | `Rocket` | 출시 | 출시 센터 |
| 5 | `Activity` | 워치독 | 모니터링 대시보드 |
| 6 | `Shield` | 보안 | 보안/품질 검증 |
| --- | (구분선) | | |
| 하단 | `Settings` | 설정 | 설정 화면 |
| 하단 | `User` | 계정 | 계정 & 연결 |

---

## 7. 애니메이션 & 트랜지션

### 7.1 기본 타이밍 함수

```css
--ease-default: cubic-bezier(0.25, 0.1, 0.25, 1);      /* ease */
--ease-in: cubic-bezier(0.42, 0, 1, 1);
--ease-out: cubic-bezier(0, 0, 0.58, 1);
--ease-in-out: cubic-bezier(0.42, 0, 0.58, 1);
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);      /* 약간의 바운스 */
```

### 7.2 트랜지션 명세

| 요소 | 속성 | 지속시간 | 타이밍 |
|------|------|---------|--------|
| **패널 전환** (사이드바 접기/펼치기) | `width`, `opacity` | 200ms | `ease-out` |
| **패널 리사이즈** | `width`, `height` | 0ms (실시간) | — |
| **모달 열기** | `opacity`, `transform` | 150ms | `ease-out` |
| **모달 닫기** | `opacity`, `transform` | 100ms | `ease-in` |
| **토스트 진입** | `transform`, `opacity` | 300ms | `ease-spring` |
| **토스트 퇴장** | `transform`, `opacity` | 200ms | `ease-in` |
| **드롭다운 열기** | `opacity`, `transform` | 150ms | `ease-out` |
| **드롭다운 닫기** | `opacity` | 100ms | `ease-in` |
| **탭 전환** | `border-color`, `color` | 150ms | `ease` |
| **버튼 호버** | `background`, `box-shadow` | 150ms | `ease` |
| **입력 포커스** | `border-color`, `box-shadow` | 150ms | `ease` |
| **리스트 항목 호버** | `background` | 100ms | `ease` |
| **트리 항목 접기/펼치기** | `height` | 200ms | `ease-out` |
| **토글 전환** | `background`, `transform` | 150ms | `ease` |
| **프로그레스바 채우기** | `width` | 300ms | `ease` |
| **페이지 전환** | `opacity` | 200ms | `ease` |

### 7.3 모달 애니메이션

```css
/* 열기 */
@keyframes modal-enter {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

/* 닫기 */
@keyframes modal-exit {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.95);
  }
}

/* 오버레이 */
@keyframes overlay-enter {
  from { opacity: 0; }
  to { opacity: 1; }
}
```

### 7.4 토스트 애니메이션

```css
/* 우측 하단에서 슬라이드 인 */
@keyframes toast-enter {
  from {
    opacity: 0;
    transform: translateX(100%);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

/* 슬라이드 아웃 */
@keyframes toast-exit {
  from {
    opacity: 1;
    transform: translateX(0);
  }
  to {
    opacity: 0;
    transform: translateX(100%);
  }
}
```

### 7.5 로딩 스피너

```css
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.spinner {
  animation: spin 1s linear infinite;
  color: var(--text-secondary);
}

/* 크기 */
.spinner--sm { width: 16px; height: 16px; }
.spinner--md { width: 24px; height: 24px; }
.spinner--lg { width: 40px; height: 40px; }
```

### 7.6 스켈레톤 로딩

```css
@keyframes skeleton-shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.skeleton {
  background: linear-gradient(
    90deg,
    var(--bg-surface) 25%,
    var(--bg-active) 50%,
    var(--bg-surface) 75%
  );
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.5s ease infinite;
  border-radius: 4px;
}

/* 텍스트 줄 */
.skeleton--text { height: 14px; margin-bottom: 8px; }
.skeleton--title { height: 20px; width: 60%; margin-bottom: 12px; }
.skeleton--avatar { width: 32px; height: 32px; border-radius: 50%; }
.skeleton--card { height: 180px; border-radius: 8px; }
```

### 7.7 AI 타이핑 애니메이션

```css
/* 점 3개 깜빡임 */
@keyframes typing-dot {
  0%, 80%, 100% { opacity: 0.3; }
  40% { opacity: 1; }
}

.typing-indicator {
  display: flex;
  gap: 4px;
  padding: 12px 16px;
}

.typing-indicator__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-secondary);
  animation: typing-dot 1.4s ease infinite;
}

.typing-indicator__dot:nth-child(1) { animation-delay: 0ms; }
.typing-indicator__dot:nth-child(2) { animation-delay: 200ms; }
.typing-indicator__dot:nth-child(3) { animation-delay: 400ms; }
```

AI 응답 스트리밍 시에는 타이핑 인디케이터 대신, 텍스트가 한 글자씩 나타나는 방식을 사용한다. 커서(블링크)를 텍스트 끝에 표시한다:

```css
@keyframes blink-cursor {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.ai-cursor {
  display: inline-block;
  width: 2px;
  height: 1em;
  background: var(--accent-primary);
  animation: blink-cursor 1s step-end infinite;
  margin-left: 1px;
  vertical-align: text-bottom;
}
```

---

## 8. 각 화면별 상세 디자인 스펙

### 8.1 온보딩/초기 설정

전체 4스텝 위자드, 풀스크린 레이아웃.

**Step 1: 웰컴 화면**
- 중앙에 VidEplace 로고 (SVG, 48px 높이)
- 아래 캐치프레이즈: "아이디어를 작성하세요. 나머지는 AI가 해드립니다." (20px, `--text-primary`)
- 서브텍스트: 간단한 기능 소개 3줄 (14px, `--text-secondary`)
- `[시작하기]` Primary 버튼 (lg)
- 하단: "이미 서비스가 있나요? [기존 서비스 열기]" 링크

**Step 2: AI 프로바이더 연결**
- 제목: "AI 서비스를 연결하세요" (24px, semibold)
- 설명: "사용 중인 AI 서비스에 로그인하면 바로 사용할 수 있어요" (14px, `--text-secondary`)
- 프로바이더 카드 리스트 (세로 배열):
  - `Claude 로그인` - 보라색 아이콘 (#bc8cff), 카드 좌측 4px 보라 바
  - `ChatGPT 로그인` - 초록색 아이콘 (#3fb950)
  - `Google (Gemini) 로그인` - 파란색 아이콘 (#58a6ff)
  - `Ollama (로컬)` - 회색 아이콘
- 각 카드: 높이 64px, border-radius 8px, 호버 시 `--border-strong`
- 구분선: "또는"
- `[API 키 직접 입력 (고급)]` Ghost 버튼
- 연결 완료 시 카드에 체크마크 + "연결됨" 뱃지 (Success)

**Step 3: GitHub 연결**
- 제목: "GitHub 계정을 연결하세요" (24px)
- `[GitHub로 로그인]` 큰 버튼 (검정 배경 + GitHub 로고)
- 연결 후 자동 셋업 진행 표시:
  - 체크리스트 (CheckCircle 아이콘 + 텍스트)
  - OAuth 인증 완료, SSH 키 생성, SSH 키 등록, Git 설정
  - 각 항목이 순차적으로 완료 애니메이션
- `[건너뛰기]` Ghost 버튼 (나중에 설정 가능)

**Step 4: 테마 선택**
- 제목: "테마를 선택하세요" (24px)
- 테마 카드 3개 (가로 배열):
  - Dark (기본 선택), Light, Monokai
  - 각 카드: 160px x 120px, 테마 미리보기 이미지
  - 선택됨: `--border-accent` 2px 테두리 + 체크마크 뱃지
- 하단: `[VidEplace 시작하기]` Primary 버튼 (lg)

**스텝 인디케이터:**
- 4개 원형 도트 (10px), 연결 라인
- 완료: `--accent-primary` 채워진 원
- 현재: `--accent-primary` 테두리 + 진행 애니메이션
- 미완료: `--border-default` 빈 원

### 8.2 서비스 대시보드

**상단 네비게이션 바 (56px)**
- 좌측: VidEplace 로고 (24px) + "내 서비스" 텍스트
- 우측: 알림 벨 아이콘 (뱃지 카운트) + 사용자 아바타 (32px) + 드롭다운

**서비스 카드 그리드**
- 그리드: `repeat(auto-fill, minmax(280px, 1fr))`
- 카드 구조:
  ```
  ┌─────────────────────────────┐
  │  서비스명          ● 운영중  │  <- 16px semibold + status dot
  │  Next.js + Supabase         │  <- 12px, text-secondary, 스택 뱃지
  │                             │
  │  마지막 활동: 2분 전          │  <- 12px, text-tertiary
  │  shop.vercel.app ↗          │  <- 12px, 링크
  │                             │
  │  트래픽: 2.3k/일  에러: 0    │  <- 11px, 요약 메트릭
  └─────────────────────────────┘
  ```
- 호버 시: `border-color` 밝아짐, 미세한 `box-shadow`
- 클릭: 해당 서비스의 IDE 메인 화면으로 진입

**새 서비스 카드 (+ 버튼)**
- 점선 테두리 (`dashed`), 중앙에 `Plus` 아이콘 + "새 서비스"
- 호버 시 배경 밝아짐
- 클릭 시 모달: 생성 방식 선택 (PRD 작성 / 폴더 연결 / GitHub 클론)

**최근 활동 섹션**
- 타이틀: "최근 활동" (16px, semibold)
- 리스트: 각 항목 높이 40px
  - 좌측: 프로젝트 아이콘 (16px) + 서비스명
  - 중앙: 활동 설명 ("출시 완료", "AI 코드 생성 중...")
  - 우측: 상대 시간 ("2분 전")
- 상태별 아이콘 색상: 출시=초록, 생성중=파랑(spin), 에러=빨강

### 8.3 IDE 메인 화면

#### 사이드바 (240px)

**파일 탐색기 (기본 탭)**
- 헤더: "탐색기" (12px, 대문자, `--text-secondary`) + 액션 아이콘들 (새 파일, 새 폴더, 접기)
- 파일 트리: Tree View 컴포넌트 (5.13)
- 하단 구분선 아래: 서비스 패널

**서비스 패널 (사이드바 하단)**
- 접을 수 있는 섹션
- AI 프로바이더 상태:
  - 연결된 서비스: 아이콘 + 이름 + 상태 도트
  - 선택된 모델: 드롭다운 (인라인)
- Git 상태: 브랜치명 + 변경 파일 수
- 출시 상태: 플랫폼명 + 상태 도트

#### 에디터 탭바 + Monaco (메인 영역)

**탭바 (35px)**
- 탭: 파일 아이콘(16px) + 파일명(12px) + 닫기(X, 12px)
- 활성 탭: `--bg-editor` 배경, 상단 2px `--accent-primary` 라인
- 비활성 탭: `--bg-sidebar` 배경
- 수정됨 표시: 닫기 버튼 자리에 8px 원형 도트 (`--text-secondary`)
- 탭 overflow: 좌우 스크롤 화살표 또는 드롭다운

**에디터 (Monaco Editor)**
- 배경: `--bg-editor`
- 좌측 거터: 줄 번호 (40px 너비, `--text-disabled`), 줄 접기 아이콘
- 현재 줄 하이라이트: `rgba(255, 255, 255, 0.04)`
- 선택 영역: `rgba(56, 139, 253, 0.3)`
- 미니맵: 우측 80px (옵션)
- 상단 Breadcrumb: 파일 경로 (에디터 내 상단)

#### AI 채팅 패널 (360px)

**패널 헤더 (40px)**
- 좌측: "AI 채팅" (13px, semibold) + 현재 대화 세션명
- 우측: 모델 선택 드롭다운 (Claude Opus ▼) + 새 대화 버튼 (+)

**메시지 영역 (스크롤)**
- 상단: 시스템 메시지 (서비스 컨텍스트 정보)
- 메시지 버블: Chat Bubble 컴포넌트 (5.14)
- 코드블록: Code Block 컴포넌트 (5.15), 채팅 내 인라인
- 파일 생성 진행률:
  ```
  ── 파일 생성 중 ──
  ✅ src/app/page.tsx          (diff 보기)
  ✅ src/app/products/page.tsx (diff 보기)
  🔄 src/app/cart/page.tsx     생성 중...
  ○  src/app/checkout/page.tsx  대기
  ```
  - 완료: CheckCircle (초록) + 파일명 (링크) + "diff 보기" 링크
  - 진행중: Loader2 (spin) + 파일명
  - 대기: Circle (회색) + 파일명

**승인/거절 버튼 영역**
- AI 코드 제안 아래:
  - `[승인하고 적용]` Primary 버튼 (md)
  - `[수정 요청]` Secondary 버튼 (md)
  - `[무시]` Ghost 버튼 (md)

**토큰 카운터**
- 입력 영역 하단: "토큰: 1,234 / ~$0.02" (11px, `--text-tertiary`)
- 좌측 아이콘: `Coins` (Lucide)
- 경고 임계치 초과 시 `--accent-warning` 색상

**입력 영역 (하단 고정)**
- Chat Input 컴포넌트 (5.14)
- 입력 상단: 첨부 파일 / `@파일명` 컨텍스트 태그 영역
- 좌측 아이콘: 이미지 첨부 (`Image`), `@` 파일 태그
- 우측: 전송 버튼 (Send)

#### 미리보기 패널

**주소바 (36px)**
- 좌측: 뒤로/앞으로 화살표 (ChevronLeft/Right)
- 중앙: URL 입력 필드 (`localhost:3000`, 수정 가능)
- 우측: 새로고침 (RefreshCw) + 뷰포트 전환 버튼 3개
  - `Monitor` (데스크톱, 100%)
  - `Tablet` (태블릿, 768px)
  - `Smartphone` (모바일, 375px)

**BrowserView 영역**
- Electron BrowserView로 실제 웹 페이지 렌더링
- 뷰포트 전환 시: 중앙 정렬된 디바이스 프레임 + 크기 조절 애니메이션 (200ms)
- 하단 바: "핫 리로드: ✅ 활성 | 마지막 업데이트: 14:25" (11px)

#### 디버그 패널 (하단 200px)

**패널 헤더 (32px)**
- 탭: Console | Network | Problems | Terminal
- Problems 탭에 에러/경고 카운트 뱃지
- 우측: 패널 최대화 (Maximize2) / 닫기 (X)

**Console 탭**
- 모노스페이스 폰트 (JetBrains Mono, 12px)
- 각 로그 라인: 타임스탬프(11px, `--text-tertiary`) + 레벨 뱃지 + 메시지
- 레벨별 색상:
  - `info`: `--text-primary`
  - `warn`: `--accent-warning`
  - `error`: `--accent-danger` + 배경 `--accent-danger-muted`
- 에러 옆 `[AI에게 수정 요청]` Ghost 버튼 (11px)
- 하단: 필터 드롭다운 (All/Info/Warn/Error) + 검색 + 지우기

**Network 탭**
- 테이블: Method | URL | Status | Time | Size
- 상태코드 색상: 2xx=초록, 3xx=파랑, 4xx=노랑, 5xx=빨강
- 행 클릭: 상세 패널 (Request/Response 헤더, Body)
- 500 에러 행: 빨간 배경 + `[AI에게 원인 분석 요청]` 버튼

**Problems 탭**
- 리스트: 아이콘 + 파일:라인 + 메시지 + 소스(ESLint/TypeScript/Security)
- 심각도: Error(빨강 XCircle), Warning(노랑 AlertTriangle), Info(파랑 Info)
- 보안 이슈: Shield 아이콘 + "HIGH/MED/LOW" 뱃지
- 행 클릭: 에디터에서 해당 위치로 이동

**Terminal 탭**
- xterm.js 기반 터미널 에뮬레이터
- 배경: `#0d1117` (가장 어두운 배경)
- 폰트: JetBrains Mono, 13px
- 커서: 블록 커서, `--accent-primary` 색상
- 여러 터미널 인스턴스: 탭 또는 분할

### 8.4 Git 패널

사이드바에서 `GitBranch` 아이콘 클릭 시 표시.

**소스 컨트롤 섹션**
- 브랜치: 현재 브랜치명 + 드롭다운 (브랜치 전환)
- 변경사항 트리:
  - "Staged Changes (2)" 접을 수 있는 그룹
  - "Changes (3)" 접을 수 있는 그룹
  - 각 파일: 상태 아이콘 (M=수정/A=추가/D=삭제/R=이름변경) + 파일명
    - M: `--accent-warning` (노랑)
    - A: `--accent-success` (초록)
    - D: `--accent-danger` (빨강)
    - R: `--accent-purple` (보라)
  - 파일 클릭: 에디터에서 diff 보기
  - 우측 아이콘: Stage(+) / Unstage(-) / Discard

**커밋 영역**
- 커밋 메시지 입력: TextArea (최소 32px, 최대 120px)
- `[AI 커밋 메시지 생성]` Ghost 버튼 (아이콘: MessageSquare)
- `[커밋]` Primary 버튼 + `[커밋 & 푸시]` Secondary 버튼

**동기화 영역**
- Push/Pull 카운트: "↑ 0 ↓ 2"
- `[푸시]` `[풀]` `[페치]` `[동기화]` 아이콘 버튼 (Ghost)

**PR 목록 (GitHub 탭)**
- 섹션 토글: "Pull Requests"
- 각 PR 카드:
  - `#42 feat: 장바구니 기능` (13px)
  - `feat/cart -> main` (12px, `--text-secondary`)
  - 상태 뱃지: Open(초록)/Merged(보라)/Closed(빨강)
  - 변경량: `+234 -12`
  - 액션: `[diff 보기]` `[머지]` `[AI 리뷰]`

**Diff 뷰**
- 에디터 영역에 열림 (별도 탭)
- Diff View 컴포넌트 (5.16)
- 상단: 파일명 + 변경 요약 (+N -M)
- 뷰 모드 토글: Inline / Side-by-side

### 8.5 출시 화면

사이드바에서 `Rocket` 아이콘 클릭 시 표시.

**출시 대시보드 (기본 뷰)**
- 현재 출시 상태 카드 (환경별):
  - Production: URL + 버전 + 출시 시간 + 상태 도트
  - Staging: 동일 구조
  - Preview: PR 기반 자동 출시
- 카드 액션: `[롤백]` `[재출시]` `[로그]` `[Prod 승격]`
- 출시 히스토리: 테이블 (버전, 커밋, 메시지, 시간, 상태)

**출시 위자드 (첫 출시)**
- 레이아웃: 4.7.5 참조
- 총 6스텝: 서비스 분석 -> 플랫폼 추천 -> 계정 연결 -> 환경변수 -> 검증 게이트 -> 출시 실행
- 각 스텝 완료 시 체크마크 애니메이션
- 플랫폼 추천: 별점으로 추천도 표시, 월 비용 함께 표시
- 환경변수: 자동 감지된 변수 목록 + 입력 필드, 민감 값은 마스킹
- 검증 게이트: 체크리스트 (성공=초록, 경고=노랑, 실패=빨강)

**실시간 빌드 로그**
- 프로그레스바: 상단 전체 너비 (Progress Bar - thin)
- 스텝 리스트: 체크마크 + 스텝명 + 소요 시간
- 로그 스트림: 터미널 스타일 (어두운 배경, 모노스페이스)

### 8.6 워치독 화면

사이드바에서 `Activity` 아이콘 클릭 시 표시.

**대시보드 레이아웃**
- 상단: 서비스명 + 기간 선택 탭 (실시간/1h/24h/7d/30d)
- 메트릭 카드 행 (4열 그리드):
  - 서비스 상태 (도트 + "정상"/"장애")
  - 업타임 (99.97%)
  - 평균 응답시간 (142ms)
  - 에러율 (0.3%)
- 트래픽 그래프: 전체 너비, 높이 240px
  - X축: 시간, Y축: req/s
  - 라인 색상: `--accent-primary`
  - 호버: 수직 가이드라인 + 툴팁 (정확한 값)
- 2열 그리드:
  - 응답시간 분포 (p50/p90/p95/p99 바 차트)
  - 상태코드 분포 (2xx/3xx/4xx/5xx 가로 누적 바)

**에러 목록**
- 테이블 또는 카드 리스트
- 각 에러: 에러 ID + 타입 + 발생 횟수 + 심각도 뱃지 + 영향 사용자 수
- 펼치면: 스택트레이스 + AI 분석 결과 + `[AI 자동 수정]` 버튼
- 에러 추세 미니 그래프 (sparkline, 높이 40px)

**로그 스트림**
- 터미널 스타일, 실시간 자동 스크롤
- 필터: 레벨 (info/warn/error) + 텍스트 검색
- 각 로그 라인: 타임스탬프 + 레벨 뱃지 + 메시지
- 에러 로그 옆 `[AI 분석]` 버튼

**AI 인사이트 카드**
- 배경: `--bg-surface` + 좌측 4px `--accent-primary` 바
- AI 아이콘 + 메시지 텍스트 (14px)
- 액션 버튼 행: `[실행]` `[나중에]` `[무시]`

### 8.7 설정 화면

레이아웃: 4.7.4 참조.

**사이드 내비게이션 (200px)**
- 섹션 목록: 일반, 에디터, AI 프로바이더, GitHub, 출시, 보안, 키바인딩, 정보
- 각 항목: 16px 아이콘 + 라벨 (13px)
- 활성 항목: `--bg-active` 배경 + `--text-primary` + 좌측 2px `--accent-primary` 바

**설정 폼 영역 (최대 680px)**
- 섹션 제목: 16px, semibold, 하단 구분선
- 각 설정 항목:
  ```
  ┌────────────────────────────────────┐
  │  라벨 (13px, semibold)             │
  │  설명 (12px, text-secondary)       │
  │                                    │
  │  [입력/선택/토글 컴포넌트]            │
  └────────────────────────────────────┘
  ```
- 항목 간 간격: 24px
- 섹션 간 간격: 32px + 구분선

설정 항목 유형:
- 드롭다운: 테마, 언어, 폰트, 모델 선택
- 토글: 자동 저장, 미니맵, 줄 번호, 다크모드
- 슬라이더: 폰트 크기, 탭 크기
- 텍스트 입력: API 키, 커스텀 URL
- 키바인딩: 키 조합 입력 위젯 (Press key... 상태)

### 8.8 내장 로그인 모달

AI 프로바이더 및 외부 서비스 로그인 시 사용하는 모달.

**구조:**
```
┌──────────────────────────────────────────┐
│  모달 오버레이 (bg-overlay + blur)         │
│                                          │
│    ┌──────────────────────────────────┐  │
│    │  [← 뒤로]  Claude 로그인   [✕]   │  │  <- 모달 헤더 (40px)
│    ├──────────────────────────────────┤  │
│    │                                  │  │
│    │     Electron BrowserView        │  │  <- 640px x 500px
│    │     (claude.ai 로그인 페이지)     │  │
│    │                                  │  │
│    │                                  │  │
│    │                                  │  │
│    └──────────────────────────────────┘  │
│                                          │
└──────────────────────────────────────────┘
```

- 모달 크기: 640px x 560px (헤더 포함)
- 오버레이: `--bg-overlay` + `backdrop-filter: blur(4px)`
- 헤더: 뒤로 버튼 (이전 URL) + 서비스명 + 닫기 버튼
- BrowserView: 모달 콘텐츠 영역 전체를 채움
- 로그인 완료 감지: URL/쿠키 변화 모니터링 -> 자동 모달 닫기 + 성공 토스트
- 로딩 상태: BrowserView 위에 스켈레톤/스피너 오버레이
- 보안: 모달 외부 클릭/ESC로 닫기 가능, URL 바 표시 (피싱 방지)

---

## 9. 다크/라이트 모드 전환 가이드

### 9.1 CSS 변수 기반 테마 시스템

모든 색상은 CSS 커스텀 프로퍼티(변수)로 정의한다. 테마 전환 시 루트 요소의 `data-theme` 속성만 바꾸면 된다.

```css
/* 기본 (Dark 테마) */
:root,
[data-theme="dark"] {
  --bg-canvas: #0d1117;
  --bg-sidebar: #161b22;
  --bg-editor: #1c2128;
  --bg-surface: #21262d;
  --bg-elevated: #2d333b;
  --bg-overlay: rgba(1, 4, 9, 0.8);
  --bg-input: #0d1117;
  --bg-hover: rgba(177, 186, 196, 0.12);
  --bg-active: rgba(177, 186, 196, 0.2);

  --text-primary: #e6edf3;
  --text-secondary: #8b949e;
  --text-tertiary: #6e7681;
  --text-disabled: #484f58;
  --text-link: #58a6ff;
  --text-on-accent: #ffffff;

  --accent-primary: #58a6ff;
  --accent-primary-hover: #79c0ff;
  --accent-primary-muted: rgba(56, 139, 253, 0.4);
  --accent-success: #3fb950;
  --accent-success-muted: rgba(63, 185, 80, 0.15);
  --accent-danger: #f85149;
  --accent-danger-muted: rgba(248, 81, 73, 0.15);
  --accent-warning: #d29922;
  --accent-warning-muted: rgba(210, 153, 34, 0.15);

  --border-default: #30363d;
  --border-muted: #21262d;
  --border-strong: #484f58;
  --border-accent: #58a6ff;

  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.3);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.4);
  --shadow-xl: 0 16px 48px rgba(0, 0, 0, 0.5);
}

[data-theme="light"] {
  --bg-canvas: #ffffff;
  --bg-sidebar: #f6f8fa;
  --bg-editor: #ffffff;
  --bg-surface: #f6f8fa;
  --bg-elevated: #ffffff;
  --bg-overlay: rgba(27, 31, 36, 0.5);
  --bg-input: #ffffff;
  --bg-hover: rgba(208, 215, 222, 0.32);
  --bg-active: rgba(208, 215, 222, 0.48);

  --text-primary: #1f2328;
  --text-secondary: #656d76;
  --text-tertiary: #8b949e;
  --text-disabled: #b1bac4;
  --text-link: #0969da;
  --text-on-accent: #ffffff;

  --accent-primary: #0969da;
  --accent-primary-hover: #0550ae;
  --accent-primary-muted: rgba(9, 105, 218, 0.15);
  --accent-success: #1a7f37;
  --accent-success-muted: rgba(26, 127, 55, 0.15);
  --accent-danger: #cf222e;
  --accent-danger-muted: rgba(207, 34, 46, 0.15);
  --accent-warning: #9a6700;
  --accent-warning-muted: rgba(154, 103, 0, 0.15);

  --border-default: #d0d7de;
  --border-muted: #e6edf3;
  --border-strong: #8b949e;
  --border-accent: #0969da;

  --shadow-sm: 0 1px 3px rgba(27, 31, 36, 0.1);
  --shadow-md: 0 4px 12px rgba(27, 31, 36, 0.15);
  --shadow-lg: 0 8px 24px rgba(27, 31, 36, 0.2);
  --shadow-xl: 0 16px 48px rgba(27, 31, 36, 0.25);
}

[data-theme="monokai"] {
  --bg-canvas: #1e1f1c;
  --bg-sidebar: #252620;
  --bg-editor: #272822;
  --bg-surface: #2d2e27;
  --bg-elevated: #3e3d32;
  --bg-overlay: rgba(0, 0, 0, 0.7);
  --bg-input: #1e1f1c;
  --bg-hover: rgba(255, 255, 255, 0.08);
  --bg-active: rgba(255, 255, 255, 0.15);

  --text-primary: #f8f8f2;
  --text-secondary: #a6a18a;
  --text-tertiary: #75715e;
  --text-disabled: #5b5a4f;
  --text-link: #66d9ef;
  --text-on-accent: #ffffff;

  --accent-primary: #66d9ef;
  --accent-primary-hover: #89e3f5;
  --accent-primary-muted: rgba(102, 217, 239, 0.3);
  --accent-success: #a6e22e;
  --accent-success-muted: rgba(166, 226, 46, 0.15);
  --accent-danger: #f92672;
  --accent-danger-muted: rgba(249, 38, 114, 0.15);
  --accent-warning: #e6db74;
  --accent-warning-muted: rgba(230, 219, 116, 0.15);

  --border-default: #3e3d32;
  --border-muted: #2d2e27;
  --border-strong: #75715e;
  --border-accent: #66d9ef;

  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.3);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.4);
  --shadow-xl: 0 16px 48px rgba(0, 0, 0, 0.5);
}
```

### 9.2 테마 전환 로직

```typescript
// 테마 변경 함수
function setTheme(theme: 'dark' | 'light' | 'monokai') {
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem('videplace-theme', theme);

  // Monaco Editor 테마도 동기화
  monaco.editor.setTheme(
    theme === 'light' ? 'videplace-light'
    : theme === 'monokai' ? 'videplace-monokai'
    : 'videplace-dark'
  );
}

// 시스템 테마 감지 (자동 전환 옵션)
const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
mediaQuery.addEventListener('change', (e) => {
  if (userPreference === 'system') {
    setTheme(e.matches ? 'dark' : 'light');
  }
});
```

### 9.3 Tailwind CSS 다크 모드 설정

```javascript
// tailwind.config.js
module.exports = {
  darkMode: ['selector', '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        canvas: 'var(--bg-canvas)',
        sidebar: 'var(--bg-sidebar)',
        editor: 'var(--bg-editor)',
        surface: 'var(--bg-surface)',
        elevated: 'var(--bg-elevated)',
      },
      textColor: {
        primary: 'var(--text-primary)',
        secondary: 'var(--text-secondary)',
        tertiary: 'var(--text-tertiary)',
        disabled: 'var(--text-disabled)',
      },
      borderColor: {
        DEFAULT: 'var(--border-default)',
        muted: 'var(--border-muted)',
        strong: 'var(--border-strong)',
        accent: 'var(--border-accent)',
      },
      boxShadow: {
        sm: 'var(--shadow-sm)',
        md: 'var(--shadow-md)',
        lg: 'var(--shadow-lg)',
        xl: 'var(--shadow-xl)',
      },
    },
  },
};
```

Tailwind에서 CSS 변수를 직접 참조하므로, `data-theme` 속성만 바꾸면 모든 유틸리티 클래스가 자동으로 해당 테마 색상을 사용한다. Tailwind의 `dark:` 접두사 대신 CSS 변수 기반 접근을 사용하는 것이 3개 이상의 테마를 지원하는 데 적합하다.

---

## 10. 접근성 (a11y)

### 10.1 키보드 내비게이션

**전역 단축키:**

| 단축키 | 동작 |
|--------|------|
| `Ctrl+B` | 사이드바 토글 |
| `Ctrl+Shift+I` | AI 채팅 패널 토글 |
| `` Ctrl+` `` | 하단 패널(터미널) 토글 |
| `Ctrl+P` | 빠른 파일 열기 (Quick Open) |
| `Ctrl+Shift+P` | 명령 팔레트 |
| `Ctrl+,` | 설정 열기 |
| `Ctrl+S` | 파일 저장 |
| `Ctrl+Shift+F` | 전체 검색 |
| `Ctrl+Tab` | 에디터 탭 전환 |
| `Ctrl+W` | 현재 탭 닫기 |
| `F5` | 미리보기 새로고침 |
| `Ctrl+Shift+G` | Git 패널 열기 |
| `Ctrl+Enter` | AI 채팅 메시지 전송 |
| `Escape` | 모달/드롭다운/컨텍스트메뉴 닫기 |

**포커스 이동:**
- `Tab`: 포커스 가능한 다음 요소로 이동
- `Shift+Tab`: 이전 요소로 이동
- `Arrow Keys`: 리스트/트리/탭 내에서 이동
- `Enter` / `Space`: 선택/활성화
- `Escape`: 현재 컨텍스트에서 벗어남

**트리 뷰 (파일 탐색기) 키보드:**
- `ArrowDown/Up`: 항목 이동
- `ArrowRight`: 폴더 펼치기 / 자식으로 이동
- `ArrowLeft`: 폴더 접기 / 부모로 이동
- `Enter`: 파일 열기 / 폴더 토글
- `Home/End`: 첫/마지막 항목으로

### 10.2 ARIA 라벨

모든 인터랙티브 요소에 적절한 ARIA 속성을 부여한다:

```html
<!-- 사이드바 내비게이션 -->
<nav aria-label="사이드바 탐색">
  <ul role="tree" aria-label="파일 탐색기">
    <li role="treeitem" aria-expanded="true" aria-level="1">
      src/
      <ul role="group">
        <li role="treeitem" aria-level="2">app.tsx</li>
      </ul>
    </li>
  </ul>
</nav>

<!-- 탭 -->
<div role="tablist" aria-label="디버그 패널 탭">
  <button role="tab" aria-selected="true" aria-controls="console-panel">Console</button>
  <button role="tab" aria-selected="false" aria-controls="network-panel">Network</button>
</div>
<div role="tabpanel" id="console-panel" aria-labelledby="console-tab">...</div>

<!-- 모달 -->
<div role="dialog" aria-modal="true" aria-labelledby="modal-title">
  <h2 id="modal-title">로그인</h2>
  ...
</div>

<!-- 토스트 -->
<div role="alert" aria-live="polite">
  출시가 완료되었습니다.
</div>

<!-- AI 채팅 -->
<div role="log" aria-label="AI 채팅 메시지" aria-live="polite">
  <div role="article" aria-label="사용자 메시지">...</div>
  <div role="article" aria-label="AI 응답">...</div>
</div>

<!-- 상태 인디케이터 -->
<span class="status-dot" role="status" aria-label="서비스 정상 운영 중"></span>

<!-- 아이콘 버튼 -->
<button aria-label="사이드바 접기">
  <svg><!-- ChevronLeft --></svg>
</button>

<!-- 프로그레스 바 -->
<div role="progressbar" aria-valuenow="65" aria-valuemin="0" aria-valuemax="100" aria-label="출시 진행률">
  65%
</div>
```

### 10.3 포커스 링

모든 포커스 가능한 요소에 명확한 포커스 표시:

```css
/* 글로벌 포커스 링 */
:focus-visible {
  outline: 2px solid var(--accent-primary);
  outline-offset: 2px;
}

/* 어두운 배경 위 (사이드바 등) */
.dark-bg :focus-visible {
  outline-color: var(--accent-primary);
  outline-offset: -2px;  /* 요소 안쪽으로 */
}

/* 입력 필드 (자체 포커스 스타일) */
input:focus-visible,
textarea:focus-visible,
select:focus-visible {
  outline: none;
  border-color: var(--border-accent);
  box-shadow: 0 0 0 3px var(--accent-primary-muted);
}

/* 마우스 클릭 시 포커스 링 숨김 (키보드에서만 표시) */
:focus:not(:focus-visible) {
  outline: none;
}
```

포커스 링 색상:
- Dark 테마: `#58a6ff` (파란색, 어두운 배경에서 잘 보임)
- Light 테마: `#0969da` (진한 파란색)
- Monokai: `#66d9ef` (시안)

### 10.4 고대비 모드

운영체제의 고대비 모드 또는 사용자 설정에 대응:

```css
@media (forced-colors: active) {
  /* 강제 색상 모드에서의 오버라이드 */
  .btn-primary {
    border: 2px solid ButtonText;
  }

  .status-dot {
    forced-color-adjust: none;  /* 상태 도트는 원래 색상 유지 */
  }

  .toast {
    border: 2px solid CanvasText;
  }
}

/* 대비 향상 옵션 (설정에서 활성화) */
[data-high-contrast="true"] {
  --text-primary: #ffffff;
  --text-secondary: #c9d1d9;
  --text-tertiary: #8b949e;
  --border-default: #484f58;
  --border-muted: #30363d;

  /* 버튼 대비 강화 */
  --accent-primary: #79c0ff;
  --accent-success: #56d364;
  --accent-danger: #ff7b72;
  --accent-warning: #e3b341;
}
```

**추가 접근성 가이드라인:**
- 모든 색상 조합은 WCAG 2.1 AA 기준 이상의 대비율을 충족한다 (일반 텍스트 4.5:1, 큰 텍스트 3:1).
- 색상만으로 정보를 전달하지 않는다. 항상 아이콘 또는 텍스트를 병행한다 (예: 에러=빨간색+X 아이콘, 성공=초록색+체크 아이콘).
- 애니메이션은 `prefers-reduced-motion: reduce` 미디어 쿼리 시 비활성화한다:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

- 이미지 및 아이콘에 적절한 `alt` 텍스트 또는 `aria-label`을 제공한다.
- AI 채팅의 실시간 응답은 `aria-live="polite"`로 스크린 리더에 전달한다.
- 모달 열림 시 포커스 트랩을 적용하고, 닫힐 때 이전 포커스 위치로 복귀한다.
