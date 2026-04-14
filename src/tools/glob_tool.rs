use anyhow::Result;

/// 파일 패턴 검색 (glob)
/// 예: **/*.rs, src/**/*.toml
pub fn glob_files(pattern: &str) -> Result<Vec<String>> {
    let entries = glob::glob(pattern)
        .map_err(|e| anyhow::anyhow!("잘못된 glob 패턴: {}", e))?;

    let mut files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    files.sort();

    if files.len() > 200 {
        files.truncate(200);
        files.push(format!("... (200개로 제한됨)"));
    }

    Ok(files)
}
