use anyhow::Result;
use tracing::instrument;

use crate::tools::code_executor::{run_code, ExecutionResult};

/// 코드 디버그 실행 (오류 분석 포함)
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
            write!(f, "\n\n[디버그 힌트]\n타입: {}\n힌트: {}", analysis.error_type, analysis.hint)?;
        }
        Ok(())
    }
}

/// 오류 메시지에서 타입과 힌트 추출
fn analyze_error(result: &ExecutionResult) -> ErrorAnalysis {
    let combined = format!("{}\n{}", result.stdout, result.stderr);

    // Python 오류
    if combined.contains("SyntaxError") {
        return ErrorAnalysis {
            error_type: "SyntaxError".to_string(),
            hint: "문법 오류입니다. 들여쓰기, 괄호, 콜론을 확인하세요.".to_string(),
        };
    }
    if combined.contains("NameError") {
        return ErrorAnalysis {
            error_type: "NameError".to_string(),
            hint: "정의되지 않은 변수나 함수를 사용했습니다.".to_string(),
        };
    }
    if combined.contains("TypeError") {
        return ErrorAnalysis {
            error_type: "TypeError".to_string(),
            hint: "잘못된 타입의 값을 사용했습니다.".to_string(),
        };
    }
    if combined.contains("IndexError") {
        return ErrorAnalysis {
            error_type: "IndexError".to_string(),
            hint: "리스트/배열의 범위를 벗어난 인덱스입니다.".to_string(),
        };
    }
    if combined.contains("ImportError") || combined.contains("ModuleNotFoundError") {
        return ErrorAnalysis {
            error_type: "ImportError".to_string(),
            hint: "모듈을 찾을 수 없습니다. pip install로 설치하세요.".to_string(),
        };
    }

    // Rust 오류
    if combined.contains("error[E") {
        return ErrorAnalysis {
            error_type: "Rust 컴파일 오류".to_string(),
            hint: "rustc 오류 코드를 확인하고 `rustc --explain E{code}`를 실행해보세요.".to_string(),
        };
    }
    if combined.contains("cannot borrow") {
        return ErrorAnalysis {
            error_type: "Borrow Checker 오류".to_string(),
            hint: "소유권 규칙 위반입니다. 참조(&)와 클론(.clone())을 검토하세요.".to_string(),
        };
    }

    // 일반
    ErrorAnalysis {
        error_type: "런타임 오류".to_string(),
        hint: format!("종료 코드: {}. stderr를 확인하세요.", result.exit_code),
    }
}
