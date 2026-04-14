/// 코드 품질 툴: 린트, 포맷, 테스트 실행
///
/// 언어별 표준 도구를 자동으로 선택하여 실행

use anyhow::Result;
use std::process::Command;
use std::time::Duration;

const QUALITY_TIMEOUT: u64 = 120;
const MAX_OUTPUT: usize = 16_000;

#[derive(Debug)]
pub struct QualityResult {
    pub output: String,
    pub success: bool,
    pub tool: String,
}

impl std::fmt::Display for QualityResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.tool, self.output)
    }
}

// ─── 린트 ────────────────────────────────────────────────────────────────────

/// 코드 품질 검사 (lint)
pub fn lint(language: &str, path: &str) -> Result<QualityResult> {
    let path = if path.is_empty() { "." } else { path };
    match language.to_lowercase().as_str() {
        "rust" | "rs" => lint_rust(path),
        "python" | "py" | "python3" => lint_python(path),
        "javascript" | "js" | "typescript" | "ts" => lint_js(path),
        "go" | "golang" => lint_go(path),
        "c" | "c++" | "cpp" => lint_cpp(path),
        "ruby" | "rb" => lint_ruby(path),
        "php" => lint_php(path),
        other => anyhow::bail!("지원하지 않는 린트 언어: '{}'", other),
    }
}

fn lint_rust(path: &str) -> Result<QualityResult> {
    // cargo clippy 우선, 없으면 cargo check
    let cargo = find_cargo();
    let r = run_quality(&[&cargo, "clippy", "--", "-D", "warnings"], path, "cargo clippy")?;
    if !r.success {
        // clippy 실패 시 check로 폴백
        return run_quality(&[&cargo, "check"], path, "cargo check");
    }
    Ok(r)
}

fn lint_python(path: &str) -> Result<QualityResult> {
    // ruff > flake8 > pylint 순서로 시도
    if cmd_exists("ruff") {
        run_quality(&["ruff", "check", path], ".", "ruff")
    } else if cmd_exists("flake8") {
        run_quality(&["flake8", path, "--max-line-length=100"], ".", "flake8")
    } else if cmd_exists("pylint") {
        run_quality(&["pylint", path], ".", "pylint")
    } else {
        run_quality(&["python3", "-m", "py_compile", path], ".", "py_compile")
    }
}

fn lint_js(path: &str) -> Result<QualityResult> {
    if cmd_exists("eslint") {
        run_quality(&["eslint", path, "--max-warnings=0"], ".", "eslint")
    } else if cmd_exists("biome") {
        run_quality(&["biome", "check", path], ".", "biome")
    } else {
        anyhow::bail!("ESLint 또는 Biome이 설치되지 않음. npm install -g eslint 로 설치")
    }
}

fn lint_go(path: &str) -> Result<QualityResult> {
    if cmd_exists("golangci-lint") {
        run_quality(&["golangci-lint", "run", "./..."], path, "golangci-lint")
    } else {
        run_quality(&["go", "vet", "./..."], path, "go vet")
    }
}

fn lint_cpp(path: &str) -> Result<QualityResult> {
    if cmd_exists("clang-tidy") {
        run_quality(&["clang-tidy", path], ".", "clang-tidy")
    } else if cmd_exists("cppcheck") {
        run_quality(&["cppcheck", "--enable=all", path], ".", "cppcheck")
    } else {
        anyhow::bail!("cppcheck 또는 clang-tidy 설치 필요: sudo apt install cppcheck")
    }
}

fn lint_ruby(path: &str) -> Result<QualityResult> {
    if cmd_exists("rubocop") {
        run_quality(&["rubocop", path], ".", "rubocop")
    } else {
        anyhow::bail!("RuboCop 설치 필요: gem install rubocop")
    }
}

fn lint_php(path: &str) -> Result<QualityResult> {
    if cmd_exists("phpstan") {
        run_quality(&["phpstan", "analyse", path], ".", "phpstan")
    } else if cmd_exists("php") {
        run_quality(&["php", "-l", path], ".", "php -l")
    } else {
        anyhow::bail!("PHP가 설치되지 않음")
    }
}

// ─── 포맷 ────────────────────────────────────────────────────────────────────

/// 코드 자동 포맷팅
pub fn format_code(language: &str, path: &str) -> Result<QualityResult> {
    let path = if path.is_empty() { "." } else { path };
    match language.to_lowercase().as_str() {
        "rust" | "rs" => format_rust(path),
        "python" | "py" | "python3" => format_python(path),
        "javascript" | "js" | "typescript" | "ts" => format_js(path),
        "go" | "golang" => format_go(path),
        "c" | "c++" | "cpp" => format_cpp(path),
        "ruby" | "rb" => format_ruby(path),
        other => anyhow::bail!("지원하지 않는 포맷 언어: '{}'", other),
    }
}

