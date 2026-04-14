use anyhow::{Context, Result};

/// Find and replace an exact string in a file (edit_file)
pub fn edit_file(path: &str, old_str: &str, new_str: &str) -> Result<String> {
    if old_str.is_empty() {
        anyhow::bail!("Replacement string (OLD) is empty.");
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;

    if !content.contains(old_str) {
        anyhow::bail!(
            "The specified string was not found in the file.\n---search string---\n{}\n---file path: {}",
            old_str,
            path
        );
    }

    let count = content.matches(old_str).count();
    if count > 1 {
        anyhow::bail!(
            "String found {} times. Please be more specific.",
            count
        );
    }

    let new_content = content.replacen(old_str, new_str, 1);
    std::fs::write(path, &new_content)
        .with_context(|| format!("Failed to write file: {}", path))?;

    Ok(format!("File edited: {}", path))
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
        assert!(result.contains("File edited"));
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "hello Rust");
    }

    #[test]
    fn edit_not_found_returns_err() {
        let f = temp_with("hello world");
        let path = f.path().to_str().unwrap();
        let err = edit_file(path, "missing", "x").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn edit_ambiguous_returns_err() {
        let f = temp_with("foo foo foo");
        let path = f.path().to_str().unwrap();
        let err = edit_file(path, "foo", "bar").unwrap_err();
        assert!(err.to_string().contains("3 times"));
    }

    #[test]
    fn edit_empty_old_returns_err() {
        let f = temp_with("hello");
        let path = f.path().to_str().unwrap();
        let err = edit_file(path, "", "something").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn edit_missing_file_returns_err() {
        let err = edit_file("/tmp/nonexistent_xyz_test.txt", "x", "y").unwrap_err();
        assert!(err.to_string().contains("Failed to read"));
    }
}
