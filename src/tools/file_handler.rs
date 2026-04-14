use anyhow::{Context, Result};
use std::path::Path;
use tracing::instrument;

/// 파일 읽기
#[instrument]
pub fn read_file(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("파일 읽기 실패: {}", path))?;
    Ok(content)
}

/// 파일 쓰기 (덮어쓰기)
#[instrument(skip(content))]
pub fn write_file(path: &str, content: &str) -> Result<()> {
    // 부모 디렉토리가 없으면 생성
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("디렉토리 생성 실패: {}", parent.display()))?;
        }
    }
    std::fs::write(path, content)
        .with_context(|| format!("파일 쓰기 실패: {}", path))?;
    Ok(())
}

/// 파일에 내용 추가 (append)
#[instrument(skip(content))]
pub fn append_file(path: &str, content: &str) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("파일 열기 실패: {}", path))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("파일 쓰기 실패: {}", path))?;
    Ok(())
}

/// 디렉토리 목록 (재귀 아님)
#[instrument]
pub fn list_dir(path: &str) -> Result<Vec<String>> {
    let entries = std::fs::read_dir(path)
        .with_context(|| format!("디렉토리 읽기 실패: {}", path))?;

    let mut items: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                format!("{}/", name)
            } else {
                name
            }
        })
        .collect();

    items.sort();
    Ok(items)
}

/// 파일 삭제
#[instrument]
pub fn delete_file(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.is_dir() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("디렉토리 삭제 실패: {}", path))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("파일 삭제 실패: {}", path))?;
    }
    Ok(())
}

/// 파일/디렉토리 이동 (rename)
#[instrument]
pub fn move_file(src: &str, dst: &str) -> Result<()> {
    // dst가 기존 디렉토리이면 그 안으로 이동
    let dst_path = Path::new(dst);
    let final_dst = if dst_path.is_dir() {
        let filename = Path::new(src).file_name()
            .ok_or_else(|| anyhow::anyhow!("소스 파일명을 읽을 수 없습니다: {}", src))?;
        dst_path.join(filename)
    } else {
        dst_path.to_path_buf()
    };
    if let Some(parent) = final_dst.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("디렉토리 생성 실패: {}", parent.display()))?;
        }
    }
    std::fs::rename(src, &final_dst)
        .with_context(|| format!("이동 실패: {} → {}", src, final_dst.display()))?;
    Ok(())
}

/// 파일 복사
#[instrument]
pub fn copy_file(src: &str, dst: &str) -> Result<u64> {
    let dst_path = Path::new(dst);
    let final_dst = if dst_path.is_dir() {
        let filename = Path::new(src).file_name()
            .ok_or_else(|| anyhow::anyhow!("소스 파일명을 읽을 수 없습니다: {}", src))?;
        dst_path.join(filename)
    } else {
        dst_path.to_path_buf()
    };
    if let Some(parent) = final_dst.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("디렉토리 생성 실패: {}", parent.display()))?;
        }
    }
    let bytes = std::fs::copy(src, &final_dst)
        .with_context(|| format!("복사 실패: {} → {}", src, final_dst.display()))?;
    Ok(bytes)
}

/// 디렉토리 생성 (중간 경로 포함)
#[instrument]
pub fn make_dir(path: &str) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("디렉토리 생성 실패: {}", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt").to_str().unwrap().to_string();
        write_file(&path, "hello").unwrap();
        let content = read_file(&path).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn write_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub/dir/file.txt").to_str().unwrap().to_string();
        write_file(&path, "nested").unwrap();
        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn read_missing_file_returns_err() {
        let err = read_file("/tmp/no_such_file_xyz.txt").unwrap_err();
        assert!(err.to_string().contains("파일 읽기 실패"));
    }

    #[test]
    fn append_file_creates_and_extends() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("append.txt").to_str().unwrap().to_string();
        append_file(&path, "first").unwrap();
        append_file(&path, " second").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "first second");
    }

    #[test]
    fn list_dir_returns_sorted_entries() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::create_dir(dir.path().join("z_dir")).unwrap();
        let entries = list_dir(dir.path().to_str().unwrap()).unwrap();
        // sorted: a.txt, b.txt, z_dir/
        assert_eq!(entries, vec!["a.txt", "b.txt", "z_dir/"]);
    }

    #[test]
    fn delete_file_removes_it() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("del.txt");
        std::fs::write(&path, "bye").unwrap();
        delete_file(path.to_str().unwrap()).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn delete_dir_removes_recursively() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("file.txt"), "x").unwrap();
        delete_file(sub.to_str().unwrap()).unwrap();
        assert!(!sub.exists());
    }

    #[test]
    fn move_file_basic() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");
        std::fs::write(&src, "data").unwrap();
        move_file(src.to_str().unwrap(), dst.to_str().unwrap()).unwrap();
        assert!(!src.exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "data");
    }

    #[test]
    fn move_file_into_directory() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let src = dir.path().join("file.txt");
        std::fs::write(&src, "hello").unwrap();
        move_file(src.to_str().unwrap(), sub.to_str().unwrap()).unwrap();
        assert!(sub.join("file.txt").exists());
    }

    #[test]
    fn copy_file_basic() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("orig.txt");
        let dst = dir.path().join("copy.txt");
        std::fs::write(&src, "content").unwrap();
        let bytes = copy_file(src.to_str().unwrap(), dst.to_str().unwrap()).unwrap();
        assert!(bytes > 0);
        assert!(src.exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "content");
    }

    #[test]
    fn make_dir_nested() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a/b/c");
        make_dir(nested.to_str().unwrap()).unwrap();
        assert!(nested.is_dir());
    }
}
