//! User story and task definitions
//!
//! Core units of Agile development:
//!   Epic → UserStory → Task
//!
//! Each story includes Acceptance Criteria and
//! a QA checklist.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Priority ─────────────────────────────────────────────────────────────────

// Declaration order is Ord magnitude — Low(0) < Medium(1) < High(2) < Critical(3)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    pub fn icon(&self) -> &'static str {
        match self {
            Priority::Low      => "🟢",
            Priority::Medium   => "🟡",
            Priority::High     => "🟠",
            Priority::Critical => "🔴",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Priority::Low      => "Low",
            Priority::Medium   => "Medium",
            Priority::High     => "High",
            Priority::Critical => "Critical",
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.label())
    }
}

// ─── Kanban columns (story status) ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StoryStatus {
    // ── Preparation stage ─────────────────
    Backlog,
    Todo,
    // ── Planning/UX stage ──────────────
    UXReview,          // BA + UX review in progress
    // ── Development stage ─────────────────
    InProgress,
    Review,            // Awaiting code review
    QA,                // QA verification in progress
    QAFailed,          // QA failed → returns to InProgress
    // ── Security audit ─────────────────
    SecurityReview,    // HackerAgent security audit in progress
    // ── Approval/deployment preparation ────────────
    TechLeadReview,    // TechLead gate review
    Documentation,     // TechnicalWriter documentation in progress
    DevOpsSetup,       // DevOpsEngineer CI/CD setup in progress
    SRESetup,          // SRE monitoring/runbook setup in progress
    ReleasePrep,       // ReleaseManager release preparation in progress
    // ── Done ──────────────────────
    Released,          // Release complete
    Done,              // Fast completion (optional stages skipped)
    Blocked(String),   // Blocking reason
}

impl StoryStatus {
    pub fn column_name(&self) -> &'static str {
        match self {
            StoryStatus::Backlog        => "📦 Backlog",
            StoryStatus::Todo           => "📋 Todo",
            StoryStatus::UXReview       => "🎨 UX Review",
            StoryStatus::InProgress     => "⚙️  In Progress",
            StoryStatus::Review         => "👁️  Review",
            StoryStatus::QA             => "🔬 QA",
            StoryStatus::QAFailed       => "❌ QA Failed",
            StoryStatus::SecurityReview => "🔒 Security Review",
            StoryStatus::TechLeadReview => "🎯 TechLead Review",
            StoryStatus::Documentation  => "📝 Documentation",
            StoryStatus::DevOpsSetup    => "🚀 DevOps Setup",
            StoryStatus::SRESetup       => "📡 SRE Setup",
            StoryStatus::ReleasePrep    => "📦 Release Prep",
            StoryStatus::Released       => "🎉 Released",
            StoryStatus::Done           => "✅ Done",
            StoryStatus::Blocked(_)     => "🚫 Blocked",
        }
    }

    pub fn next_status(&self) -> Option<StoryStatus> {
        match self {
            StoryStatus::Backlog        => Some(StoryStatus::Todo),
            StoryStatus::Todo           => Some(StoryStatus::UXReview),
            StoryStatus::UXReview       => Some(StoryStatus::InProgress),
            StoryStatus::InProgress     => Some(StoryStatus::Review),
            StoryStatus::Review         => Some(StoryStatus::QA),
            StoryStatus::QA             => Some(StoryStatus::SecurityReview),
            StoryStatus::QAFailed       => Some(StoryStatus::InProgress),
            StoryStatus::SecurityReview => Some(StoryStatus::TechLeadReview),
            StoryStatus::TechLeadReview => Some(StoryStatus::Documentation),
            StoryStatus::Documentation  => Some(StoryStatus::DevOpsSetup),
            StoryStatus::DevOpsSetup    => Some(StoryStatus::SRESetup),
            StoryStatus::SRESetup       => Some(StoryStatus::ReleasePrep),
            StoryStatus::ReleasePrep    => Some(StoryStatus::Released),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, StoryStatus::Released | StoryStatus::Done | StoryStatus::QAFailed)
    }
}

// ─── Task (sub-item of story) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub assigned_to: Option<String>,  // agent role name
    pub done: bool,
    pub notes: String,
}

// ─── QA checklist item ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QACheck {
    pub description: String,
    pub passed: Option<bool>,   // None = not yet verified
    pub notes: String,
}

impl QACheck {
    pub fn new(description: &str) -> Self {
        Self {
            description: description.to_string(),
            passed: None,
            notes: String::new(),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self.passed {
            Some(true)  => "✅",
            Some(false) => "❌",
            None        => "⏳",
        }
    }
}

// ─── User story ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStory {
    pub id: String,
    pub epic: Option<String>,
    pub title: String,
    pub description: String,       // As a [role], I want [feature], so that [benefit]
    pub acceptance_criteria: Vec<String>,
    pub tasks: Vec<Task>,
    pub qa_checks: Vec<QACheck>,
    pub priority: Priority,
    pub story_points: u8,          // 1, 2, 3, 5, 8, 13
    pub status: StoryStatus,
    pub assigned_to: Option<String>,
    pub sprint_id: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    // Agent outputs — planning/UX
    pub business_analysis: Option<String>,
    pub ux_design: Option<String>,
    // Agent outputs — development
    pub plan: Option<String>,
    pub implementation: Option<String>,
    pub review_feedback: Option<String>,
    pub qa_report: Option<String>,
    pub bug_reports: Vec<BugReport>,
    // Agent outputs — deployment/operations
    pub tech_lead_review: Option<String>,
    pub devops_artifacts: Option<String>,
    pub docs: Option<String>,
    pub sre_config: Option<String>,
    pub release_notes: Option<String>,
}

