//! Agile Board — Kanban + Scrum board
//!
//! Handles story status transitions, sprint management, and board rendering.
//! Board state is persisted to file.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::story::{BugReport, StoryStatus, UserStory};

const BOARD_FILE: &str = ".agile_board.json";

// ─── Sprint ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: String,
    pub number: u32,
    pub goal: String,
    pub story_ids: Vec<String>,
    pub started: bool,
    pub completed: bool,
    pub created_at: u64,
}

impl Sprint {
    pub fn new(number: u32, goal: &str) -> Self {
        Self {
            id: format!("S{}", number),
            number,
            goal: goal.to_string(),
            story_ids: Vec::new(),
            started: false,
            completed: false,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    pub fn velocity(&self, stories: &HashMap<String, UserStory>) -> u32 {
        self.story_ids.iter()
            .filter_map(|id| stories.get(id))
            .filter(|s| matches!(s.status, StoryStatus::Done | StoryStatus::Released))
            .map(|s| s.story_points as u32)
            .sum()
    }

    pub fn total_points(&self, stories: &HashMap<String, UserStory>) -> u32 {
        self.story_ids.iter()
            .filter_map(|id| stories.get(id))
            .map(|s| s.story_points as u32)
            .sum()
    }
}

// ─── Activity log ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub timestamp: u64,
    pub agent: String,
    pub action: String,
    pub story_id: Option<String>,
    pub detail: String,
}

impl ActivityLog {
    pub fn new(agent: &str, action: &str, story_id: Option<&str>, detail: &str) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            agent: agent.to_string(),
            action: action.to_string(),
            story_id: story_id.map(|s| s.to_string()),
            detail: detail.to_string(),
        }
    }

    pub fn format(&self) -> String {
        let story = self.story_id.as_deref()
            .map(|id| format!("[{}] ", id))
            .unwrap_or_default();
        format!("[{}] {}{}: {}", self.agent, story, self.action, self.detail)
    }
}

// ─── 보드 상태 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardState {
    pub project_name: String,
    pub stories: HashMap<String, UserStory>,
    pub sprints: Vec<Sprint>,
    pub current_sprint_id: Option<String>,
    pub activity_log: Vec<ActivityLog>,
    pub bug_counter: usize,
    pub story_counter: usize,
}

impl BoardState {
    pub fn new(project_name: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            stories: HashMap::new(),
            sprints: Vec::new(),
            current_sprint_id: None,
            activity_log: Vec::new(),
            bug_counter: 0,
            story_counter: 0,
        }
    }

    pub fn next_story_id(&mut self) -> String {
        self.story_counter += 1;
        format!("US-{}", self.story_counter)
    }

    pub fn next_bug_id(&mut self) -> String {
        self.bug_counter += 1;
        format!("BUG-{}", self.bug_counter)
    }
}

// ─── 애자일 보드 ─────────────────────────────────────────────────────────────

pub struct AgileBoard {
    state: Arc<Mutex<BoardState>>,
    path: String,
}

impl AgileBoard {
    /// 새 보드 생성
    pub fn new(project_name: &str) -> Self {
        Self {
            state: Arc::new(Mutex::new(BoardState::new(project_name))),
            path: BOARD_FILE.to_string(),
        }
    }

    /// 파일에서 보드 로드 (없으면 새로 생성)
    pub fn load_or_new(project_name: &str) -> Self {
        let state = if let Ok(content) = std::fs::read_to_string(BOARD_FILE) {
            serde_json::from_str::<BoardState>(&content)
                .unwrap_or_else(|_| BoardState::new(project_name))
        } else {
            BoardState::new(project_name)
        };
        Self {
            state: Arc::new(Mutex::new(state)),
            path: BOARD_FILE.to_string(),
        }
    }

    /// 상태를 Arc로 공유 (에이전트 간 공유)
    pub fn shared_state(&self) -> Arc<Mutex<BoardState>> {
        self.state.clone()
    }

    // ─── 스토리 관리 ───────────────────────────────────────────────────────────

    pub fn add_story(&self, story: UserStory) -> Result<String> {
        let id = story.id.clone();
        let mut state = self.state.lock().unwrap();
        state.activity_log.push(ActivityLog::new(
            "System", "ADD_STORY", Some(&id),
            &format!("{} ({}pts, {})", story.title, story.story_points, story.priority),
        ));
        state.stories.insert(id.clone(), story);
        drop(state);
        self.save()?;
        Ok(id)
    }

    pub fn update_story_status(
        &self,
        story_id: &str,
        new_status: StoryStatus,
        agent: &str,
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let story = state.stories.get_mut(story_id)
            .ok_or_else(|| anyhow::anyhow!("스토리 없음: {}", story_id))?;
        let old = story.status.clone();
        story.status = new_status.clone();
        story.touch();
        state.activity_log.push(ActivityLog::new(
            agent, "STATUS_CHANGE", Some(story_id),
            &format!("{:?} → {:?}", old, new_status),
        ));
        drop(state);
        self.save()
    }

