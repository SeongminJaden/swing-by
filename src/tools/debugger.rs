use anyhow::Result;
use tracing::instrument;

use crate::tools::code_executor::{run_code, ExecutionResult};

/// Run code with debug analysis (includes error analysis)
#[instrument(skip(code))]
pub fn debug_code(language: &str, code: &str) -> Result<DebugResult> {
    let exec = run_code(language, code)?;

    let analysis = if exec.success {
        None
    } else {
        Some(analyze_error(&exec))
    };

    Ok(DebugResult { exec, analysis })
}

#[derive(Debug)]
pub struct DebugResult {
    pub exec: ExecutionResult,
    pub analysis: Option<ErrorAnalysis>,
}

#[derive(Debug)]
pub struct ErrorAnalysis {
    pub error_type: String,
    pub hint: String,
}

impl std::fmt::Display for DebugResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.exec)?;
        if let Some(analysis) = &self.analysis {
            write!(f, "\n\n[Debug hint]\nType: {}\nHint: {}", analysis.error_type, analysis.hint)?;
        }
        Ok(())
    }
}

/// Extract error type and hint from error message
fn analyze_error(result: &ExecutionResult) -> ErrorAnalysis {
    let combined = format!("{}\n{}", result.stdout, result.stderr);

    // Python errors
    if combined.contains("SyntaxError") {
        return ErrorAnalysis {
            error_type: "SyntaxError".to_string(),
            hint: "Syntax error. Check indentation, brackets, and colons.".to_string(),
        };
    }
    if combined.contains("NameError") {
        return ErrorAnalysis {
            error_type: "NameError".to_string(),
            hint: "Undefined variable or function used.".to_string(),
        };
    }
    if combined.contains("TypeError") {
        return ErrorAnalysis {
            error_type: "TypeError".to_string(),
            hint: "Wrong type of value used.".to_string(),
        };
    }
    if combined.contains("IndexError") {
        return ErrorAnalysis {
            error_type: "IndexError".to_string(),
            hint: "Index out of range for list/array.".to_string(),
        };
    }
    if combined.contains("ImportError") || combined.contains("ModuleNotFoundError") {
        return ErrorAnalysis {
            error_type: "ImportError".to_string(),
            hint: "Module not found. Install it with pip install.".to_string(),
        };
    }

    // Rust errors
    if combined.contains("error[E") {
        return ErrorAnalysis {
            error_type: "Rust compile error".to_string(),
            hint: "Check the rustc error code and run `rustc --explain E{code}`.".to_string(),
        };
    }
    if combined.contains("cannot borrow") {
        return ErrorAnalysis {
            error_type: "Borrow checker error".to_string(),
            hint: "Ownership rule violation. Review references (&) and clones (.clone()).".to_string(),
        };
    }

    // General
    ErrorAnalysis {
        error_type: "Runtime error".to_string(),
        hint: format!("Exit code: {}. Check stderr.", result.exit_code),
    }
}
