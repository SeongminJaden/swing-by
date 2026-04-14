//! Real-time system monitoring
//!
//! Displays the following information in the CLI status bar:
//!   - Token usage and context utilization
//!   - AI model status (Ollama)
//!   - GPU utilization (nvidia-smi or rocm-smi)
//!   - CPU / memory utilization

use std::time::Duration;

// ─── System stats snapshot ───────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SystemStats {
    pub cpu_pct: f32,         // CPU utilization (%)
    pub mem_used_mb: u64,     // Used memory (MB)
    pub mem_total_mb: u64,    // Total memory (MB)
    pub gpu_pct: Option<f32>, // GPU utilization (%, None if no GPU)
    pub gpu_mem_used_mb: Option<u64>,
    pub gpu_mem_total_mb: Option<u64>,
    pub gpu_name: Option<String>,
}

impl SystemStats {
    /// Collect system stats from /proc/stat and /proc/meminfo
    pub fn collect() -> Self {
        let mut stats = Self::default();
        stats.mem_used_mb = collect_mem_used();
        stats.mem_total_mb = collect_mem_total();
        stats.cpu_pct = collect_cpu_pct();
        let gpu = collect_gpu();
        stats.gpu_pct = gpu.0;
        stats.gpu_mem_used_mb = gpu.1;
        stats.gpu_mem_total_mb = gpu.2;
        stats.gpu_name = gpu.3;
        stats
    }

    /// Generate a single-line status string
    pub fn status_line(&self) -> String {
        let cpu = format!("CPU:{:.0}%", self.cpu_pct);
        let mem = if self.mem_total_mb > 0 {
            let pct = self.mem_used_mb * 100 / self.mem_total_mb.max(1);
            format!("MEM:{:.0}% ({}/{}MB)", pct, self.mem_used_mb, self.mem_total_mb)
        } else {
            String::new()
        };

        let gpu = match (self.gpu_pct, self.gpu_mem_used_mb, self.gpu_mem_total_mb) {
            (Some(pct), Some(used), Some(total)) => {
                format!("GPU:{:.0}% VRAM:{}/{}MB", pct, used, total)
            }
            (Some(pct), _, _) => format!("GPU:{:.0}%", pct),
            _ => String::new(),
        };

        [cpu, mem, gpu].iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

// ─── CPU collection (/proc/stat) ─────────────────────────────────────────────

fn collect_cpu_pct() -> f32 {
    // Read /proc/stat twice to compute delta
    let snap1 = read_cpu_stat();
    std::thread::sleep(Duration::from_millis(100));
    let snap2 = read_cpu_stat();

    if let (Some((idle1, total1)), Some((idle2, total2))) = (snap1, snap2) {
        let idle_delta = idle2.saturating_sub(idle1) as f32;
        let total_delta = total2.saturating_sub(total1) as f32;
        if total_delta > 0.0 {
            return (1.0 - idle_delta / total_delta) * 100.0;
        }
    }
    0.0
}

fn read_cpu_stat() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;  // "cpu  ..."
    let nums: Vec<u64> = line.split_whitespace()
        .skip(1)  // skip "cpu"
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() < 4 { return None; }
    // idle = nums[3], iowait = nums.get(4).copied().unwrap_or(0)
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total: u64 = nums.iter().sum();
    Some((idle, total))
}

// ─── Memory collection (/proc/meminfo) ───────────────────────────────────────

fn parse_meminfo_kb(content: &str, key: &str) -> Option<u64> {
    content.lines()
        .find(|l| l.starts_with(key))?
        .split_whitespace()
        .nth(1)?
        .parse().ok()
}

fn collect_mem_total() -> u64 {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    parse_meminfo_kb(&content, "MemTotal:").unwrap_or(0) / 1024
}

fn collect_mem_used() -> u64 {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let total = parse_meminfo_kb(&content, "MemTotal:").unwrap_or(0);
    let avail = parse_meminfo_kb(&content, "MemAvailable:").unwrap_or(0);
    total.saturating_sub(avail) / 1024
}

// ─── GPU collection (nvidia-smi / rocm-smi) ───────────────────────────────────

fn collect_gpu() -> (Option<f32>, Option<u64>, Option<u64>, Option<String>) {
    // Try NVIDIA first
    if let Some(result) = try_nvidia_smi() {
        return result;
    }
    // Try AMD ROCm
    if let Some(result) = try_rocm_smi() {
        return result;
    }
    (None, None, None, None)
}

fn try_nvidia_smi() -> Option<(Option<f32>, Option<u64>, Option<u64>, Option<String>)> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,memory.used,memory.total,name",
            "--format=csv,noheader,nounits"
        ])
        .output()
        .ok()?;

    if !output.status.success() { return None; }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?;
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 4 { return None; }

    let gpu_pct: f32 = parts[0].parse().ok()?;
    let mem_used: u64 = parts[1].parse().ok()?;
    let mem_total: u64 = parts[2].parse().ok()?;
    let name = parts[3].to_string();

    Some((Some(gpu_pct), Some(mem_used), Some(mem_total), Some(name)))
}

