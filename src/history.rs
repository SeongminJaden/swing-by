//! 대화 기록 영속화
//!
//! 세션 간에 대화 히스토리를 JSON 파일로 저장하고 불러옵니다.
//!
//! 저장 위치:
//!   ~/.claude/projects/<project_hash>/history.json  (전역)
//!   ./.ai_history.json                               (프로젝트 로컬)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::models::Message;

// ─── 세션 기록 ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: u64,
    pub title: String,           // 첫 번째 사용자 메시지 요약
    pub messages: Vec<Message>,
    pub token_count: usize,      // 추정 토큰 수 (문자 수 / 4)
}

impl Session {
    pub fn new(id: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            id: id.to_string(),
            created_at: now,
            title: String::new(),
            messages: Vec::new(),
            token_count: 0,
        }
    }

    pub fn add_message(&mut self, msg: Message) {
        self.token_count += msg.content.len() / 4;
        // 첫 사용자 메시지를 제목으로 사용
        if self.title.is_empty() {
            if msg.role == crate::models::Role::User {
                self.title = crate::utils::trunc(&msg.content, 60).to_string();
            }
        }
        self.messages.push(msg);
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn summary(&self) -> String {
        let ts = chrono_format(self.created_at);
        format!(
            "[{}] {} — {} 메시지, ~{}토큰",
            ts,
            if self.title.is_empty() { "(제목 없음)" } else { &self.title },
            self.message_count(),
            self.token_count,
        )
    }
}

fn chrono_format(unix: u64) -> String {
    // 간단한 날짜 포맷 (외부 크레이트 없이)
    let secs = unix % 86400;
    let days = unix / 86400;
    // 1970-01-01 기준 대략적 날짜 (정확하지 않아도 표시용으로 충분)
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, h, m)
}

// ─── 히스토리 저장소 ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HistoryStore {
    pub sessions: Vec<Session>,
}

impl HistoryStore {
    pub fn load(path: &PathBuf) -> Self {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("디렉토리 생성 실패: {:?}", parent))?;
        }
        let json = serde_json::to_string_pretty(self).context("직렬화 실패")?;
        std::fs::write(path, json)
            .with_context(|| format!("히스토리 저장 실패: {:?}", path))
    }

    pub fn add_session(&mut self, session: Session) {
        // 같은 ID가 있으면 덮어쓰기
        if let Some(pos) = self.sessions.iter().position(|s| s.id == session.id) {
            self.sessions[pos] = session;
        } else {
            self.sessions.push(session);
        }
        // 최대 100세션 유지
        if self.sessions.len() > 100 {
            self.sessions.drain(0..self.sessions.len() - 100);
        }
    }

    pub fn last_session_messages(&self) -> Vec<Message> {
        self.sessions.last()
            .map(|s| s.messages.clone())
            .unwrap_or_default()
    }

    pub fn list(&self) -> Vec<String> {
        self.sessions.iter().rev().take(20)
            .map(|s| s.summary())
            .collect()
    }
}

// ─── 기본 경로 ───────────────────────────────────────────────────────────────

