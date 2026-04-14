//! Conversation history persistence
//!
//! Saves and loads conversation history as a JSON file across sessions.
//!
//! Storage locations:
//!   ~/.claude/projects/<project_hash>/history.json  (global)
//!   ./.ai_history.json                               (project local)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::models::Message;

// ─── Session record ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: u64,
    pub title: String,           // Summary of the first user message
    pub messages: Vec<Message>,
    pub token_count: usize,      // Estimated token count (char count / 4)
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
        // Use the first user message as the session title
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
            "[{}] {} — {} messages, ~{} tokens",
            ts,
            if self.title.is_empty() { "(no title)" } else { &self.title },
            self.message_count(),
            self.token_count,
        )
    }
}

fn chrono_format(unix: u64) -> String {
    // Simple date format (no external crates)
    let secs = unix % 86400;
    let days = unix / 86400;
    // Approximate date from 1970-01-01 (good enough for display purposes)
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, h, m)
}

// ─── History store ────────────────────────────────────────────────────────────

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
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
        let json = serde_json::to_string_pretty(self).context("Serialization failed")?;
        std::fs::write(path, json)
            .with_context(|| format!("Failed to save history: {:?}", path))
    }

    pub fn add_session(&mut self, session: Session) {
        // Overwrite if the same ID exists
        if let Some(pos) = self.sessions.iter().position(|s| s.id == session.id) {
            self.sessions[pos] = session;
        } else {
            self.sessions.push(session);
        }
        // Keep at most 100 sessions
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

// ─── Default path ─────────────────────────────────────────────────────────────

pub fn default_history_path() -> PathBuf {
    // Prefer project local
    let local = PathBuf::from(".ai_history.json");
    if local.exists() {
        return local;
    }
    // Global path: ~/.ai_agent/history.json
    dirs_or_home().join(".ai_agent").join("history.json")
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

// ─── History manager ──────────────────────────────────────────────────────────

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

    /// Load the last N messages from the previous session as context
    pub fn load_context(&self, max_messages: usize) -> Vec<Message> {
        self.store.sessions.last()
            .map(|s| {
                let msgs = &s.messages;
                let skip = msgs.len().saturating_sub(max_messages);
                // Exclude system messages, take the last N
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
            println!("No saved conversation history");
            return;
        }
        println!("\n── Recent conversation history (up to 20) ──");
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
        session.add_message(Message::user("Hello, nice to meet you"));
        assert!(!session.title.is_empty());
        assert!(session.title.contains("Hello"));
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
        assert_eq!(loaded.sessions.len(), 1); // no duplicates
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