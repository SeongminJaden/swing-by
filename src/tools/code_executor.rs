use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;
use tempfile::NamedTempFile;
use tracing::instrument;

const EXEC_TIMEOUT_SECS: u64 = 60;
const MAX_OUTPUT: usize = 16_000;

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
            if self.stdout.is_empty() && !self.stderr.is_empty() {
                write!(f, "{}", self.stderr)
            } else {
                write!(f, "{}", self.stdout)
            }
        } else {
            let mut parts = vec![];
            if !self.stdout.is_empty() { parts.push(self.stdout.clone()); }
            if !self.stderr.is_empty() { parts.push(format!("[stderr] {}", self.stderr)); }
            write!(f, "오류 (종료코드 {}):\n{}", self.exit_code, parts.join("\n"))
        }
    }
}

// ─── 언어별 실행 ──────────────────────────────────────────────────────────────

#[instrument(skip(code))]
pub fn run_python(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".py").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("python3", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_javascript(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".js").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("node", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_typescript(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".ts").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    // ts-node 우선, 없으면 deno
    if which("ts-node") {
        run_command_timeout("ts-node", &[tmp.path().to_str().unwrap()])
    } else if which("deno") {
        run_command_timeout("deno", &["run", tmp.path().to_str().unwrap()])
    } else {
        anyhow::bail!("TypeScript 실행기를 찾을 수 없습니다. npm install -g ts-node 또는 deno 설치 필요")
    }
}

#[instrument(skip(code))]
pub fn run_rust(code: &str) -> Result<ExecutionResult> {
    let tmp_dir = tempfile::tempdir().context("임시 디렉토리 생성 실패")?;
    let src_path = tmp_dir.path().join("main.rs");
    let bin_path = tmp_dir.path().join("main");
    std::fs::write(&src_path, code)?;

    let compile = run_command_timeout(
        "rustc",
        &[src_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap()],
    )?;

    if !compile.success {
        return Ok(ExecutionResult {
            stdout: String::new(),
            stderr: format!("컴파일 오류:\n{}", compile.stderr),
            exit_code: compile.exit_code,
            success: false,
        });
    }
    run_command_timeout(bin_path.to_str().unwrap(), &[])
}

#[instrument(skip(code))]
pub fn run_go(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".go").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("go", &["run", tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_bash(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".sh").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("bash", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_ruby(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".rb").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("ruby", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_php(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".php").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("php", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_perl(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".pl").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("perl", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_lua(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".lua").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("lua", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_r(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".R").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("Rscript", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_java(code: &str) -> Result<ExecutionResult> {
    let tmp_dir = tempfile::tempdir().context("임시 디렉토리 생성 실패")?;

    // 클래스 이름 추출 (public class XXX)
    let class_name = extract_java_class(code).unwrap_or_else(|| "Main".to_string());
    let src_path = tmp_dir.path().join(format!("{}.java", class_name));
    std::fs::write(&src_path, code)?;

    let compile = run_command_timeout(
        "javac",
        &[src_path.to_str().unwrap()],
    )?;

    if !compile.success {
        return Ok(ExecutionResult {
            stdout: String::new(),
            stderr: format!("컴파일 오류:\n{}", compile.stderr),
            exit_code: compile.exit_code,
            success: false,
        });
    }

    // java -cp <dir> <class>
    let dir_str = tmp_dir.path().to_str().unwrap();
    run_command_timeout("java", &["-cp", dir_str, &class_name])
}

#[instrument(skip(code))]
pub fn run_c(code: &str) -> Result<ExecutionResult> {
    let tmp_dir = tempfile::tempdir().context("임시 디렉토리 생성 실패")?;
    let src_path = tmp_dir.path().join("main.c");
    let bin_path = tmp_dir.path().join("main");
    std::fs::write(&src_path, code)?;

    let compile = run_command_timeout(
        "gcc",
        &[src_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap(), "-lm"],
    )?;

    if !compile.success {
        return Ok(ExecutionResult {
            stdout: String::new(),
            stderr: format!("컴파일 오류:\n{}", compile.stderr),
            exit_code: compile.exit_code,
            success: false,
        });
    }
    run_command_timeout(bin_path.to_str().unwrap(), &[])
}

#[instrument(skip(code))]
pub fn run_cpp(code: &str) -> Result<ExecutionResult> {
    let tmp_dir = tempfile::tempdir().context("임시 디렉토리 생성 실패")?;
    let src_path = tmp_dir.path().join("main.cpp");
    let bin_path = tmp_dir.path().join("main");
    std::fs::write(&src_path, code)?;

    let compile = run_command_timeout(
        "g++",
        &[src_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap(), "-std=c++17", "-lm"],
    )?;

    if !compile.success {
        return Ok(ExecutionResult {
            stdout: String::new(),
            stderr: format!("컴파일 오류:\n{}", compile.stderr),
            exit_code: compile.exit_code,
            success: false,
        });
    }
    run_command_timeout(bin_path.to_str().unwrap(), &[])
}

#[instrument(skip(code))]
pub fn run_kotlin(code: &str) -> Result<ExecutionResult> {
    let tmp_dir = tempfile::tempdir().context("임시 디렉토리 생성 실패")?;
    let src_path = tmp_dir.path().join("main.kt");
    let jar_path = tmp_dir.path().join("main.jar");
    std::fs::write(&src_path, code)?;

    let compile = run_command_timeout(
        "kotlinc",
        &[src_path.to_str().unwrap(), "-include-runtime", "-d", jar_path.to_str().unwrap()],
    )?;

    if !compile.success {
        return Ok(ExecutionResult {
            stdout: String::new(),
            stderr: format!("컴파일 오류:\n{}", compile.stderr),
            exit_code: compile.exit_code,
            success: false,
        });
    }
    run_command_timeout("java", &["-jar", jar_path.to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_swift(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".swift").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    run_command_timeout("swift", &[tmp.path().to_str().unwrap()])
}

#[instrument(skip(code))]
pub fn run_scala(code: &str) -> Result<ExecutionResult> {
    let tmp = NamedTempFile::with_suffix(".sc").context("임시 파일 생성 실패")?;
    std::fs::write(tmp.path(), code)?;
    if which("scala-cli") {
        run_command_timeout("scala-cli", &["run", tmp.path().to_str().unwrap()])
    } else {
        run_command_timeout("scala", &[tmp.path().to_str().unwrap()])
    }
}

// ─── 통합 실행 ───────────────────────────────────────────────────────────────

/// 언어를 감지하여 코드 실행
pub fn run_code(language: &str, code: &str) -> Result<ExecutionResult> {
    match language.to_lowercase().trim() {
        "python" | "python3" | "py" => run_python(code),
        "javascript" | "js" | "node" => run_javascript(code),
        "typescript" | "ts" => run_typescript(code),
        "rust" | "rs" => run_rust(code),
        "go" | "golang" => run_go(code),
        "bash" | "sh" | "shell" | "zsh" => run_bash(code),
        "ruby" | "rb" => run_ruby(code),
        "php" => run_php(code),
        "perl" | "pl" => run_perl(code),
        "lua" => run_lua(code),
        "r" | "rscript" => run_r(code),
        "java" => run_java(code),
        "c" => run_c(code),
        "c++" | "cpp" | "cxx" => run_cpp(code),
        "kotlin" | "kt" => run_kotlin(code),
        "swift" => run_swift(code),
        "scala" => run_scala(code),
        other => {
            // 인터프리터가 있으면 직접 시도
            if which(other) {
                let tmp = NamedTempFile::with_suffix(&format!(".{}", other))
                    .context("임시 파일 생성 실패")?;
                std::fs::write(tmp.path(), code)?;
                run_command_timeout(other, &[tmp.path().to_str().unwrap()])
            } else {
                anyhow::bail!(
                    "지원하지 않는 언어: '{}'. 지원 언어: python, javascript, typescript, rust, go, bash, ruby, php, perl, lua, r, java, c, c++, kotlin, swift, scala",
                    other
                )
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────────

fn which(program: &str) -> bool {
    Command::new("which")
        .arg(program)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn extract_java_class(code: &str) -> Option<String> {
    // "public class Foo" 패턴 찾기
    let re = regex::Regex::new(r"public\s+class\s+(\w+)").ok()?;
    re.captures(code)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// 타임아웃 있는 명령어 실행
fn run_command_timeout(program: &str, args: &[&str]) -> Result<ExecutionResult> {
    let timeout = Duration::from_secs(EXEC_TIMEOUT_SECS);
    let program_owned = program.to_string();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let output = Command::new(&program_owned).args(&args_owned).output();
        let _ = tx.send(output);
    });

    let output = rx
        .recv_timeout(timeout)
        .with_context(|| format!("실행 타임아웃 ({}초): {}", EXEC_TIMEOUT_SECS, program))?
        .with_context(|| format!("명령어 실행 실패: {}", program))?;

    let stdout = truncate(String::from_utf8_lossy(&output.stdout).to_string());
    let stderr = truncate(String::from_utf8_lossy(&output.stderr).to_string());

    Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}

fn truncate(s: String) -> String {
    if s.len() > MAX_OUTPUT {
        let cut = crate::utils::trunc(&s, MAX_OUTPUT);
        format!("{}\n[출력 잘림: 총 {}바이트]", cut, s.len())
    } else {
        s
    }
}