impl UserStory {
    pub fn new(id: &str, title: &str, description: &str, priority: Priority, points: u8) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            id: id.to_string(),
            epic: None,
            title: title.to_string(),
            description: description.to_string(),
            acceptance_criteria: Vec::new(),
            tasks: Vec::new(),
            qa_checks: Vec::new(),
            priority,
            story_points: points,
            status: StoryStatus::Backlog,
            assigned_to: None,
            sprint_id: None,
            created_at: now,
            updated_at: now,
            business_analysis: None,
            ux_design: None,
            plan: None,
            implementation: None,
            review_feedback: None,
            qa_report: None,
            bug_reports: Vec::new(),
            tech_lead_review: None,
            devops_artifacts: None,
            docs: None,
            sre_config: None,
            release_notes: None,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Add acceptance criteria
    pub fn add_acceptance_criteria(&mut self, criteria: &str) -> &mut Self {
        self.acceptance_criteria.push(criteria.to_string());
        self
    }

    /// Add QA checklist item
    pub fn add_qa_check(&mut self, description: &str) -> &mut Self {
        self.qa_checks.push(QACheck::new(description));
        self
    }

    /// Whether QA passed (true if all checks pass)
    pub fn qa_passed(&self) -> bool {
        if self.qa_checks.is_empty() { return false; }
        self.qa_checks.iter().all(|c| c.passed == Some(true))
    }

    /// Completed tasks / total tasks
    pub fn task_progress(&self) -> (usize, usize) {
        let done = self.tasks.iter().filter(|t| t.done).count();
        (done, self.tasks.len())
    }

    /// Print brief summary
    pub fn summary(&self) -> String {
        let (done, total) = self.task_progress();
        let qa_ok = if self.qa_checks.is_empty() {
            String::new()
        } else {
            let passed = self.qa_checks.iter().filter(|c| c.passed == Some(true)).count();
            format!(" QA:{}/{}", passed, self.qa_checks.len())
        };
        format!(
            "[{}] {} {} {} ({} pts, {}/{}tasks{})",
            self.id,
            self.priority.icon(),
            self.status.column_name(),
            self.title,
            self.story_points,
            done, total,
            qa_ok,
        )
    }
}

// ─── Bug report ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub id: String,
    pub story_id: String,
    pub title: String,
    pub description: String,
    pub severity: Priority,
    pub steps_to_reproduce: Vec<String>,
    pub expected: String,
    pub actual: String,
    pub fixed: bool,
    pub created_at: u64,
}

impl BugReport {
    pub fn new(id: &str, story_id: &str, title: &str, severity: Priority) -> Self {
        Self {
            id: id.to_string(),
            story_id: story_id.to_string(),
            title: title.to_string(),
            description: String::new(),
            severity,
            steps_to_reproduce: Vec::new(),
            expected: String::new(),
            actual: String::new(),
            fixed: false,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_creation() {
        let story = UserStory::new("US-1", "Login feature", "User must be able to log in", Priority::High, 5);
        assert_eq!(story.id, "US-1");
        assert_eq!(story.story_points, 5);
        assert!(matches!(story.status, StoryStatus::Backlog));
    }

    #[test]
    fn qa_passes_when_all_checks_pass() {
        let mut story = UserStory::new("US-2", "test", "desc", Priority::Low, 1);
        story.add_qa_check("Unit tests pass");
        story.add_qa_check("Integration tests pass");
        story.qa_checks[0].passed = Some(true);
        story.qa_checks[1].passed = Some(true);
        assert!(story.qa_passed());
    }

    #[test]
    fn qa_fails_when_any_check_fails() {
        let mut story = UserStory::new("US-3", "test", "desc", Priority::Low, 1);
        story.add_qa_check("Test A");
        story.add_qa_check("Test B");
        story.qa_checks[0].passed = Some(true);
        story.qa_checks[1].passed = Some(false);
        assert!(!story.qa_passed());
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Critical > Priority::Low);
        assert!(Priority::High > Priority::Medium);
    }

    #[test]
    fn status_transitions() {
        assert_eq!(StoryStatus::Backlog.next_status(), Some(StoryStatus::Todo));
        assert_eq!(StoryStatus::Todo.next_status(), Some(StoryStatus::UXReview));
        assert_eq!(StoryStatus::UXReview.next_status(), Some(StoryStatus::InProgress));
        assert_eq!(StoryStatus::QA.next_status(), Some(StoryStatus::SecurityReview));
        assert_eq!(StoryStatus::SecurityReview.next_status(), Some(StoryStatus::TechLeadReview));
        assert_eq!(StoryStatus::TechLeadReview.next_status(), Some(StoryStatus::Documentation));
        assert_eq!(StoryStatus::ReleasePrep.next_status(), Some(StoryStatus::Released));
        assert_eq!(StoryStatus::QAFailed.next_status(), Some(StoryStatus::InProgress));
        assert!(StoryStatus::Released.is_terminal());
        assert!(StoryStatus::Done.is_terminal());
    }
}
