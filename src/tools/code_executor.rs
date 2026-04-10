use anyhow::{Context, Result};
use std::process::Command;
use tempfile::NamedTempFile;
use tracing::instrument;

#[derive(Debug)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

impl std::fmt::Display for ExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.success {
            write!(f, "{}", self.stdout)
        } else {
            write!(f, "오류 (종료코드 {}):\n{}\n{}", self.exit_code, self.stdout, self.stderr)
        }
    }
}

/// Python 코드 실행
#[instrument(skip(code))]
pub fn run_python(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".py")
        .context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command("python", &[tmp.path().to_str().unwrap()])
}

/// JavaScript 코드 실행 (Node.js)
#[instrument(skip(code))]
pub fn run_javascript(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".js")
        .context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command("node", &[tmp.path().to_str().unwrap()])
}

/// Rust 코드 실행 (rustc + 실행)
#[instrument(skip(code))]
pub fn run_rust(code: &str) -> Result<ExecutionResult> {
    let tmp_dir = tempfile::tempdir().context("임시 디렉토리 생성 실패")?;
    let src_path = tmp_dir.path().join("main.rs");
    let bin_path = tmp_dir.path().join("main");

    std::fs::write(&src_path, code)?;

    // 컴파일
    let compile = run_command(
        "rustc",
        &[
            src_path.to_str().unwrap(),
            "-o",
            bin_path.to_str().unwrap(),
        ],
    )?;

    if !compile.success {
        return Ok(ExecutionResult {
            stdout: String::new(),
            stderr: format!("컴파일 오류:\n{}", compile.stderr),
            exit_code: compile.exit_code,
            success: false,
        });
    }

    // 실행
    run_command(bin_path.to_str().unwrap(), &[])
}

/// 셸 명령어로 코드 실행 (언어 자동 감지)
pub fn run_code(language: &str, code: &str) -> Result<ExecutionResult> {
    match language.to_lowercase().as_str() {
        "python" | "py" => run_python(code),
        "javascript" | "js" | "node" => run_javascript(code),
        "rust" | "rs" => run_rust(code),
        other => anyhow::bail!("지원하지 않는 언어: {}", other),
    }
}

/// 내부: 프로세스 실행 헬퍼
fn run_command(program: &str, args: &[&str]) -> Result<ExecutionResult> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("명령어 실행 실패: {}", program))?;

    Ok(ExecutionResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}
