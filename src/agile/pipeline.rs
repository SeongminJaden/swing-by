//! 애자일 Pipeline — 전체 SDLC 에이전트 오케스트레이션
//!
//! 전체 흐름 (full 모드):
//!   ProductOwner    → User story 생성
//!   ScrumMaster     → Sprint 계획
//!   [스토리별]
//!     BusinessAnalyst → 요구사항 정제 + 비즈니스 케이스
//!     UXDesigner      → 사용자 흐름 + 와이어프레임
//!     Architect       → 기술 설계 (외부 자료 검색 포함)
//!     Developer       → 구현 (TDD)
//!     Reviewer        → 코드 리뷰
//!     QAEngineer      → 검증 + Bug report (최대 3회 루프)
//!     HackerAgent     → OWASP 보안 감사 (최대 5회 루프)
//!     TechLead        → 기술 게이트 리뷰 + ADR
//!     TechnicalWriter → 문서화
//!     DevOpsEngineer  → CI/CD + 인프라 코드
//!     SRE             → 모니터링 + 런북
//!     ReleaseManager  → 릴리즈 노트 + 배포 체크리스트
//!   → StoryStatus::Released
//!
//! 빠른 모드 (fast=true): BA/UX/DevOps/Writer/SRE/Release 스킵

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::agent::ollama::OllamaClient;
use crate::agent::node::{NodeHub, NodeMessage, MsgType};
use crate::agile::board::AgileBoard;
use crate::agile::story::{Priority, StoryStatus, UserStory, BugReport};
use crate::agile::team::AgileRole;
use crate::agile::runner::{run_agile_agent, run_agent_simple};

// ─── 체크포인트 (Sprint 중단 후 재개) ─────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
struct Checkpoint {
    done_ids: Vec<String>,
    completed: Vec<String>,
    released: Vec<String>,
    failed: Vec<String>,
    total_bugs: usize,
    security_findings: usize,
    docs_generated: usize,
}