fn try_rocm_smi() -> Option<(Option<f32>, Option<u64>, Option<u64>, Option<String>)> {
    let output = std::process::Command::new("rocm-smi")
        .args(["--showuse", "--showmemuse", "--csv"])
        .output()
        .ok()?;

    if !output.status.success() { return None; }
    let text = String::from_utf8_lossy(&output.stdout);
    // rocm-smi CSV parsing (simple)
    for line in text.lines().skip(1) {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            let gpu_pct: f32 = parts.get(1).and_then(|s| s.trim_end_matches('%').parse().ok()).unwrap_or(0.0);
            return Some((Some(gpu_pct), None, None, Some("AMD GPU".to_string())));
        }
    }
    None
}

// ─── Ollama model status ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub model: String,
    pub running: bool,
    pub vram_mb: Option<u64>,
    pub context_tokens: Option<usize>,
}

/// Parse `ollama ps` output and return information about the currently running model
pub async fn get_model_status(model_name: &str) -> ModelStatus {
    let output = tokio::process::Command::new("ollama")
        .arg("ps")
        .output()
        .await;

    let mut status = ModelStatus {
        model: model_name.to_string(),
        running: false,
        vram_mb: None,
        context_tokens: None,
    };

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().skip(1) {  // skip header
            if line.contains(model_name) {
                status.running = true;
                // Parse "NAME    ID    SIZE    PROCESSOR    UNTIL" format
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(size_str) = parts.get(2) {
                    // Convert "4.2 GB" → MB
                    if let Some(size_mb) = parse_size_to_mb(size_str, parts.get(3).copied()) {
                        status.vram_mb = Some(size_mb);
                    }
                }
                break;
            }
        }
    }

    status
}

fn parse_size_to_mb(num: &str, unit: Option<&str>) -> Option<u64> {
    let n: f64 = num.parse().ok()?;
    let mb = match unit.unwrap_or("").to_uppercase().as_str() {
        "GB" | "GIB" => (n * 1024.0) as u64,
        "MB" | "MIB" => n as u64,
        _ => return None,
    };
    Some(mb)
}

// ─── Real-time status display ─────────────────────────────────────────────────

/// Print status bar for the chat loop
/// Displays current state on a single line and moves cursor to line start (overwritable)
pub fn print_status_bar(
    token_used: usize,
    token_total: usize,
    sys: &SystemStats,
    model: &ModelStatus,
) {
    let ctx_pct = token_used * 100 / token_total.max(1);
    let ctx_bar = make_bar(ctx_pct, 10);

    let model_icon = if model.running { "●" } else { "○" };
    let vram_str = model.vram_mb.map(|m| format!(" {:.1}GB", m as f64 / 1024.0)).unwrap_or_default();

    let sys_str = sys.status_line();

    let line = format!(
        "\x1b[2m[CTX:{}{} {}/{}.  {} {} {}{}\x1b[0m]",
        ctx_bar,
        ctx_pct,
        token_used / 1000,
        token_total / 1000,
        model_icon,
        model.model,
        vram_str,
        if sys_str.is_empty() { String::new() } else { format!("  {}", sys_str) },
    );

    // Truncate to terminal width
    let max_width = terminal_width().min(200);
    let truncated = crate::utils::trunc(&line, max_width);
    print!("{}\r\n", truncated);
}

fn make_bar(pct: usize, width: usize) -> String {
    let filled = (pct * width) / 100;
    let color = if pct >= 90 { "\x1b[31m" } else if pct >= 70 { "\x1b[33m" } else { "\x1b[32m" };
    let bar: String = (0..width).map(|i| if i < filled { '█' } else { '░' }).collect();
    format!("{}{}%\x1b[0m", color, bar)
}

fn terminal_width() -> usize {
    // Get terminal width via ioctl (fallback to default on failure)
    if let Ok(output) = std::process::Command::new("tput").arg("cols").output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Ok(n) = s.trim().parse::<usize>() {
                return n;
            }
        }
    }
    120
}

// ─── Background monitor ───────────────────────────────────────────────────────

/// Periodically collect system stats and store them in a shared Arc
pub fn start_background_monitor(
    interval_ms: u64,
) -> (Arc<std::sync::Mutex<SystemStats>>, tokio::task::JoinHandle<()>) {
    use std::sync::Arc;
    let stats = Arc::new(std::sync::Mutex::new(SystemStats::default()));
    let stats_clone = stats.clone();

    let handle = tokio::task::spawn_blocking(move || {
        loop {
            let new_stats = SystemStats::collect();
            if let Ok(mut guard) = stats_clone.lock() {
                *guard = new_stats;
            }
            std::thread::sleep(Duration::from_millis(interval_ms));
        }
    });

    (stats, handle)
}

use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_stats_not_zero() {
        let stats = SystemStats::collect();
        // On Linux, mem_total_mb should be > 0
        assert!(stats.mem_total_mb > 0, "total memory should be readable");
    }

    #[test]
    fn status_line_not_empty() {
        let stats = SystemStats::collect();
        let line = stats.status_line();
        assert!(!line.is_empty());
        assert!(line.contains("CPU:"));
        assert!(line.contains("MEM:"));
    }

    #[test]
    fn make_bar_100pct() {
        let bar = make_bar(100, 10);
        assert!(bar.contains('█'));
    }

    #[test]
    fn make_bar_0pct() {
        let bar = make_bar(0, 10);
        assert!(bar.contains('░'));
    }
}
