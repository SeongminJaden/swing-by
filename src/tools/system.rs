use anyhow::{Context, Result};
use std::process::Command;
use tracing::instrument;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_OUTPUT_BYTES: usize = 16_000;

#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    #[allow(dead_code)]
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

/// 셸 명령어 실행 (타임아웃 30초, 출력 16KB 제한)
#[instrument(skip(cmd))]
pub fn run_shell(cmd: &str) -> Result<CommandResult> {
    let parts = shlex::split(cmd)
        .ok_or_else(|| anyhow::anyhow!("명령어 파싱 실패: {}", cmd))?;

    if parts.is_empty() {
        anyhow::bail!("빈 명령어");
    }

    // 위험 명령어 기본 차단
    let first = parts[0].to_lowercase();
    if matches!(first.as_str(), "rm" | "dd" | "mkfs" | "fdisk" | "shutdown" | "reboot") {
        // 단순 경고만 하고 실행 (사용자가 의도한 경우 허용)
        // 필요 시 아래 줄을 활성화하여 차단 가능
        // anyhow::bail!("안전을 위해 '{}' 명령어는 차단되었습니다.", first);
    }

    let (program, args) = parts.split_first().unwrap();

    // 타임아웃을 위해 스레드에서 실행
    let timeout = std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS);
    let program_owned = program.clone();
    let args_owned: Vec<String> = args.to_vec();

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", &args_owned.join(" ")]).output()
        } else {
            Command::new(&program_owned).args(&args_owned).output()
        };
        let _ = tx.send(output);
    });

    let output = rx
        .recv_timeout(timeout)
        .with_context(|| format!("명령어 타임아웃 ({}초): {}", DEFAULT_TIMEOUT_SECS, cmd))?
        .with_context(|| format!("명령어 실행 실패: {}", program))?;

    let stdout = truncate_output(String::from_utf8_lossy(&output.stdout).to_string());
    let stderr = truncate_output(String::from_utf8_lossy(&output.stderr).to_string());

    Ok(CommandResult {
        stdout,
        stderr,
        exit_code: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}

fn truncate_output(s: String) -> String {
    if s.len() > MAX_OUTPUT_BYTES {
        let cut = crate::utils::trunc(&s, MAX_OUTPUT_BYTES);
        format!("{}\n[출력 잘림: 총 {}바이트]", cut, s.len())
    } else {
        s
    }
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

/// 환경변수 설정 (현재 프로세스 내)
pub fn set_env(key: &str, value: &str) -> Result<()> {
    if key.is_empty() {
        anyhow::bail!("환경변수 키가 비어있습니다");
    }
    std::env::set_var(key, value);
    Ok(())
}

/// 환경변수 목록 (필터 옵션)
pub fn env_list(filter: &str) -> Vec<(String, String)> {
    let filter_lower = filter.to_lowercase();
    let mut vars: Vec<(String, String)> = std::env::vars()
        .filter(|(k, v)| {
            if filter.is_empty() { return true; }
            k.to_lowercase().contains(&filter_lower)
                || v.to_lowercase().contains(&filter_lower)
        })
        .collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    vars
}