fn format_rust(path: &str) -> Result<QualityResult> {
    let cargo = find_cargo();
    run_quality(&[&cargo, "fmt"], path, "cargo fmt")
}

fn format_python(path: &str) -> Result<QualityResult> {
    if cmd_exists("ruff") {
        run_quality(&["ruff", "format", path], ".", "ruff format")
    } else if cmd_exists("black") {
        run_quality(&["black", path], ".", "black")
    } else if cmd_exists("autopep8") {
        run_quality(&["autopep8", "--in-place", "--recursive", path], ".", "autopep8")
    } else {
        anyhow::bail!("black 또는 ruff 설치 필요: pip install black")
    }
}

fn format_js(path: &str) -> Result<QualityResult> {
    if cmd_exists("prettier") {
        run_quality(&["prettier", "--write", path], ".", "prettier")
    } else if cmd_exists("biome") {
        run_quality(&["biome", "format", "--write", path], ".", "biome format")
    } else {
        anyhow::bail!("Prettier 설치 필요: npm install -g prettier")
    }
}

fn format_go(path: &str) -> Result<QualityResult> {
    run_quality(&["gofmt", "-w", "."], path, "gofmt")
}

fn format_cpp(path: &str) -> Result<QualityResult> {
    if cmd_exists("clang-format") {
        run_quality(&["clang-format", "-i", path], ".", "clang-format")
    } else {
        anyhow::bail!("clang-format 설치 필요: sudo apt install clang-format")
    }
}

fn format_ruby(path: &str) -> Result<QualityResult> {
    if cmd_exists("rubocop") {
        run_quality(&["rubocop", "-a", path], ".", "rubocop -a")
    } else {
        anyhow::bail!("RuboCop 설치 필요: gem install rubocop")
    }
}

// ─── 테스트 ───────────────────────────────────────────────────────────────────

/// 테스트 실행
pub fn run_tests(language: &str, path: &str, filter: &str) -> Result<QualityResult> {
    let path = if path.is_empty() { "." } else { path };
    match language.to_lowercase().as_str() {
        "rust" | "rs" => test_rust(path, filter),
        "python" | "py" | "python3" => test_python(path, filter),
        "javascript" | "js" | "node" => test_js(path, filter),
        "typescript" | "ts" => test_ts(path, filter),
        "go" | "golang" => test_go(path, filter),
        "java" => test_java(path, filter),
        other => {
            // shell로 test 명령 시도
            let cmd = if filter.is_empty() {
                format!("cd '{}' && {} test", path, other)
            } else {
                format!("cd '{}' && {} test {}", path, other, filter)
            };
            crate::tools::system::run_shell(&cmd)
                .map(|r| QualityResult { output: r.to_string(), success: r.success, tool: other.to_string() })
        }
    }
}

fn test_rust(path: &str, filter: &str) -> Result<QualityResult> {
    let cargo = find_cargo();
    if filter.is_empty() {
        run_quality(&[&cargo, "test", "--", "--nocapture"], path, "cargo test")
    } else {
        run_quality(&[&cargo, "test", filter, "--", "--nocapture"], path, "cargo test")
    }
}

fn test_python(path: &str, filter: &str) -> Result<QualityResult> {
    if cmd_exists("pytest") {
        if filter.is_empty() {
            run_quality(&["pytest", "-v", path], ".", "pytest")
        } else {
            run_quality(&["pytest", "-v", "-k", filter, path], ".", "pytest")
        }
    } else {
        run_quality(&["python3", "-m", "unittest", "discover", "-v"], path, "unittest")
    }
}

fn test_js(path: &str, _filter: &str) -> Result<QualityResult> {
    if cmd_exists("jest") {
        run_quality(&["jest", "--passWithNoTests"], path, "jest")
    } else {
        run_quality(&["npm", "test"], path, "npm test")
    }
}

fn test_ts(path: &str, filter: &str) -> Result<QualityResult> {
    test_js(path, filter)
}

fn test_go(path: &str, filter: &str) -> Result<QualityResult> {
    if filter.is_empty() {
        run_quality(&["go", "test", "-v", "./..."], path, "go test")
    } else {
        run_quality(&["go", "test", "-v", "-run", filter, "./..."], path, "go test")
    }
}

