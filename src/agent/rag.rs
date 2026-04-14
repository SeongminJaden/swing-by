//! Codebase RAG (Retrieval-Augmented Generation)
//!
//! Indexes project files into chunks and injects relevant chunks into context at query time.
//! Implemented with simple TF-IDF keyword scoring — no external vector DB required.
//!
//! Supported commands:
//!   - /rag index [path]  : Index the specified path (default: current directory)
//!   - /rag query <question>  : Search relevant code chunks and answer via AI
//!   - /rag status        : Show index status

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const INDEX_FILE: &str = ".rag_index.json";
const MAX_CHUNK_SIZE: usize = 800;   // 청크당 최대 문자 수
const TOP_K: usize = 5;              // 쿼리 시 반환할 상위 청크 수
const MAX_FILES: usize = 500;        // 최대 인덱싱 파일 수

// Extensions to index
const INDEXED_EXTS: &[&str] = &[
    "rs", "py", "ts", "js", "tsx", "jsx", "go", "java", "c", "cpp", "h",
    "md", "toml", "yaml", "yml", "json", "sh", "sql",
];

// Directory patterns to skip
const SKIP_DIRS: &[&str] = &["target", "node_modules", ".git", "__pycache__", ".venv", "dist", "build"];

// ─── Data structures ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: usize,
    pub file: String,
    pub start_line: usize,
    pub content: String,
    pub tokens: Vec<String>,  // 소문자 키워드
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagIndex {
    pub root: String,
    pub chunks: Vec<Chunk>,
    pub file_count: usize,
    pub indexed_at: u64,
}

impl RagIndex {
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn status(&self) -> String {
        let age_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0)
            .saturating_sub(self.indexed_at);
        format!(
            "루트: {}\n파일: {} 개\n청크: {} 개\n인덱싱: {}분 전",
            self.root, self.file_count, self.chunks.len(), age_secs / 60
        )
    }
}

// ─── 인덱싱 ──────────────────────────────────────────────────────────────────

pub fn index_codebase(root: &str) -> Result<RagIndex> {
    let root_path = PathBuf::from(root).canonicalize().unwrap_or_else(|_| PathBuf::from(root));
    let mut chunks = Vec::new();
    let mut file_count = 0;
    let mut chunk_id = 0;

    collect_files(&root_path, &mut |path: &Path| {
        if file_count >= MAX_FILES { return; }
        let Ok(content) = std::fs::read_to_string(path) else { return };
        if content.is_empty() { return; }

        let rel_path = path.strip_prefix(&root_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        // Split file into chunks
        let lines: Vec<&str> = content.lines().collect();
        let mut start = 0;
        while start < lines.len() {
            let mut end = start;
            let mut size = 0;
            while end < lines.len() && size < MAX_CHUNK_SIZE {
                size += lines[end].len() + 1;
                end += 1;
            }
            let chunk_content = lines[start..end].join("\n");
            let tokens = tokenize(&chunk_content);

            chunks.push(Chunk {
                id: chunk_id,
                file: rel_path.clone(),
                start_line: start + 1,
                content: chunk_content,
                tokens,
            });
            chunk_id += 1;
            start = end;
        }
        file_count += 1;
    });

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);

    Ok(RagIndex {
        root: root_path.to_string_lossy().to_string(),
        chunks,
        file_count,
        indexed_at: now,
    })
}

fn collect_files(dir: &Path, callback: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if SKIP_DIRS.iter().any(|skip| name == *skip) { continue; }

        if path.is_dir() {
            collect_files(&path, callback);
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if INDEXED_EXTS.contains(&ext) {
                callback(&path);
            }
        }
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_string())
        .collect()
}

// ─── Save / Load ──────────────────────────────────────────────────────────────

pub fn save_index(index: &RagIndex) -> Result<()> {
    let json = serde_json::to_string(index)?;
    std::fs::write(INDEX_FILE, json)?;
    Ok(())
}

pub fn load_index() -> Option<RagIndex> {
    let json = std::fs::read_to_string(INDEX_FILE).ok()?;
    serde_json::from_str(&json).ok()
}

// ─── 검색 ────────────────────────────────────────────────────────────────────

pub fn search<'a>(index: &'a RagIndex, query: &str) -> Vec<&'a Chunk> {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() { return vec![]; }

    // TF 점수: 청크에 쿼리 토큰이 몇 번 나타나는지
    let mut scores: Vec<(usize, f64)> = index.chunks.iter().enumerate().map(|(i, chunk)| {
        let token_freq: HashMap<&str, usize> = chunk.tokens.iter().fold(HashMap::new(), |mut m, t| {
            *m.entry(t.as_str()).or_insert(0) += 1;
            m
        });
        let score: f64 = query_tokens.iter().map(|qt| {
            *token_freq.get(qt.as_str()).unwrap_or(&0) as f64
        }).sum();
        // 파일 이름에 쿼리 토큰이 있으면 가중치
        let file_bonus: f64 = query_tokens.iter()
            .filter(|qt| chunk.file.to_lowercase().contains(qt.as_str()))
            .count() as f64 * 2.0;
        (i, score + file_bonus)
    }).collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scores.iter()
        .filter(|(_, s)| *s > 0.0)
        .take(TOP_K)
        .map(|(i, _)| &index.chunks[*i])
        .collect()
}

/// 검색 결과를 컨텍스트 문자열로 변환
pub fn build_context(chunks: &[&Chunk]) -> String {
    if chunks.is_empty() {
        return String::new();
    }
    let mut parts = vec!["## 관련 코드 (RAG 검색 결과)".to_string()];
    for chunk in chunks {
        parts.push(format!("### {} ({}번째 줄~)\n```\n{}\n```",
            chunk.file, chunk.start_line, crate::utils::trunc(&chunk.content, 600)));
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("fn main() { println!(\"hello\"); }");
        assert!(tokens.contains(&"fn".to_string()));
        assert!(tokens.contains(&"main".to_string()));
        assert!(tokens.contains(&"println".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short() {
        let tokens = tokenize("a b cc ddd");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"b".to_string()));
        assert!(tokens.contains(&"cc".to_string()));
        assert!(tokens.contains(&"ddd".to_string()));
    }

    #[test]
    fn test_search_empty_index() {
        let index = RagIndex {
            root: ".".to_string(),
            chunks: vec![],
            file_count: 0,
            indexed_at: 0,
        };
        let results = search(&index, "hello");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_finds_relevant_chunk() {
        let chunk = Chunk {
            id: 0,
            file: "src/main.rs".to_string(),
            start_line: 1,
            content: "fn main() { println!(\"hello world\"); }".to_string(),
            tokens: tokenize("fn main() { println!(\"hello world\"); }"),
        };
        let index = RagIndex {
            root: ".".to_string(),
            chunks: vec![chunk],
            file_count: 1,
            indexed_at: 0,
        };
        let results = search(&index, "main println");
        assert!(!results.is_empty());
        assert_eq!(results[0].file, "src/main.rs");
    }

    #[test]
    fn test_build_context_empty() {
        let ctx = build_context(&[]);
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_build_context_with_chunks() {
        let chunk = Chunk {
            id: 0, file: "test.rs".to_string(), start_line: 1,
            content: "fn test() {}".to_string(),
            tokens: vec!["fn".to_string(), "test".to_string()],
        };
        let ctx = build_context(&[&chunk]);
        assert!(ctx.contains("RAG"));
        assert!(ctx.contains("test.rs"));
    }
}