    pub fn update_story_field(
        &self,
        story_id: &str,
        agent: &str,
        update: impl FnOnce(&mut UserStory),
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let story = state.stories.get_mut(story_id)
            .ok_or_else(|| anyhow::anyhow!("스토리 없음: {}", story_id))?;
        update(story);
        story.touch();
        state.activity_log.push(ActivityLog::new(agent, "UPDATE", Some(story_id), "필드 업데이트"));
        drop(state);
        self.save()
    }

    pub fn add_bug(&self, bug: BugReport, agent: &str) -> Result<String> {
        let bug_id = bug.id.clone();
        let story_id = bug.story_id.clone();
        let mut state = self.state.lock().unwrap();
        // 스토리에 버그 추가
        if let Some(story) = state.stories.get_mut(&story_id) {
            story.bug_reports.push(bug.clone());
            story.touch();
        }
        state.activity_log.push(ActivityLog::new(
            agent, "BUG_REPORT", Some(&story_id),
            &format!("[{}] {} ({})", bug_id, bug.title, bug.severity),
        ));
        drop(state);
        self.save()?;
        Ok(bug_id)
    }

    // ─── Sprint 관리 ─────────────────────────────────────────────────────────

    pub fn create_sprint(&self, goal: &str) -> Result<String> {
        let mut state = self.state.lock().unwrap();
        let number = state.sprints.len() as u32 + 1;
        let sprint = Sprint::new(number, goal);
        let sprint_id = sprint.id.clone();
        state.sprints.push(sprint);
        state.activity_log.push(ActivityLog::new(
            "ScrumMaster", "SPRINT_CREATE",
            None, &format!("Sprint {} — {}", number, goal),
        ));
        drop(state);
        self.save()?;
        Ok(sprint_id)
    }

    pub fn add_story_to_sprint(&self, story_id: &str, sprint_id: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let sprint = state.sprints.iter_mut()
            .find(|s| s.id == sprint_id)
            .ok_or_else(|| anyhow::anyhow!("스프린트 없음: {}", sprint_id))?;
        if !sprint.story_ids.contains(&story_id.to_string()) {
            sprint.story_ids.push(story_id.to_string());
        }
        if let Some(story) = state.stories.get_mut(story_id) {
            story.sprint_id = Some(sprint_id.to_string());
            story.status = StoryStatus::Todo;
            story.touch();
        }
        drop(state);
        self.save()
    }

    pub fn start_sprint(&self, sprint_id: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let sprint = state.sprints.iter_mut()
            .find(|s| s.id == sprint_id)
            .ok_or_else(|| anyhow::anyhow!("스프린트 없음: {}", sprint_id))?;
        sprint.started = true;
        state.current_sprint_id = Some(sprint_id.to_string());
        state.activity_log.push(ActivityLog::new(
            "ScrumMaster", "SPRINT_START", None,
            &format!("Sprint {} 시작", sprint_id),
        ));
        drop(state);
        self.save()
    }

    pub fn complete_sprint(&self, sprint_id: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let sprint = state.sprints.iter_mut()
            .find(|s| s.id == sprint_id)
            .ok_or_else(|| anyhow::anyhow!("스프린트 없음: {}", sprint_id))?;
        sprint.completed = true;
        drop(state);
        self.save()
    }

    // ─── 보드 조회 ─────────────────────────────────────────────────────────────

    pub fn get_stories_by_status(&self, status: &StoryStatus) -> Vec<UserStory> {
        let state = self.state.lock().unwrap();
        state.stories.values()
            .filter(|s| std::mem::discriminant(&s.status) == std::mem::discriminant(status))
            .cloned()
            .collect()
    }

    pub fn get_current_sprint(&self) -> Option<Sprint> {
        let state = self.state.lock().unwrap();
        let sprint_id = state.current_sprint_id.clone()?;
        state.sprints.iter().find(|s| s.id == sprint_id).cloned()
    }

    pub fn get_story(&self, story_id: &str) -> Option<UserStory> {
        let state = self.state.lock().unwrap();
        state.stories.get(story_id).cloned()
    }

    pub fn next_story_id(&self) -> String {
        let mut state = self.state.lock().unwrap();
        state.next_story_id()
    }

    pub fn next_bug_id(&self) -> String {
        let mut state = self.state.lock().unwrap();
        state.next_bug_id()
    }

    // ─── 보드 렌더링 ───────────────────────────────────────────────────────────

