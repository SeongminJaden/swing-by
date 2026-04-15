# TODO - 배포 전 남은 작업

## 필수 (배포 전)

### 인프라 설정
- [ ] Supabase 프로젝트 생성 및 `supabase/schema.sql` 실행
- [ ] Supabase에서 Google/GitHub/Apple OAuth 프로바이더 활성화
- [ ] `~/.videplace/supabase.json`에 Supabase URL + anon key 설정
- [ ] OAuth App 등록 (GitHub Developer Settings, Google Cloud Console)
  - Callback URL: `http://localhost:39281/callback`
- [ ] Stripe 계정 설정 + API 키 입력

### 빌드 & 배포
- [ ] `npm run dist:linux` → AppImage + deb 생성 확인
- [ ] `npm run dist:mac` → dmg 생성 확인 (macOS에서 실행)
- [ ] `npm run dist:win` → NSIS installer 생성 확인 (Windows에서 실행)
- [ ] GitHub Release v0.1.0-alpha 발행 + 바이너리 업로드
- [ ] 랜딩 페이지 호스팅 (GitHub Pages 또는 Vercel)
  - `landing/index.html` 배포

### 테스트
- [ ] 실제 Google OAuth 로그인 E2E 테스트
- [ ] 실제 GitHub OAuth 로그인 E2E 테스트
- [ ] Stripe 테스트 모드 결제 플로우 E2E 테스트
- [ ] Claude API 키 연동 → AI 채팅 E2E 테스트
- [ ] 프로젝트 생성 → AI 코드 생성 → 파일 저장 E2E 테스트
- [ ] Vercel 배포 E2E 테스트

---

## 선택 (v0.2.0 이후)

### 기능 개선
- [ ] 실시간 협업 (Supabase Realtime)
- [ ] Git diff viewer (Monaco diff editor)
- [ ] 파일 검색 (Ctrl+P)
- [ ] 전체 검색 (Ctrl+Shift+F)
- [ ] 멀티 터미널 탭
- [ ] 디버그 콘솔 실제 연동 (프로젝트 dev server stdout 캡처)
- [ ] 네트워크 탭 실제 연동 (프로젝트 HTTP 요청 프록시)
- [ ] Problems 탭 ESLint 연동
- [ ] AI 코드 리뷰 기능
- [ ] AI 커밋 메시지 자동 생성

### 인프라
- [ ] CI/CD 파이프라인 (GitHub Actions)
  - 자동 빌드 + 테스트 + Release 발행
- [ ] 앱 자동 업데이트 서명 (코드 사이닝)
  - macOS: Apple Developer Certificate
  - Windows: EV Code Signing Certificate
- [ ] Sentry 에러 트래킹 연동
- [ ] 사용자 분석 (Mixpanel 또는 PostHog)

### 플랫폼
- [ ] iOS/Android 앱 (React Native 또는 Flutter)
- [ ] 웹 버전 (Electron → 브라우저 포팅)
- [ ] VS Code Extension 버전

### 비즈니스
- [ ] 마케팅 사이트 (Next.js)
- [ ] 블로그
- [ ] 문서 사이트 (Docusaurus)
- [ ] Product Hunt 런칭
- [ ] 커뮤니티 (Discord 서버)
