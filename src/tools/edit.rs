use anyhow::{Context, Result};

/// 파일에서 정확한 문자열을 찾아 교체 (edit_file)
pub fn edit_file(path: &str, old_str: &str, new_str: &str) -> Result<String> {
    if old_str.is_empty() {
        anyhow::bail!("교체할 문자열(OLD)이 비어있습니다.");
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("파일 읽기 실패: {}", path))?;

    if !content.contains(old_str) {
        anyhow::bail!(
            "파일에서 해당 문자열을 찾을 수 없습니다.\n---찾는 내용---\n{}\n---파일 경로: {}",
            old_str,
            path
        );
    }

    let count = content.matches(old_str).count();
    if count > 1 {
        anyhow::bail!(
            "해당 문자열이 {}번 발견됩니다. 교체할 내용을 더 구체적으로 지정해주세요.",
            count
        );
    }

    let new_content = content.replacen(old_str, new_str, 1);
    std::fs::write(path, &new_content)
        .with_context(|| format!("파일 쓰기 실패: {}", path))?;

    Ok(format!("파일 편집 완료: {}", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn temp_with(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn edit_basic_replace() {
        let f = temp_with("hello world");
        let path = f.path().to_str().unwrap();
        let result = edit_file(path, "world", "Rust").unwrap();
        assert!(result.contains("편집 완료"));
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "hello Rust");
    }

    #[test]
    fn edit_not_found_returns_err() {
        let f = temp_with("hello world");
        let path = f.path().to_str().unwrap();
        let err = edit_file(path, "missing", "x").unwrap_err();
        assert!(err.to_string().contains("찾을 수 없습니다"));
    }

    #[test]
    fn edit_ambiguous_returns_err() {
        let f = temp_with("foo foo foo");
        let path = f.path().to_str().unwrap();
        let err = edit_file(path, "foo", "bar").unwrap_err();
        assert!(err.to_string().contains("3번"));
    }

    #[test]
    fn edit_empty_old_returns_err() {
        let f = temp_with("hello");
        let path = f.path().to_str().unwrap();
        let err = edit_file(path, "", "something").unwrap_err();
        assert!(err.to_string().contains("비어있습니다"));
    }

    #[test]
    fn edit_missing_file_returns_err() {
        let err = edit_file("/tmp/nonexistent_xyz_test.txt", "x", "y").unwrap_err();
        assert!(err.to_string().contains("파일 읽기 실패"));
    }
}
