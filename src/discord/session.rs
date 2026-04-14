//! Per-channel session management

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::models::Message;

/// Channel session (channel ID → history)
#[derive(Default)]
pub struct SessionStore {
    inner: Arc<Mutex<HashMap<u64, ChannelSession>>>,
}

impl Clone for SessionStore {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

pub struct ChannelSession {
    pub history: Vec<Message>,
    pub created_at: std::time::Instant,
    pub message_count: usize,
}

impl ChannelSession {
    pub fn new(system_prompt: &str) -> Self {
        Self {
            history: vec![Message::system(system_prompt)],
            created_at: std::time::Instant::now(),
            message_count: 0,
        }
    }
}

impl SessionStore {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Get session (create if not found)
    pub fn get_or_create(&self, channel_id: u64, system_prompt: &str) -> Vec<Message> {
        let mut store = self.inner.lock().unwrap();
        let session = store.entry(channel_id)
            .or_insert_with(|| ChannelSession::new(system_prompt));
        session.history.clone()
    }

    /// Update history
    pub fn update(&self, channel_id: u64, history: Vec<Message>) {
        let mut store = self.inner.lock().unwrap();
        if let Some(session) = store.get_mut(&channel_id) {
            session.history = history;
            session.message_count += 1;
        }
    }

    /// Clear session
    pub fn clear(&self, channel_id: u64, system_prompt: &str) {
        let mut store = self.inner.lock().unwrap();
        store.insert(channel_id, ChannelSession::new(system_prompt));
    }

    /// Session statistics
    pub fn stats(&self, channel_id: u64) -> Option<(usize, String)> {
        let store = self.inner.lock().unwrap();
        store.get(&channel_id).map(|s| {
            let elapsed = s.created_at.elapsed();
            let time_str = if elapsed.as_secs() < 60 {
                format!("{}s", elapsed.as_secs())
            } else {
                format!("{}m", elapsed.as_secs() / 60)
            };
            (s.history.len(), time_str)
        })
    }
}
