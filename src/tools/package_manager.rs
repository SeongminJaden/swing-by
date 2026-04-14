use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;

const PKG_TIMEOUT_SECS: u64 = 300; // 패키지 설치는 최대 5분
const MAX_OUTPUT: usize = 16_000;

#[derive(Debug)]
pub struct PkgResult {
    pub output: String,
    #[allow(dead_code)]
    pub success: bool,
}

impl std::fmt::Display for PkgResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output)
    }
}

/// 패키지 설치
pub fn pkg_install(manager: &str, package: &str) -> Result<PkgResult> {
    match manager.to_lowercase().as_str() {
        "apt" | "apt-get" => run_pkg(&["sudo", "apt-get", "install", "-y", package]),
        "pip" | "pip3" => {
            // python3 -m pip 우선 시도 (pip3 없는 환경 대응)
            if run_quick("pip3", &["--version"]).is_ok() {
                run_pkg(&["pip3", "install", package])
            } else {
                run_pkg(&["python3", "-m", "pip", "install", package])
            }
        }
        "npm" => run_pkg(&["npm", "install", "-g", package]),
        "cargo" => run_pkg(&["cargo", "install", package]),
        "gem" => run_pkg(&["gem", "install", package]),
        "go" | "goget" => run_pkg(&["go", "install", &format!("{}@latest", package)]),
        "snap" => run_pkg(&["sudo", "snap", "install", package]),
        "brew" => run_pkg(&["brew", "install", package]),
        "yum" => run_pkg(&["sudo", "yum", "install", "-y", package]),
        "dnf" => run_pkg(&["sudo", "dnf", "install", "-y", package]),
        "pacman" => run_pkg(&["sudo", "pacman", "-S", "--noconfirm", package]),
        "conda" => run_pkg(&["conda", "install", "-y", package]),
        other => anyhow::bail!(
            "지원하지 않는 패키지 매니저: '{}'. 지원: apt, pip, npm, cargo, gem, go, snap, brew, yum, dnf, pacman, conda",
            other
        ),
    }
}

/// 패키지 제거
pub fn pkg_remove(manager: &str, package: &str) -> Result<PkgResult> {
    match manager.to_lowercase().as_str() {
        "apt" | "apt-get" => run_pkg(&["sudo", "apt-get", "remove", "-y", package]),
        "pip" | "pip3" => {
            if run_quick("pip3", &["--version"]).is_ok() {
                run_pkg(&["pip3", "uninstall", "-y", package])
            } else {
                run_pkg(&["python3", "-m", "pip", "uninstall", "-y", package])
            }
        }
        "npm" => run_pkg(&["npm", "uninstall", "-g", package]),
        "cargo" => anyhow::bail!("cargo uninstall은 shell 툴을 사용하세요"),
        "gem" => run_pkg(&["gem", "uninstall", package]),
        "snap" => run_pkg(&["sudo", "snap", "remove", package]),
        "brew" => run_pkg(&["brew", "uninstall", package]),
        "yum" => run_pkg(&["sudo", "yum", "remove", "-y", package]),
        "dnf" => run_pkg(&["sudo", "dnf", "remove", "-y", package]),
        "pacman" => run_pkg(&["sudo", "pacman", "-R", "--noconfirm", package]),
        "conda" => run_pkg(&["conda", "remove", "-y", package]),
        other => anyhow::bail!("지원하지 않는 패키지 매니저: '{}'", other),
    }
}

/// 패키지 목록 조회
pub fn pkg_list(manager: &str) -> Result<PkgResult> {
    match manager.to_lowercase().as_str() {
        "apt" | "apt-get" => run_pkg(&["dpkg", "--get-selections"]),
        "pip" | "pip3" => {
            if run_quick("pip3", &["--version"]).is_ok() {
                run_pkg(&["pip3", "list"])
            } else {
                run_pkg(&["python3", "-m", "pip", "list"])
            }
        }
        "npm" => run_pkg(&["npm", "list", "-g", "--depth=0"]),
        "cargo" => run_pkg(&["cargo", "install", "--list"]),
        "gem" => run_pkg(&["gem", "list"]),
        "snap" => run_pkg(&["snap", "list"]),
        "brew" => run_pkg(&["brew", "list"]),
        "conda" => run_pkg(&["conda", "list"]),
        other => anyhow::bail!("지원하지 않는 패키지 매니저: '{}'", other),
    }
}

/// 패키지 검색
pub fn pkg_search(manager: &str, query: &str) -> Result<PkgResult> {
    match manager.to_lowercase().as_str() {
        "apt" | "apt-get" => run_pkg(&["apt-cache", "search", query]),
        "pip" | "pip3" => {
            // pip search는 deprecated, pypi.org JSON API 사용 권장
            Err(anyhow::anyhow!(
                "pip search는 더 이상 지원되지 않습니다.\n\
                 대신 web_search 툴로 'pypi {}' 검색하거나\n\
                 https://pypi.org/search/?q={} 에서 직접 검색하세요.",
                query, query
            ))
        }
        "npm" => run_pkg(&["npm", "search", query]),
        "snap" => run_pkg(&["snap", "find", query]),
        "brew" => run_pkg(&["brew", "search", query]),
        other => anyhow::bail!("지원하지 않는 패키지 매니저: '{}'", other),
    }
}

