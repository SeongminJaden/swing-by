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
    std::fs::remove_file(path)
        .with_context(|| format!("파일 삭제 실패: {}", path))?;
    Ok(())
}