    /// 전체 보드를 터미널 텍스트로 렌더링
    pub fn render(&self) -> String {
        let state = self.state.lock().unwrap();
        let mut out = Vec::new();

        out.push(format!(
            "\n╔══════════════════════════════════════════════════════╗\n\
             ║  📊 Agile Board — {}   \n\
             ╚══════════════════════════════════════════════════════╝",
            crate::utils::trunc(&state.project_name, 30)
        ));

        // 현재 Sprint 정보
        if let Some(sprint_id) = &state.current_sprint_id {
            if let Some(sprint) = state.sprints.iter().find(|s| &s.id == sprint_id) {
                let vel = sprint.velocity(&state.stories);
                let total = sprint.total_points(&state.stories);
                out.push(format!(
                    "\n🏃 Sprint {} — {}\n   진행: {}pts / {}pts ({}%)",
                    sprint.number, sprint.goal,
                    vel, total,
                    if total > 0 { vel * 100 / total } else { 0 }
                ));
            }
        }

        // Display stories by column
        let columns = [
            StoryStatus::Backlog,
            StoryStatus::Todo,
            StoryStatus::UXReview,
            StoryStatus::InProgress,
            StoryStatus::Review,
            StoryStatus::QA,
            StoryStatus::QAFailed,
            StoryStatus::SecurityReview,
            StoryStatus::TechLeadReview,
            StoryStatus::Documentation,
            StoryStatus::DevOpsSetup,
            StoryStatus::SRESetup,
            StoryStatus::ReleasePrep,
            StoryStatus::Done,
            StoryStatus::Released,
            StoryStatus::Blocked(String::new()),
        ];

        for col_status in &columns {
            let mut col_stories: Vec<&UserStory> = state.stories.values()
                .filter(|s| std::mem::discriminant(&s.status) == std::mem::discriminant(col_status))
                .collect();
            col_stories.sort_by(|a, b| a.priority.cmp(&b.priority));

            if col_stories.is_empty() { continue; }

            out.push(format!("\n── {} ({}) ──", col_status.column_name(), col_stories.len()));
            for story in col_stories {
                let (done, total) = story.task_progress();
                let bugs = story.bug_reports.iter().filter(|b| !b.fixed).count();
                let bug_str = if bugs > 0 { format!(" 🐛{}", bugs) } else { String::new() };
                let assignee = story.assigned_to.as_deref()
                    .map(|a| format!(" @{}", a))
                    .unwrap_or_default();
                out.push(format!(
                    "  {} [{}] {} ({} pts){}{} tasks:{}/{}",
                    story.priority.icon(),
                    story.id,
                    crate::utils::trunc(&story.title, 40),
                    story.story_points,
                    assignee,
                    bug_str,
                    done, total,
                ));
            }
        }

        // Recent activity
        out.push("\n── 최근 활동 ──".to_string());
        for log in state.activity_log.iter().rev().take(5) {
            out.push(format!("  {}", log.format()));
        }

        out.push(String::new());
        out.join("\n")
    }

    /// Sprint 번다운 차트 (ASCII)
    pub fn render_burndown(&self) -> String {
        let state = self.state.lock().unwrap();
        let sprint_id = match &state.current_sprint_id {
            Some(id) => id.clone(),
            None => return "진행 중인 스프린트 없음".to_string(),
        };
        let sprint = match state.sprints.iter().find(|s| s.id == sprint_id) {
            Some(s) => s,
            None => return "스프린트 데이터 없음".to_string(),
        };

        let total = sprint.total_points(&state.stories);
        let done = sprint.velocity(&state.stories);
        let remaining = total.saturating_sub(done);
        let pct = if total > 0 { done * 100 / total } else { 0 };

        let bar_width = 30usize;
        let filled = (pct as usize * bar_width) / 100;
        let bar: String = (0..bar_width)
            .map(|i| if i < filled { '▓' } else { '░' })
            .collect();

        format!(
            "\n📉 Burndown — Sprint {}\n\
             완료: {}pts / {}pts ({}%)\n\
             [{}] \n\
             남은 포인트: {}pts",
            sprint.number, done, total, pct, bar, remaining
        )
    }

    /// Activity log 마지막 N개
    pub fn recent_activity(&self, n: usize) -> Vec<ActivityLog> {
        let state = self.state.lock().unwrap();
        state.activity_log.iter().rev().take(n).cloned().collect()
    }

    // ─── 영속화 ────────────────────────────────────────────────────────────────