fn test_java(path: &str, _filter: &str) -> Result<QualityResult> {
    if std::path::Path::new(&format!("{}/pom.xml", path)).exists() {
        run_quality(&["mvn", "test", "-q"], path, "mvn test")
    } else if std::path::Path::new(&format!("{}/build.gradle", path)).exists() {
        run_quality(&["./gradlew", "test"], path, "gradle test")
    } else {
        anyhow::bail!("Maven pom.xml 또는 Gradle build.gradle 없음")
    }
}

// ─── 빌드 ────────────────────────────────────────────────────────────────────

/// 프로젝트 빌드
pub fn build_project(language: &str, path: &str) -> Result<QualityResult> {
    let path = if path.is_empty() { "." } else { path };
    match language.to_lowercase().as_str() {
        "rust" | "rs" => {
            let cargo = find_cargo();
            run_quality(&[&cargo, "build", "--release"], path, "cargo build")
        }
        "go" | "golang" => run_quality(&["go", "build", "./..."], path, "go build"),
        "node" | "js" | "typescript" | "ts" => run_quality(&["npm", "run", "build"], path, "npm build"),
        "python" | "py" => {
            // Python: 문법 체크만
            run_quality(&["python3", "-m", "compileall", "."], path, "python compile")
        }
        "java" => {
            if std::path::Path::new(&format!("{}/pom.xml", path)).exists() {
                run_quality(&["mvn", "package", "-q", "-DskipTests"], path, "mvn package")
            } else {
                run_quality(&["./gradlew", "build"], path, "gradle build")
            }
        }
        "c" => run_quality(&["make"], path, "make"),
        "c++" | "cpp" => run_quality(&["make"], path, "make"),
        other => {
            anyhow::bail!("지원하지 않는 빌드 언어: '{}'", other)
        }
    }
}

// ─── 환경 관리 ────────────────────────────────────────────────────────────────

/// Python 가상환경 생성
pub fn create_venv(path: &str, name: &str) -> Result<QualityResult> {
    let venv_name = if name.is_empty() { ".venv" } else { name };
    let target = if path.is_empty() {
        venv_name.to_string()
    } else {
        format!("{}/{}", path, venv_name)
    };
    run_quality(&["python3", "-m", "venv", &target], ".", "python venv")
}

/// Node 버전 관리 (nvm 사용)
pub fn nvm_use(version: &str) -> Result<QualityResult> {
    let nvm_dir = std::env::var("NVM_DIR")
        .unwrap_or_else(|_| format!("{}/.nvm", std::env::var("HOME").unwrap_or_default()));
    let cmd = format!(
        "export NVM_DIR='{}' && [ -s \"$NVM_DIR/nvm.sh\" ] && . \"$NVM_DIR/nvm.sh\" && nvm use {}",
        nvm_dir, version
    );
    crate::tools::system::run_shell(&cmd)
        .map(|r| QualityResult { output: r.to_string(), success: r.success, tool: "nvm".to_string() })
}

// ─── Helpers ─────────────────────────────────────────────────────────────────────

fn run_quality(args: &[&str], cwd: &str, tool_name: &str) -> Result<QualityResult> {
    let timeout = Duration::from_secs(QUALITY_TIMEOUT);
    let program = args[0].to_string();
    let rest: Vec<String> = args[1..].iter().map(|s| s.to_string()).collect();
    let cwd_owned = if cwd.is_empty() || cwd == "." { ".".to_string() } else { cwd.to_string() };
    let tool_owned = tool_name.to_string();

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = Command::new(&program)
            .args(&rest)
            .current_dir(&cwd_owned)
            .output();
        let _ = tx.send(out);
    });

    let output = rx.recv_timeout(timeout)
        .map_err(|_| anyhow::anyhow!("타임아웃 ({}초): {}", QUALITY_TIMEOUT, tool_name))?
        .map_err(|e| anyhow::anyhow!("실행 실패 {}: {}", tool_name, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    let combined = if stdout.is_empty() { stderr } else if stderr.is_empty() { stdout }
        else { format!("{}\n[stderr]\n{}", stdout, stderr) };

    let out_text = if combined.len() > MAX_OUTPUT {
        format!("{}...[잘림]", crate::utils::trunc(&combined, MAX_OUTPUT))
    } else {
        combined.trim().to_string()
    };

    Ok(QualityResult { output: out_text, success, tool: tool_owned })
}

fn cmd_exists(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn find_cargo() -> String {
    if cmd_exists("cargo") { return "cargo".to_string(); }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let p = format!("{}/.cargo/bin/cargo", home);
    if std::path::Path::new(&p).exists() { return p; }
    "cargo".to_string()
}
