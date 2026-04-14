use anyhow::Result;

/// File pattern search (glob)
/// e.g. **/*.rs, src/**/*.toml
pub fn glob_files(pattern: &str) -> Result<Vec<String>> {
    let entries = glob::glob(pattern)
        .map_err(|e| anyhow::anyhow!("Invalid glob pattern: {}", e))?;

    let mut files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    files.sort();

    if files.len() > 200 {
        files.truncate(200);
        files.push(format!("... (limited to 200)"));
    }

    Ok(files)
}