    pub fn save(&self) -> Result<()> {
        let state = self.state.lock().unwrap();
        let json = serde_json::to_string_pretty(&*state)
            .context("보드 직렬화 실패")?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("보드 저장 실패: {}", self.path))?;
        Ok(())
    }

    pub fn load_from(&self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("보드 파일 읽기 실패: {}", path))?;
        let new_state: BoardState = serde_json::from_str(&content)
            .context("보드 파싱 실패")?;
        let mut state = self.state.lock().unwrap();
        *state = new_state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agile::story::{Priority, StoryStatus, UserStory};
    use tempfile::TempDir;

    fn temp_board() -> (AgileBoard, TempDir) {
        let dir = TempDir::new().unwrap();
        let board_path = dir.path().join(".agile_board.json");
        let board = AgileBoard {
            state: Arc::new(Mutex::new(BoardState::new("test-project"))),
            path: board_path.to_string_lossy().into_owned(),
        };
        (board, dir)
    }

    fn make_story(id: &str) -> UserStory {
        let mut s = UserStory::new(id, "테스트 스토리", "desc", Priority::Medium, 3);
        s.add_qa_check("빌드 성공");
        s
    }

    #[test]
    fn add_and_get_story() {
        let (board, _dir) = temp_board();
        let story = make_story("US-1");
        board.add_story(story).unwrap();
        let got = board.get_story("US-1").unwrap();
        assert_eq!(got.title, "테스트 스토리");
    }

    #[test]
    fn get_story_missing_returns_none() {
        let (board, _dir) = temp_board();
        assert!(board.get_story("US-999").is_none());
    }

    #[test]
    fn update_story_status_transitions() {
        let (board, _dir) = temp_board();
        board.add_story(make_story("US-1")).unwrap();
        board.update_story_status("US-1", StoryStatus::InProgress, "Dev").unwrap();
        let s = board.get_story("US-1").unwrap();
        assert!(matches!(s.status, StoryStatus::InProgress));
    }

    #[test]
    fn update_story_status_missing_returns_err() {
        let (board, _dir) = temp_board();
        let result = board.update_story_status("US-999", StoryStatus::Done, "Dev");
        assert!(result.is_err());
    }

    #[test]
    fn sprint_create_and_start() {
        let (board, _dir) = temp_board();
        board.add_story(make_story("US-1")).unwrap();
        let sprint_id = board.create_sprint("스프린트 1").unwrap();
        board.add_story_to_sprint("US-1", &sprint_id).unwrap();
        board.start_sprint(&sprint_id).unwrap();
        let state = board.shared_state();
        let s = state.lock().unwrap();
        assert_eq!(s.current_sprint_id.as_deref(), Some(sprint_id.as_str()));
        let story = s.stories.get("US-1").unwrap();
        assert!(matches!(story.status, StoryStatus::Todo));
    }

    #[test]
    fn sprint_velocity_counts_done_stories() {
        let (board, _dir) = temp_board();
        board.add_story(make_story("US-1")).unwrap();
        board.add_story(make_story("US-2")).unwrap();
        let sprint_id = board.create_sprint("v1").unwrap();
        board.add_story_to_sprint("US-1", &sprint_id).unwrap();
        board.add_story_to_sprint("US-2", &sprint_id).unwrap();
        board.start_sprint(&sprint_id).unwrap();
        board.update_story_status("US-1", StoryStatus::Done, "QA").unwrap();
        // US-2 is Todo — not Done
        let state = board.shared_state();
        let s = state.lock().unwrap();
        let sprint = s.sprints.iter().find(|sp| sp.id == sprint_id).unwrap();
        let vel = sprint.velocity(&s.stories);
        assert_eq!(vel, 3); // only US-1 (3pts) is Done
    }

    #[test]
    fn next_story_and_bug_ids_increment() {
        let (board, _dir) = temp_board();
        let id1 = board.next_story_id();
        let id2 = board.next_story_id();
        assert_eq!(id1, "US-1");
        assert_eq!(id2, "US-2");
        let b1 = board.next_bug_id();
        let b2 = board.next_bug_id();
        assert_eq!(b1, "BUG-1");
        assert_eq!(b2, "BUG-2");
    }

    #[test]
    fn render_includes_project_name() {
        let (board, _dir) = temp_board();
        let rendered = board.render();
        assert!(rendered.contains("test-project"));
    }

    #[test]
    fn board_save_and_reload() {
        let (board, dir) = temp_board();
        board.add_story(make_story("US-1")).unwrap();
        board.save().unwrap();

        let board2 = AgileBoard {
            state: Arc::new(Mutex::new(BoardState::new("test-project"))),
            path: dir.path().join(".agile_board.json").to_string_lossy().into_owned(),
        };
        board2.load_from(&dir.path().join(".agile_board.json").to_string_lossy()).unwrap();
        let s = board2.get_story("US-1").unwrap();
        assert_eq!(s.title, "테스트 스토리");
    }

    #[test]
    fn get_stories_by_status() {
        let (board, _dir) = temp_board();
        board.add_story(make_story("US-1")).unwrap();
        board.add_story(make_story("US-2")).unwrap();
        board.update_story_status("US-2", StoryStatus::InProgress, "Dev").unwrap();
        let backlog = board.get_stories_by_status(&StoryStatus::Backlog);
        assert_eq!(backlog.len(), 1);
        assert_eq!(backlog[0].id, "US-1");
    }
}
