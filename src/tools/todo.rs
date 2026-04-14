use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const TODO_FILE: &str = ".ai_todos.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,   // "pending" | "in_progress" | "completed"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>, // "high" | "medium" | "low"
}

/// Save TODO list (JSON array string)
pub fn todo_write(json_str: &str) -> Result<String> {
    let todos: Vec<TodoItem> = serde_json::from_str(json_str.trim())
        .context("Failed to parse TODO JSON. Expected format: [{\"id\":\"1\",\"content\":\"task\",\"status\":\"pending\"}]")?;

    let json = serde_json::to_string_pretty(&todos)
        .context("Failed to serialize TODO list")?;
    std::fs::write(TODO_FILE, &json)
        .context("Failed to write TODO file")?;

    Ok(format_todos(&todos))
}

/// Read TODO list
pub fn todo_read() -> Result<Vec<TodoItem>> {
    if !std::path::Path::new(TODO_FILE).exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(TODO_FILE)
        .context("Failed to read TODO file")?;
    serde_json::from_str(&content).context("Failed to parse TODO JSON")
}

fn format_todos(todos: &[TodoItem]) -> String {
    if todos.is_empty() {
        return "TODO list is empty.".to_string();
    }

    todos
        .iter()
        .map(|t| {
            let status_icon = match t.status.as_str() {
                "completed" => "✅",
                "in_progress" => "🔄",
                _ => "⏳",
            };
            let prio_icon = match t.priority.as_deref().unwrap_or("medium") {
                "high" => "🔴",
                "low" => "🟢",
                _ => "🟡",
            };
            format!("{} {} [{}] {}", status_icon, prio_icon, t.id, t.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Serialize all tests that change the working directory
    static DIR_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_dir<F: FnOnce()>(f: F) {
        let _lock = DIR_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        f();
        std::env::set_current_dir(orig).unwrap();
    }

    #[test]
    fn todo_write_and_read_roundtrip() {
        with_temp_dir(|| {
            let json = r#"[{"id":"1","content":"test task","status":"pending"}]"#;
            let output = todo_write(json).unwrap();
            assert!(output.contains("test task"));

            let items = todo_read().unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].id, "1");
            assert_eq!(items[0].status, "pending");
        });
    }

    #[test]
    fn todo_read_empty_when_no_file() {
        with_temp_dir(|| {
            let items = todo_read().unwrap();
            assert!(items.is_empty());
        });
    }

    #[test]
    fn todo_write_invalid_json_returns_err() {
        with_temp_dir(|| {
            let err = todo_write("not json").unwrap_err();
            assert!(err.to_string().contains("parsing failed"));
        });
    }

    #[test]
    fn todo_write_with_priority() {
        with_temp_dir(|| {
            let json = r#"[{"id":"1","content":"high prio","status":"pending","priority":"high"}]"#;
            let output = todo_write(json).unwrap();
            assert!(output.contains("🔴")); // high priority icon
        });
    }

    #[test]
    fn format_status_icons() {
        let items = vec![
            TodoItem { id: "1".into(), content: "a".into(), status: "completed".into(), priority: None },
            TodoItem { id: "2".into(), content: "b".into(), status: "in_progress".into(), priority: None },
            TodoItem { id: "3".into(), content: "c".into(), status: "pending".into(), priority: None },
        ];
        let out = format_todos(&items);
        assert!(out.contains("✅"));
        assert!(out.contains("🔄"));
        assert!(out.contains("⏳"));
    }
}
