use anyhow::Result;
use regex::Regex;
use walkdir::WalkDir;

const MAX_RESULTS: usize = 100;

/// 파일 내용 검색 (grep)
/// pattern: 정규식 (앞에 -i 붙이면 대소문자 무시)
/// path: 검색 경로 (파일 또는 디렉토리)
pub fn grep_files(pattern: &str, path: &str) -> Result<Vec<String>> {
    // "-i <pattern>" 형식 지원
    let (case_insensitive, pat) = if let Some(p) = pattern.strip_prefix("-i ") {
        (true, p)
    } else if pattern.starts_with("(?i)") {
        (false, pattern) // 이미 (?i) 포함
    } else {
        (false, pattern)
    };

    let regex_str = if case_insensitive {
        format!("(?i){}", pat)
    } else {
        pat.to_string()
    };

    let regex = Regex::new(&regex_str)
        .map_err(|e| anyhow::anyhow!("잘못된 정규식 '{}': {}", pat, e))?;

    let target = if path.is_empty() || path == "." { "." } else { path };
    let mut results = Vec::new();

    let meta = std::fs::metadata(target);
    let is_file = meta.as_ref().map(|m| m.is_file()).unwrap_or(false);

    if is_file {
        grep_one_file(&regex, target, &mut results);
    } else {
        for entry in WalkDir::new(target)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if results.len() >= MAX_RESULTS {
                results.push(format!("... ({}개 이상, 출력 제한됨)", MAX_RESULTS));
                break;
            }
            let p = entry.path().to_str().unwrap_or("");
            if is_binary_ext(p) {
                continue;
            }
            grep_one_file(&regex, p, &mut results);
        }
    }

    Ok(results)
}

fn grep_one_file(regex: &Regex, path: &str, results: &mut Vec<String>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (line_num, line) in content.lines().enumerate() {
        if regex.is_match(line) {
            results.push(format!("{}:{}: {}", path, line_num + 1, line));
            if results.len() >= MAX_RESULTS {
                return;
            }
        }
    }
}

fn is_binary_ext(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp"
            | "pdf" | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z"
            | "exe" | "so" | "dll" | "dylib" | "a" | "lib"
            | "mp3" | "mp4" | "avi" | "mkv" | "wav" | "flac"
            | "wasm" | "bin" | "dat" | "db" | "sqlite"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn temp_file_with(content: &str, ext: &str) -> NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(ext).tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn grep_single_file_basic() {
        let f = temp_file_with("hello world\nfoo bar\nhello again", ".txt");
        let path = f.path().to_str().unwrap();
        let results = grep_files("hello", path).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].contains("hello world"));
        assert!(results[1].contains("hello again"));
    }

    #[test]
    fn grep_case_insensitive() {
        let f = temp_file_with("Hello World\nFOO BAR\nhello again", ".txt");
        let path = f.path().to_str().unwrap();
        let results = grep_files("-i hello", path).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn grep_no_match_returns_empty() {
        let f = temp_file_with("nothing here", ".txt");
        let path = f.path().to_str().unwrap();
        let results = grep_files("xyz_not_found", path).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn grep_invalid_regex_returns_err() {
        let f = temp_file_with("test", ".txt");
        let path = f.path().to_str().unwrap();
        let err = grep_files("[invalid", path).unwrap_err();
        assert!(err.to_string().contains("잘못된 정규식"));
    }

    #[test]
    fn grep_directory_search() {
        let dir = TempDir::new().unwrap();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.txt");
        std::fs::write(&file1, "match_target here").unwrap();
        std::fs::write(&file2, "no match").unwrap();
        let results = grep_files("match_target", dir.path().to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("match_target"));
    }

    #[test]
    fn grep_result_format_includes_line_number() {
        let f = temp_file_with("line one\ntarget line\nline three", ".txt");
        let path = f.path().to_str().unwrap();
        let results = grep_files("target", path).unwrap();
        assert_eq!(results.len(), 1);
        // format: "path:linenum: content"
        assert!(results[0].contains(":2:"));
    }
}