pub fn default_history_path() -> PathBuf {
    // 프로젝트 로컬 우선
    let local = PathBuf::from(".ai_history.json");
    if local.exists() {
        return local;
    }
    // 전역 경로: ~/.ai_agent/history.json
    dirs_or_home().join(".ai_agent").join("history.json")
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

// ─── 히스토리 관리자 ─────────────────────────────────────────────────────────

pub struct HistoryManager {
    pub store: HistoryStore,
    pub path: PathBuf,
    pub current_session: Session,
}

impl HistoryManager {
    pub fn new() -> Self {
        let path = default_history_path();
        let store = HistoryStore::load(&path);
        let session_id = format!("S-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0));
        Self {
            store,
            path,
            current_session: Session::new(&session_id),
        }
    }

    /// 이전 세션의 마지막 N개 메시지를 컨텍스트로 로드
    pub fn load_context(&self, max_messages: usize) -> Vec<Message> {
        self.store.sessions.last()
            .map(|s| {
                let msgs = &s.messages;
                let skip = msgs.len().saturating_sub(max_messages);
                // system 메시지 제외하고 최근 N개
                msgs[skip..].iter()
                    .filter(|m| m.role != crate::models::Role::System)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn record(&mut self, msg: Message) {
        self.current_session.add_message(msg);
    }

    pub fn save_session(&mut self) -> Result<()> {
        if self.current_session.messages.is_empty() { return Ok(()); }
        self.store.add_session(self.current_session.clone());
        self.store.save(&self.path)
    }

    pub fn print_history(&self) {
        let list = self.store.list();
        if list.is_empty() {
            println!("저장된 대화 기록 없음");
            return;
        }
        println!("\n── 최근 대화 기록 (최대 20개) ──");
        for (i, entry) in list.iter().enumerate() {
            println!("  {}. {}", i + 1, entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, Role};
    use tempfile::TempDir;

    fn tmp_path(dir: &TempDir) -> PathBuf {
        dir.path().join("history.json")
    }

    #[test]
    fn session_add_message_updates_token_count() {
        let mut session = Session::new("S-1");
        session.add_message(Message::user("hello world")); // 11 chars / 4 = 2
        assert!(session.token_count > 0);
        assert_eq!(session.message_count(), 1);
    }

    #[test]
    fn session_title_set_from_first_user_message() {
        let mut session = Session::new("S-1");
        session.add_message(Message::system("system"));
        assert!(session.title.is_empty(), "system message must not set title");
        session.add_message(Message::user("안녕하세요 반갑습니다"));
        assert!(!session.title.is_empty());
        assert!(session.title.contains("안녕하세요"));
    }

    #[test]
    fn store_save_and_reload() {
        let dir = TempDir::new().unwrap();
        let path = tmp_path(&dir);

        let mut store = HistoryStore::default();
        let mut s = Session::new("S-1");
        s.add_message(Message::user("test message"));
        store.add_session(s);
        store.save(&path).unwrap();

        let loaded = HistoryStore::load(&path);
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].message_count(), 1);
    }

    #[test]
    fn store_max_100_sessions() {
        let dir = TempDir::new().unwrap();
        let path = tmp_path(&dir);
        let mut store = HistoryStore::default();
        for i in 0..105 {
            let mut s = Session::new(&format!("S-{}", i));
            s.add_message(Message::user("x"));
            store.add_session(s);
        }
        store.save(&path).unwrap();
        let loaded = HistoryStore::load(&path);
        assert!(loaded.sessions.len() <= 100);
    }

    #[test]
    fn store_update_existing_session() {
        let dir = TempDir::new().unwrap();
        let path = tmp_path(&dir);
        let mut store = HistoryStore::default();
        let mut s = Session::new("S-DUP");
        s.add_message(Message::user("first"));
        store.add_session(s);
        let mut s2 = Session::new("S-DUP");
        s2.add_message(Message::user("first"));
        s2.add_message(Message::assistant("second"));
        store.add_session(s2);
        store.save(&path).unwrap();
        let loaded = HistoryStore::load(&path);
        assert_eq!(loaded.sessions.len(), 1); // 중복 없음
        assert_eq!(loaded.sessions[0].message_count(), 2);
    }

    #[test]
    fn manager_load_context_skips_system() {
        let dir = TempDir::new().unwrap();
        let path = tmp_path(&dir);

        let mut store = HistoryStore::default();
        let mut s = Session::new("S-1");
        s.add_message(Message::system("sys"));
        s.add_message(Message::user("q1"));
        s.add_message(Message::assistant("a1"));
        store.add_session(s);
        store.save(&path).unwrap();

        let mgr = HistoryManager { store: HistoryStore::load(&path), path, current_session: Session::new("S-2") };
        let ctx = mgr.load_context(10);
        assert!(!ctx.iter().any(|m| m.role == Role::System), "context must exclude system messages");
        assert_eq!(ctx.len(), 2); // user + assistant only
    }
}