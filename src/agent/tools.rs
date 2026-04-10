use anyhow::Result;

use crate::models::ToolCall;
use crate::tools::{
    debug_code, list_dir, read_file, run_code, run_shell, write_file,
};

/// 툴 실행 결과
#[derive(Debug)]
pub struct ToolResult {
    pub tool_name: String,
    pub output: String,
    pub success: bool,
}

impl ToolResult {
    pub fn ok(name: impl Into<String>, output: impl Into<String>) -> Self {
        Self { tool_name: name.into(), output: output.into(), success: true }
    }

    pub fn err(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self { tool_name: name.into(), output: error.into(), success: false }
    }
}

/// 툴 설명 목록 (시스템 프롬프트에 포함)
pub fn tool_descriptions() -> &'static str {
    r#"당신은 다음 툴을 사용할 수 있습니다. 툴을 사용하려면 다음 형식으로 응답하세요:

TOOL: <툴이름> <인자1> <인자2> ...

사용 가능한 툴:
- TOOL: read_file <경로>               # 파일 읽기
- TOOL: write_file <경로> <내용>       # 파일 쓰기 (내용에 공백 포함 시 따옴표 사용)
- TOOL: list_dir <경로>                # 디렉토리 목록
- TOOL: run_code <언어> <코드>         # 코드 실행 (python/javascript/rust)
- TOOL: debug_code <언어> <코드>       # 코드 디버그 실행
- TOOL: shell <명령어>                 # 셸 명령어 실행

툴이 필요하지 않으면 일반 텍스트로 답변하세요.
대화를 끝내려면 EXIT 라고 답변하세요."#
}

/// ToolCall을 실제로 실행
pub async fn dispatch_tool(call: &ToolCall) -> ToolResult {
    let result: Result<String> = match call.name.as_str() {
        "read_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            read_file(path).map(|c| format!("파일 내용 ({}):\n{}", path, c))
        }
        "write_file" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let content = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            write_file(path, content).map(|_| format!("파일 저장 완료: {}", path))
        }
        "list_dir" => {
            let path = call.args.first().map(|s| s.as_str()).unwrap_or(".");
            list_dir(path).map(|items| items.join("\n"))
        }
        "run_code" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let code = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            run_code(lang, code).map(|r| r.to_string())
        }
        "debug_code" => {
            let lang = call.args.first().map(|s| s.as_str()).unwrap_or("");
            let code = call.args.get(1).map(|s| s.as_str()).unwrap_or("");
            debug_code(lang, code).map(|r| r.to_string())
        }
        "shell" => {
            let cmd = call.args.join(" ");
            run_shell(&cmd).map(|r| r.to_string())
        }
        unknown => {
            return ToolResult::err(unknown, format!("알 수 없는 툴: {}", unknown));
        }
    };

    match result {
        Ok(output) => ToolResult::ok(&call.name, output),
        Err(e) => ToolResult::err(&call.name, format!("오류: {}", e)),
    }
}