impl Checkpoint {
    fn load(path: &str) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

const MAX_QA_RETRIES: usize = 3;

// ─── 최종 결과 ───────────────────────────────────────────────────────────────

pub struct SprintResult {
    pub sprint_id: String,
    pub completed_stories: Vec<String>,
    pub failed_stories: Vec<String>,
    pub total_bugs: usize,
    pub velocity: u32,
    pub security_findings: usize,
    pub docs_generated: usize,
    pub released_stories: Vec<String>,
}

// ─── Sprint 메인 진입점 ────────────────────────────────────────────────────

pub async fn run_agile_sprint(
    client: &OllamaClient,
    project_name: &str,
    user_request: &str,
    on_progress: impl Fn(&str) + Clone,
) -> Result<SprintResult> {
    run_agile_sprint_opts(client, project_name, user_request, false, on_progress).await
}

/// fast=true 이면 BA/UX/DevOps/Writer/SRE/Release 단계를 스킵합니다.
pub async fn run_agile_sprint_opts(
    client: &OllamaClient,
    project_name: &str,
    user_request: &str,
    fast: bool,
    on_progress: impl Fn(&str) + Clone,
) -> Result<SprintResult> {
    let board = AgileBoard::load_or_new(project_name);
    let hub = NodeHub::new();

    print_divider("애자일 스프린트 시작");
    on_progress(&format!("📌 프로젝트: {} | 요청: {}",
        project_name, crate::utils::trunc(user_request, 60)));

    // ── 1단계: ProductOwner — User story 생성 ───────────────────────────────
    print_divider("1/5 · ProductOwner — 요구사항 분석");
    on_progress("📦 요구사항 분석 및 유저 스토리 생성 중...");

    let po_system = format!(
        "모델: {}\n\n{}\n\n{}",
        client.model(),
        crate::agent::tools::tool_descriptions(),
        AgileRole::ProductOwner.system_prompt("")
    );

    let po_output = run_agent_simple(client, &po_system, user_request, 6, &on_progress).await;
    let story_ids = parse_and_create_stories(&board, &po_output);

    let story_ids = if story_ids.is_empty() {
        on_progress("⚠️ JSON 파싱 불가 — 기본 스토리 1개 생성");
        let sid = board.next_story_id();
        let mut s = UserStory::new(&sid, user_request, user_request, Priority::High, 5);
        s.add_acceptance_criteria("기능이 요구사항대로 동작");
        s.add_acceptance_criteria("빌드 및 테스트 통과");
        s.add_qa_check("기능 정상 동작 확인");
        s.add_qa_check("빌드 성공");
        s.add_qa_check("엣지 케이스 처리");
        board.add_story(s)?;
        vec![sid]
    } else {
        story_ids
    };

    on_progress(&format!("✅ {} 개 유저 스토리 생성", story_ids.len()));

    // ── 2단계: ScrumMaster — Sprint 계획 ──────────────────────────────────
    print_divider("2/5 · ScrumMaster — 스프린트 계획");
    on_progress("🏃 스프린트 계획 수립 중...");

    let sprint_id = board.create_sprint(
        &format!("{} 구현", crate::utils::trunc(user_request, 50))
    )?;
    for sid in &story_ids {
        board.add_story_to_sprint(sid, &sprint_id)?;
    }
    board.start_sprint(&sprint_id)?;
    on_progress(&format!("✅ Sprint {} 시작 — {} 개 스토리", sprint_id, story_ids.len()));

    // ── 3단계: 스토리별 개발 사이클 ──────────────────────────────────────────
    let mut completed = Vec::new();
    let mut failed_list = Vec::new();
    let mut released_list = Vec::new();
    let mut total_bugs = 0usize;
    let mut security_findings = 0usize;
    let mut docs_generated = 0usize;

    // Load checkpoint (resume from previous stop point)
    let checkpoint_path = format!(".checkpoint-{}.json", sprint_id);
    let mut checkpoint = Checkpoint::load(&checkpoint_path);
    if !checkpoint.done_ids.is_empty() {
        on_progress(&format!("♻️  체크포인트 발견 — {}개 스토리 건너뜀", checkpoint.done_ids.len()));
        total_bugs = checkpoint.total_bugs;
        security_findings = checkpoint.security_findings;
        docs_generated = checkpoint.docs_generated;
        completed = checkpoint.completed.clone();
        released_list = checkpoint.released.clone();
        failed_list = checkpoint.failed.clone();
    }

    let sprint_stories = board.get_stories_by_status(&StoryStatus::Todo);

    for story in &sprint_stories {
        let sid = story.id.clone();

        // Skip already-processed story
        if checkpoint.done_ids.contains(&sid) {
            on_progress(&format!("⏭️  [{}] 체크포인트 건너뜀", sid));
            continue;
        }

        on_progress(&format!("\n━━━ 스토리 [{}] {} ━━━", sid,
            crate::utils::trunc(&story.title, 50)));

        let story_result = run_story_pipeline(
            client, &board, &hub, &sid, fast, on_progress.clone()
        ).await;

        match story_result {
            Some((bugs, sec, docs, released)) => {
                total_bugs += bugs;
                security_findings += sec;
                docs_generated += docs;
                if released {
                    released_list.push(sid.clone());
                    on_progress(&format!("🎉 [{}] Released (버그 {}개, 보안 {}개, 문서 {}개)",
                        sid, bugs, sec, docs));
                } else {
                    completed.push(sid.clone());
                    on_progress(&format!("✅ [{}] Done (버그 {}개, 보안 {}개)", sid, bugs, sec));
                }
            }
            None => {
                failed_list.push(sid.clone());
                on_progress(&format!("❌ [{}] QA 실패 — 백로그로 이동", sid));
                let _ = board.update_story_status(&sid, StoryStatus::Backlog, "ScrumMaster");
            }
        }

        // Save checkpoint (after each story completes)
        checkpoint.done_ids.push(sid.clone());
        checkpoint.total_bugs = total_bugs;
        checkpoint.security_findings = security_findings;
        checkpoint.docs_generated = docs_generated;
        checkpoint.completed = completed.clone();
        checkpoint.released = released_list.clone();
        checkpoint.failed = failed_list.clone();
        checkpoint.save(&checkpoint_path);
    }

    // Clean up checkpoint file
    let _ = std::fs::remove_file(&checkpoint_path);

    // ── 4단계: Sprint 완료 ─────────────────────────────────────────────────
    board.complete_sprint(&sprint_id)?;

    let velocity = {
        let state_arc = board.shared_state();
        let state = state_arc.lock().unwrap();
        let sprint = state.sprints.iter().find(|s| s.id == sprint_id);
        sprint.map(|s| s.velocity(&state.stories)).unwrap_or(0)
    };

    print_divider("스프린트 완료");
    on_progress(&board.render());
    on_progress(&board.render_burndown());

    let mut all_completed = completed.clone();
    all_completed.extend(released_list.clone());

    let result = SprintResult {
        sprint_id,
        completed_stories: all_completed,
        failed_stories: failed_list,
        total_bugs,
        velocity,
        security_findings,
        docs_generated,
        released_stories: released_list,
    };

    // Save sprint report to file
    let report_path = save_sprint_report(&result, &board, project_name);
    on_progress(&format!("📄 스프린트 보고서 저장: {}", report_path));

    Ok(result)
}

// ─── 스토리 개발 Pipeline ──────────────────────────────────────────────────
/// 반환: Some((bug_count, sec_count, docs_count, released)) = 완료, None = 실패

async fn run_story_pipeline(
    client: &OllamaClient,
    board: &AgileBoard,
    hub: &NodeHub,
    story_id: &str,
    fast: bool,
    on_progress: impl Fn(&str) + Clone,
) -> Option<(usize, usize, usize, bool)> {

    // ── Pre-dev: BA + UX (fast 모드에서 스킵) ────────────────────────────────
    if !fast {
        board.update_story_status(story_id, StoryStatus::UXReview, "ScrumMaster").ok();

        let story = board.get_story(story_id)?;

        // BusinessAnalyst
        print_divider("BusinessAnalyst — 요구사항 정제");
        on_progress(&format!("📊 [{}] 비즈니스 분석 + 요구사항 정제 중...", story_id));
        let ba_output = run_agile_agent(client, AgileRole::BusinessAnalyst, &story, "", hub, &on_progress).await;
        board.update_story_field(story_id, "BusinessAnalyst", |s| {
            s.business_analysis = Some(ba_output.clone());
        }).ok();

        // UXDesigner
        print_divider("UXDesigner — 사용자 경험 설계");
        on_progress(&format!("🎨 [{}] UX 설계 + 와이어프레임 작성 중...", story_id));
        let ux_ctx = format!("## 비즈니스 분석\n{}", crate::utils::trunc(&ba_output, 1000));
        let story = board.get_story(story_id)?;
        let ux_output = run_agile_agent(client, AgileRole::UXDesigner, &story, &ux_ctx, hub, &on_progress).await;
        board.update_story_field(story_id, "UXDesigner", |s| {
            s.ux_design = Some(ux_output.clone());
        }).ok();

        let _ = hub.send(NodeMessage {
            from: "UXDesigner".into(), to: "Architect".into(),
            msg_type: MsgType::Result, content: ux_output,
            metadata: Default::default(),
        }).await;
    }

    board.update_story_status(story_id, StoryStatus::InProgress, "UXDesigner").ok();

    // ── Architect ────────────────────────────────────────────────────────────
    print_divider("Architect — 기술 설계");
    on_progress(&format!("🏛️  [{}] 기술 설계 + 최신 아키텍처 검색...", story_id));

    let story = board.get_story(story_id)?;
    let arch_output = run_agile_agent(client, AgileRole::Architect, &story, "", hub, &on_progress).await;
    board.update_story_field(story_id, "Architect", |s| {
        s.plan = Some(arch_output.clone());
    }).ok();
    let _ = hub.send(NodeMessage { from: "Architect".into(), to: "Developer".into(),
        msg_type: MsgType::Result, content: arch_output.clone(), metadata: Default::default() }).await;

    // ── Developer + Reviewer + QA (재시도 루프) ───────────────────────────────
    let mut total_bugs = 0usize;

    for attempt in 0..MAX_QA_RETRIES {
        // Developer
        print_divider(&format!("Developer — 구현 (시도 {})", attempt + 1));
        on_progress(&format!("💻 [{}] 구현 중 ({}번째)...", story_id, attempt + 1));

        let story = board.get_story(story_id)?;
        let dev_ctx = build_dev_context(&story, attempt);
        let dev_output = run_agile_agent(client, AgileRole::Developer, &story, &dev_ctx, hub, &on_progress).await;

        board.update_story_field(story_id, "Developer", |s| {
            s.implementation = Some(dev_output.clone());
        }).ok();
        let _ = hub.send(NodeMessage { from: "Developer".into(), to: "Reviewer".into(),
            msg_type: MsgType::Result, content: dev_output.clone(), metadata: Default::default() }).await;

        // Reviewer
        print_divider("Reviewer — 코드 리뷰");
        on_progress(&format!("👁️  [{}] 코드 리뷰 중...", story_id));
        board.update_story_status(story_id, StoryStatus::Review, "Developer").ok();

        let story = board.get_story(story_id)?;
        let rev_ctx = format!("## 구현 내용\n{}", crate::utils::trunc(&dev_output, 2000));
        let rev_output = run_agile_agent(client, AgileRole::Reviewer, &story, &rev_ctx, hub, &on_progress).await;

        board.update_story_field(story_id, "Reviewer", |s| {
            s.review_feedback = Some(rev_output.clone());
        }).ok();

        let approved = check_approved(&rev_output);
        let _ = hub.send(NodeMessage { from: "Reviewer".into(), to: "Developer".into(),
            msg_type: MsgType::Result,
            content: format!("리뷰 {}: {}", if approved { "승인" } else { "반려" }, rev_output),
            metadata: Default::default() }).await;

        if !approved && attempt < MAX_QA_RETRIES - 1 {
            on_progress(&format!("❌ [{}] 리뷰 반려 → 재작업", story_id));
            board.update_story_status(story_id, StoryStatus::InProgress, "Reviewer").ok();
            continue;
        }

        // QA Engineer
        print_divider("QA — 검증");
        on_progress(&format!("🔬 [{}] QA 검증 중...", story_id));
        board.update_story_status(story_id, StoryStatus::QA, "Reviewer").ok();

        let _ = hub.send(NodeMessage { from: "Reviewer".into(), to: "QAEngineer".into(),
            msg_type: MsgType::Task, content: format!("[{}] QA 시작", story_id),
            metadata: Default::default() }).await;

        let story = board.get_story(story_id)?;
        let qa_ctx = build_qa_context(&story);
        let qa_output = run_agile_agent(client, AgileRole::QAEngineer, &story, &qa_ctx, hub, &on_progress).await;

        board.update_story_field(story_id, "QAEngineer", |s| {
            s.qa_report = Some(qa_output.clone());
        }).ok();

        let (qa_ok, bugs) = parse_qa_result(&qa_output, story_id, board);
        total_bugs += bugs.len();

        for bug in bugs {
            let _ = board.add_bug(bug, "QAEngineer");
        }

        // QA 체크리스트 업데이트
        board.update_story_field(story_id, "QAEngineer", |s| {
            for check in &mut s.qa_checks {
                check.passed = Some(qa_ok);
            }
        }).ok();

        let _ = hub.send(NodeMessage { from: "QAEngineer".into(), to: "Developer".into(),
            msg_type: MsgType::Status,
            content: format!("[{}] QA {}", story_id, if qa_ok { "통과 ✅" } else { "실패 ❌" }),
            metadata: Default::default() }).await;

        if qa_ok {
            // ── HackerAgent 보안 감사 ─────────────────────────────────────────
            print_divider("HackerAgent — 보안 감사");
            board.update_story_status(story_id, StoryStatus::SecurityReview, "QAEngineer").ok();

            let sec_result = crate::agile::hacker::run_security_fix_loop(
                client, board, hub, story_id, ".", on_progress.clone()
            ).await;

            let sec_count = sec_result.final_report.vulnerabilities.len();
            on_progress(&sec_result.final_report.render());

            if sec_result.approved {
                on_progress(&format!("✅ [{}] 보안 감사 통과 — 취약점 {}개", story_id, sec_count));
            } else {
                on_progress(&format!("⚠️  [{}] 보안 감사 미수정 {}개 존재",
                    story_id, sec_result.final_report.unfixed_count()));
            }

            // fast 모드: Done 처리 후 조기 반환
            if fast {
                board.update_story_status(story_id, StoryStatus::Done, "HackerAgent").ok();
                return Some((total_bugs, sec_count, 0, false));
            }

            // ── TechLead 게이트 리뷰 ─────────────────────────────────────────
            print_divider("TechLead — 기술 게이트 리뷰");
            board.update_story_status(story_id, StoryStatus::TechLeadReview, "HackerAgent").ok();
            on_progress(&format!("🎯 [{}] TechLead 게이트 리뷰 중...", story_id));

            let story = board.get_story(story_id)?;
            let tl_ctx = format!(
                "## 보안 감사 결과\n취약점: {}개 (미수정: {}개)\n\n\
                 ## 구현 내용\n{}\n\n\
                 ## QA 결과\n{}",
                sec_count, sec_result.final_report.unfixed_count(),
                crate::utils::trunc(story.implementation.as_deref().unwrap_or("없음"), 1500),
                crate::utils::trunc(story.qa_report.as_deref().unwrap_or("없음"), 500),
            );
            let tl_output = run_agile_agent(client, AgileRole::TechLead, &story, &tl_ctx, hub, &on_progress).await;
            let tl_approved = check_approved(&tl_output);

            board.update_story_field(story_id, "TechLead", |s| {
                s.tech_lead_review = Some(tl_output.clone());
            }).ok();

            if !tl_approved {
                on_progress(&format!("❌ [{}] TechLead 미승인 — 개발로 복귀", story_id));
                board.update_story_status(story_id, StoryStatus::InProgress, "TechLead").ok();
                // 한 번 더 시도
                continue;
            }
            on_progress(&format!("✅ [{}] TechLead 승인", story_id));

            // ── TechnicalWriter — 문서화 ──────────────────────────────────────
            print_divider("TechnicalWriter — 문서화");
            board.update_story_status(story_id, StoryStatus::Documentation, "TechLead").ok();
            on_progress(&format!("📝 [{}] 문서화 중...", story_id));

            let story = board.get_story(story_id)?;
            let tw_ctx = format!(
                "## TechLead ADR\n{}\n\n## 구현\n{}",
                crate::utils::trunc(&tl_output, 800),
                crate::utils::trunc(story.implementation.as_deref().unwrap_or(""), 1500),
            );
            let tw_output = run_agile_agent(client, AgileRole::TechnicalWriter, &story, &tw_ctx, hub, &on_progress).await;
            let docs_count = count_docs_written(&tw_output);

            board.update_story_field(story_id, "TechnicalWriter", |s| {
                s.docs = Some(tw_output.clone());
            }).ok();

            // ── DevOpsEngineer — CI/CD ────────────────────────────────────────
            print_divider("DevOpsEngineer — CI/CD 설정");
            board.update_story_status(story_id, StoryStatus::DevOpsSetup, "TechnicalWriter").ok();
            on_progress(&format!("🚀 [{}] CI/CD + 인프라 코드 생성 중...", story_id));

            let story = board.get_story(story_id)?;
            let devops_ctx = format!(
                "## 아키텍처 설계\n{}\n\n## 문서 정보\n{}",
                crate::utils::trunc(story.plan.as_deref().unwrap_or(""), 1000),
                crate::utils::trunc(&tw_output, 500),
            );
            let devops_output = run_agile_agent(client, AgileRole::DevOpsEngineer, &story, &devops_ctx, hub, &on_progress).await;
            board.update_story_field(story_id, "DevOpsEngineer", |s| {
                s.devops_artifacts = Some(devops_output.clone());
            }).ok();

            // ── SRE — 모니터링 + 런북 ────────────────────────────────────────
            print_divider("SRE — 모니터링 + 런북");
            board.update_story_status(story_id, StoryStatus::SRESetup, "DevOpsEngineer").ok();
            on_progress(&format!("📡 [{}] SLO + 알람 + 런북 생성 중...", story_id));

            let story = board.get_story(story_id)?;
            let sre_ctx = format!(
                "## 서비스 정보\n{}\n\n## DevOps 설정\n{}",
                crate::utils::trunc(&story.title, 200),
                crate::utils::trunc(&devops_output, 800),
            );
            let sre_output = run_agile_agent(client, AgileRole::SRE, &story, &sre_ctx, hub, &on_progress).await;
            board.update_story_field(story_id, "SRE", |s| {
                s.sre_config = Some(sre_output.clone());
            }).ok();

            // ── ReleaseManager — 릴리즈 준비 ─────────────────────────────────
            print_divider("ReleaseManager — 릴리즈 노트");
            board.update_story_status(story_id, StoryStatus::ReleasePrep, "SRE").ok();
            on_progress(&format!("🎁 [{}] 릴리즈 노트 + 배포 체크리스트 생성 중...", story_id));

            let story = board.get_story(story_id)?;
            let rm_ctx = format!(
                "## 스토리 정보\n{}\n\n## TechLead ADR\n{}\n\n## SRE SLO\n{}",
                story.summary(),
                crate::utils::trunc(&tl_output, 600),
                crate::utils::trunc(&sre_output, 400),
            );
            let rm_output = run_agile_agent(client, AgileRole::ReleaseManager, &story, &rm_ctx, hub, &on_progress).await;
            board.update_story_field(story_id, "ReleaseManager", |s| {
                s.release_notes = Some(rm_output.clone());
            }).ok();

            // ── Released ──────────────────────────────────────────────────────
            board.update_story_status(story_id, StoryStatus::Released, "ReleaseManager").ok();
            on_progress(&format!("🎉 [{}] Released! 문서 {}개, 보안 {}개",
                story_id, docs_count, sec_count));

            return Some((total_bugs, sec_count, docs_count, true));
        } else {
            on_progress(&format!("❌ [{}] QA 실패 (버그 {}개)", story_id, total_bugs));
            if attempt < MAX_QA_RETRIES - 1 {
                board.update_story_status(story_id, StoryStatus::InProgress, "QAEngineer").ok();
            } else {
                board.update_story_status(story_id, StoryStatus::QAFailed, "QAEngineer").ok();
            }
        }
    }

    None  // 모든 재시도 실패
}

// ─── Helpers ────────────────────────────────────────────────────────────────────

/// Sprint 완료 후 마크다운 보고서를 파일로 저장
fn save_sprint_report(result: &SprintResult, board: &AgileBoard, project_name: &str) -> String {
    let now = chrono_now();
    let filename = format!("sprint-report-{}-{}.md", result.sprint_id, now);

    let state = board.shared_state();
    let state_guard = state.lock().unwrap();

    let mut lines = vec![
        format!("# 스프린트 보고서 — {} ({})", result.sprint_id, project_name),
        format!("생성일시: {}\n", now),
        format!("## 요약"),
        format!("- 완료 스토리: {} 개", result.completed_stories.len()),
        format!("- 릴리즈 스토리: {} 개", result.released_stories.len()),
        format!("- 실패 스토리: {} 개", result.failed_stories.len()),
        format!("- 총 버그: {} 개", result.total_bugs),
        format!("- 보안 발견: {} 개", result.security_findings),
        format!("- 생성 문서: {} 개", result.docs_generated),
        format!("- 벨로시티: {} pts\n", result.velocity),
    ];

    // Per-story detailed content
    lines.push("## 스토리 상세".to_string());
    for sid in result.completed_stories.iter().chain(result.released_stories.iter()) {
        if let Some(story) = state_guard.stories.get(sid) {
            lines.push(format!("\n### [{}] {} ({}pts)", story.id, story.title, story.story_points));
            lines.push(format!("상태: {:?} | 우선순위: {:?}", story.status, story.priority));

            if let Some(ba) = &story.business_analysis {
                lines.push(format!("\n#### 비즈니스 분석\n{}", crate::utils::trunc(ba, 500)));
            }
            if let Some(ux) = &story.ux_design {
                lines.push(format!("\n#### UX 설계\n{}", crate::utils::trunc(ux, 500)));
            }
            if let Some(plan) = &story.plan {
                lines.push(format!("\n#### 아키텍처 설계\n{}", crate::utils::trunc(plan, 800)));
            }
            if let Some(impl_) = &story.implementation {
                lines.push(format!("\n#### 구현\n```\n{}\n```", crate::utils::trunc(impl_, 1500)));
            }
            if let Some(docs) = &story.docs {
                lines.push(format!("\n#### 문서\n{}", crate::utils::trunc(docs, 600)));
            }
            if let Some(devops) = &story.devops_artifacts {
                lines.push(format!("\n#### DevOps/CI-CD\n{}", crate::utils::trunc(devops, 400)));
            }
            if let Some(sre) = &story.sre_config {
                lines.push(format!("\n#### SRE 설정\n{}", crate::utils::trunc(sre, 300)));
            }
            if let Some(rn) = &story.release_notes {
                lines.push(format!("\n#### 릴리즈 노트\n{}", crate::utils::trunc(rn, 400)));
            }
            if !story.bug_reports.is_empty() {
                lines.push("\n#### 버그 리포트".to_string());
                for bug in &story.bug_reports {
                    lines.push(format!("- [{}] {} ({:?}) — {}",
                        bug.id, bug.title, bug.severity,
                        if bug.fixed { "수정됨" } else { "미수정" }));
                }
            }
        }
    }

    // Failed stories
    if !result.failed_stories.is_empty() {
        lines.push("\n## 실패 스토리".to_string());
        for sid in &result.failed_stories {
            if let Some(story) = state_guard.stories.get(sid) {
                lines.push(format!("- [{}] {} — QA 실패", story.id, story.title));
            }
        }
    }

    let content = lines.join("\n");
    drop(state_guard);

    match std::fs::write(&filename, &content) {
        Ok(_) => filename,
        Err(e) => format!("(저장 실패: {})", e),
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // YYYYMMDD-HHMMSS 형식
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400 + 719468;  // Unix epoch → civil date
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp = (5*doy + 2) / 153;
    let d = doy - (153*mp + 2)/5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}{:02}{:02}-{:02}{:02}{:02}", y, m, d, hour, min, sec)
}

fn build_dev_context(story: &UserStory, attempt: usize) -> String {
    let mut parts = Vec::new();
    if let Some(plan) = &story.plan {
        parts.push(format!("## 아키텍처 설계\n{}", crate::utils::trunc(plan, 1500)));
    }
    if attempt > 0 {
        if let Some(qa) = &story.qa_report {
            parts.push(format!("## QA 피드백 (수정 필요)\n{}", crate::utils::trunc(qa, 800)));
        }
        if let Some(rev) = &story.review_feedback {
            parts.push(format!("## 리뷰 피드백\n{}", crate::utils::trunc(rev, 600)));
        }
        if !story.bug_reports.is_empty() {
            let bugs: Vec<String> = story.bug_reports.iter()
                .filter(|b| !b.fixed)
                .map(|b| format!("- [{}] {} ({})", b.id, b.title, b.severity))
                .collect();
            parts.push(format!("## 미해결 버그\n{}", bugs.join("\n")));
        }
    }
    parts.join("\n\n")
}

fn build_qa_context(story: &UserStory) -> String {
    let mut parts = Vec::new();
    if let Some(impl_) = &story.implementation {
        parts.push(format!("## 구현 내용\n{}", crate::utils::trunc(impl_, 2000)));
    }
    let ac = story.acceptance_criteria.iter()
        .enumerate()
        .map(|(i, c)| format!("  {}. {}", i+1, c))
        .collect::<Vec<_>>().join("\n");
    parts.push(format!("## 수락 기준\n{}", ac));

    let qa_list = story.qa_checks.iter()
        .map(|c| format!("  - {}", c.description))
        .collect::<Vec<_>>().join("\n");
    parts.push(format!("## QA 체크리스트\n{}", qa_list));
    parts.join("\n\n")
}

fn parse_and_create_stories(board: &AgileBoard, text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let json = extract_json(text);

    let items: Vec<serde_json::Value> = match json {
        Some(serde_json::Value::Array(arr)) => arr,
        Some(obj @ serde_json::Value::Object(_)) => vec![obj],
        _ => return ids,
    };

    for item in &items {
        let title = match item["title"].as_str() { Some(t) => t, None => continue };
        let description = item["description"].as_str().unwrap_or(title);
        let priority = parse_priority(item["priority"].as_str().unwrap_or("Medium"));
        let points = item["story_points"].as_u64().unwrap_or(3).min(13) as u8;

        let sid = board.next_story_id();
        let mut story = UserStory::new(&sid, title, description, priority, points);

        for ac in item["acceptance_criteria"].as_array().unwrap_or(&vec![]) {
            if let Some(s) = ac.as_str() { story.add_acceptance_criteria(s); }
        }
        for qc in item["qa_checks"].as_array().unwrap_or(&vec![]) {
            if let Some(s) = qc.as_str() { story.add_qa_check(s); }
        }
        if story.qa_checks.is_empty() {
            story.add_qa_check("빌드 성공");
            story.add_qa_check("기능 동작 확인");
            story.add_qa_check("엣지 케이스 처리");
        }

        if board.add_story(story).is_ok() { ids.push(sid); }
    }
    ids
}

fn parse_qa_result(text: &str, story_id: &str, board: &AgileBoard) -> (bool, Vec<BugReport>) {
    let mut bugs = Vec::new();
    let passed = if let Some(v) = extract_json(text) {
        let overall = v["overall"].as_str().unwrap_or("FAIL");
        let ok = overall.to_uppercase() == "PASS";
        if let Some(arr) = v["bugs"].as_array() {
            for bv in arr {
                let title = bv["title"].as_str().unwrap_or("Bug");
                let sev = parse_priority(bv["severity"].as_str().unwrap_or("Medium"));
                let bug_id = board.next_bug_id();
                let mut bug = BugReport::new(&bug_id, story_id, title, sev);
                bug.description = bv["description"].as_str().unwrap_or("").to_string();
                bug.expected = bv["expected"].as_str().unwrap_or("").to_string();
                bug.actual = bv["actual"].as_str().unwrap_or("").to_string();
                if let Some(steps) = bv["steps"].as_array() {
                    bug.steps_to_reproduce = steps.iter()
                        .filter_map(|s| s.as_str().map(|s| s.to_string())).collect();
                }
                bugs.push(bug);
            }
        }
        ok
    } else {
        let u = text.to_uppercase();
        !u.contains("FAIL") && (u.contains("PASS") || u.contains("통과"))
    };
    (passed && bugs.is_empty(), bugs)
}

fn count_docs_written(text: &str) -> usize {
    // write_file 호출 횟수 또는 파일 확장자 언급 기반으로 문서 수 추정
    let file_exts = [".md", ".rst", ".adoc", ".yaml", ".yml", ".json", ".toml"];
    let tool_calls = text.matches("write_file").count();
    if tool_calls > 0 { return tool_calls; }
    file_exts.iter().map(|ext| text.matches(ext).count()).sum::<usize>().max(1)
}

fn check_approved(text: &str) -> bool {
    if let Some(v) = extract_json(text) {
        return v["approved"].as_bool().unwrap_or(false);
    }
    let u = text.to_uppercase();
    u.contains("APPROVED") || u.contains("승인")
}

fn extract_json(text: &str) -> Option<serde_json::Value> {
    let candidate = if let Some(s) = text.find("```json") {
        let after = &text[s + 7..];
        if let Some(e) = after.find("```") { &after[..e] } else { after }
    } else if let Some(s) = text.find('{') {
        if let Some(e) = text.rfind('}') { &text[s..=e] } else { return None }
    } else {
        return None;
    };
    serde_json::from_str(candidate.trim()).ok()
}

fn parse_priority(s: &str) -> Priority {
    match s.to_lowercase().as_str() {
        "critical" => Priority::Critical,
        "high"     => Priority::High,
        "low"      => Priority::Low,
        _          => Priority::Medium,
    }
}

fn print_divider(title: &str) {
    let pad = 48usize.saturating_sub(title.len()) / 2;
    let spaces = " ".repeat(pad);
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║{}{}{}║", spaces, title, spaces);
    println!("╚══════════════════════════════════════════════════╝");
}
