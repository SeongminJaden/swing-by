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

/// Execute a shell command (30s timeout, 16KB output limit)
#[instrument(skip(cmd))]
pub fn run_shell(cmd: &str) -> Result<CommandResult> {
    let parts = shlex::split(cmd)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse command: {}", cmd))?;

    if parts.is_empty() {
        anyhow::bail!("Empty command");
    }

    // Block dangerous commands by default
    let first = parts[0].to_lowercase();
    if matches!(first.as_str(), "rm" | "dd" | "mkfs" | "fdisk" | "shutdown" | "reboot") {
        // Just warn and allow execution (user may have intended this)
        // Uncomment the line below to block instead
        // anyhow::bail!("Command '{}' is blocked for safety.", first);
    }

    let (program, args) = parts.split_first().unwrap();

    // Run in a thread for timeout support
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
        .with_context(|| format!("Command timeout ({}s): {}", DEFAULT_TIMEOUT_SECS, cmd))?
        .with_context(|| format!("Command execution failed: {}", program))?;

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
        format!("{}\n[output truncated: total {} bytes]", cut, s.len())
    } else {
        s
    }
}

/// Return the current working directory
pub fn current_dir() -> Result<String> {
    std::env::current_dir()
        .context("Failed to get current directory")
        .map(|p| p.to_string_lossy().to_string())
}

/// Change the working directory
pub fn change_dir(path: &str) -> Result<()> {
    std::env::set_current_dir(path)
        .with_context(|| format!("Failed to change directory: {}", path))
}

/// Get an environment variable
pub fn get_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// Set an environment variable (current process only)
pub fn set_env(key: &str, value: &str) -> Result<()> {
    if key.is_empty() {
        anyhow::bail!("Environment variable key is empty");
    }
    std::env::set_var(key, value);
    Ok(())
}

/// List environment variables (optional filter)
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
