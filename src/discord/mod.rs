//! Discord 원격 제어 봇
//!
//! 환경변수:
//!   DISCORD_TOKEN       봇 토큰 (필수)
//!   DISCORD_PREFIX      명령어 접두사 (기본: "!")
//!   DISCORD_CHANNEL_ID  허용 채널 ID (빈 경우 전체 채널 허용)
//!
//! 지원 명령어:
//!   !ask <질문>         AI에게 질문
//!   !code <요청>        코드 생성 요청 (개발자 에이전트)
//!   !plan <작업>        작업 기획 (기획 에이전트)
//!   !debug <문제>       버그 분석 (디버거 에이전트)
//!   !pipeline <작업>    기획→개발→디버깅 전체 Pipeline
//!   !status             에이전트 상태
//!   !clear              현재 채널 세션 초기화
//!   !history [n]        대화 히스토리

pub mod bot;
pub mod session;

pub use bot::run_discord_bot;
