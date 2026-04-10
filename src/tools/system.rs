use anyhow::{Context, Result};
use std::process::Command;
use tracing::instrument;

#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

impl std::fmt::Display for CommandResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.stdout.is_empty() {
            write!(f, "{}", self.stdout)?;
        }
        if !self.success && !self.stderr.is_empty() {
            write!(f, "\n[stderr] {}", self.stderr)?;
        }
        Ok(())
    }
}

/// 셸 명령어 실행 (보안: shlex로 파싱)
#[instrument(skip(cmd))]
pub fn run_shell(cmd: &str) -> Result<CommandResult> {
    let parts = shlex::split(cmd)
        .ok_or_else(|| anyhow::anyhow!("명령어 파싱 실패: {}", cmd))?;

    if parts.is_empty() {
        anyhow::bail!("빈 명령어");
    }

    let (program, args) = parts.split_first().unwrap();

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", cmd])
            .output()
            .with_context(|| format!("명령어 실행 실패: {}", cmd))?
    } else {
        Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("명령어 실행 실패: {}", program))?
    };

    Ok(CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}

/// 현재 디렉토리 반환
pub fn current_dir() -> Result<String> {
    std::env::current_dir()
        .context("현재 디렉토리 확인 실패")
        .map(|p| p.to_string_lossy().to_string())
}

/// 디렉토리 변경
pub fn change_dir(path: &str) -> Result<()> {
    std::env::set_current_dir(path)
        .with_context(|| format!("디렉토리 변경 실패: {}", path))
}

/// 환경변수 조회
pub fn get_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}