/// 패키지 업그레이드 (특정 패키지)
pub fn pkg_upgrade(manager: &str, package: &str) -> Result<PkgResult> {
    match manager.to_lowercase().as_str() {
        "apt" | "apt-get" => run_pkg(&["sudo", "apt-get", "install", "--only-upgrade", "-y", package]),
        "pip" | "pip3" => {
            if run_quick("pip3", &["--version"]).is_ok() {
                run_pkg(&["pip3", "install", "--upgrade", package])
            } else {
                run_pkg(&["python3", "-m", "pip", "install", "--upgrade", package])
            }
        }
        "npm" => run_pkg(&["npm", "update", "-g", package]),
        "cargo" => run_pkg(&["cargo", "install", "--force", package]),
        "gem" => run_pkg(&["gem", "update", package]),
        "go" | "goget" => run_pkg(&["go", "install", &format!("{}@latest", package)]),
        "brew" => run_pkg(&["brew", "upgrade", package]),
        "snap" => run_pkg(&["sudo", "snap", "refresh", package]),
        "conda" => run_pkg(&["conda", "update", "-y", package]),
        other => anyhow::bail!("지원하지 않는 패키지 매니저: '{}'", other),
    }
}

/// 패키지 목록 업데이트 (인덱스 갱신)
pub fn pkg_update(manager: &str) -> Result<PkgResult> {
    match manager.to_lowercase().as_str() {
        "apt" | "apt-get" => run_pkg(&["sudo", "apt-get", "update"]),
        "pip" | "pip3" => {
            // pip 자체 업그레이드
            if run_quick("pip3", &["--version"]).is_ok() {
                run_pkg(&["pip3", "install", "--upgrade", "pip"])
            } else {
                run_pkg(&["python3", "-m", "pip", "install", "--upgrade", "pip"])
            }
        }
        "npm" => run_pkg(&["npm", "update"]),
        "brew" => run_pkg(&["brew", "update"]),
        "conda" => run_pkg(&["conda", "update", "-y", "conda"]),
        "cargo" => run_pkg(&["cargo", "update"]),
        "yum" => run_pkg(&["sudo", "yum", "check-update"]),
        "dnf" => run_pkg(&["sudo", "dnf", "check-update"]),
        "pacman" => run_pkg(&["sudo", "pacman", "-Sy"]),
        other => anyhow::bail!("지원하지 않는 패키지 매니저: '{}'", other),
    }
}

/// 시스템 정보
pub fn sysinfo() -> Result<PkgResult> {
    let mut parts = vec![];

    // OS 정보
    if let Ok(os) = std::fs::read_to_string("/etc/os-release") {
        let name = os.lines()
            .find(|l| l.starts_with("PRETTY_NAME="))
            .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"'))
            .unwrap_or("Unknown OS");
        parts.push(format!("OS: {}", name));
    }

    // 호스트명
    if let Ok(hostname) = run_quick("hostname", &[]) {
        parts.push(format!("Hostname: {}", hostname.trim()));
    }

    // CPU
    if let Ok(cpu_info) = std::fs::read_to_string("/proc/cpuinfo") {
        let model = cpu_info.lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim())
            .unwrap_or("Unknown CPU");
        let cores = cpu_info.lines().filter(|l| l.starts_with("processor")).count();
        parts.push(format!("CPU: {} ({} cores)", model, cores));
    }

    // 메모리
    if let Ok(mem_info) = std::fs::read_to_string("/proc/meminfo") {
        let total = mem_info.lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u64>().ok())
            .map(|kb| format!("{} MB", kb / 1024))
            .unwrap_or_else(|| "Unknown".to_string());

        let avail = mem_info.lines()
            .find(|l| l.starts_with("MemAvailable:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u64>().ok())
            .map(|kb| format!("{} MB", kb / 1024))
            .unwrap_or_else(|| "Unknown".to_string());

        parts.push(format!("Memory: {} total, {} available", total, avail));
    }

    // 디스크
    if let Ok(df) = run_quick("df", &["-h", "/"]) {
        let disk_line = df.lines().nth(1).unwrap_or("").trim().to_string();
        if !disk_line.is_empty() {
            parts.push(format!("Disk (/): {}", disk_line));
        }
    }

    // 현재 디렉토리
    if let Ok(cwd) = std::env::current_dir() {
        parts.push(format!("CWD: {}", cwd.display()));
    }

    Ok(PkgResult {
        output: parts.join("\n"),
        success: true,
    })
}

/// 프로세스 목록
pub fn process_list(filter: &str) -> Result<PkgResult> {
    let args = if filter.is_empty() {
        vec!["aux"]
    } else {
        vec!["aux"]
    };
    let output = run_quick("ps", &args)?;
    let lines: Vec<&str> = output.lines()
        .filter(|l| l.to_lowercase().contains(&filter.to_lowercase()) || filter.is_empty())
        .take(50)
        .collect();
    Ok(PkgResult {
        output: lines.join("\n"),
        success: true,
    })
}

// ─── Helpers ────────────────────────────────────────────────────────────────────

fn run_pkg(args: &[&str]) -> Result<PkgResult> {
    let timeout = Duration::from_secs(PKG_TIMEOUT_SECS);
    if args.is_empty() {
        anyhow::bail!("명령어가 비어 있음");
    }

    let program = args[0].to_string();
    let rest: Vec<String> = args[1..].iter().map(|s| s.to_string()).collect();

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = Command::new(&program).args(&rest).output();
        let _ = tx.send(out);
    });

    let output = rx
        .recv_timeout(timeout)
        .with_context(|| format!("타임아웃 ({}초)", PKG_TIMEOUT_SECS))?
        .with_context(|| format!("명령어 실행 실패: {}", args[0]))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined = if output.status.success() {
        if stdout.is_empty() { stderr.clone() } else { stdout }
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    let out_text = if combined.len() > MAX_OUTPUT {
        format!("{}...[잘림]", &combined[..MAX_OUTPUT])
    } else {
        combined
    };

    Ok(PkgResult {
        output: out_text.trim().to_string(),
        success: output.status.success(),
    })
}

fn run_quick(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("실행 실패: {}", program))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
