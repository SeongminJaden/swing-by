/**
 * AI 응답에서 코드 블록을 파싱하여 파일 경로와 코드를 추출합니다.
 *
 * 지원하는 형식:
 * 1) ```filepath:src/app/page.tsx\n...code...\n```
 * 2) ```tsx title="src/app/page.tsx"\n...code...\n```
 * 3) 코드 블록 첫 줄에 "// file: src/app/page.tsx" 또는 "# file: ..."
 */

export interface ParsedCodeBlock {
  filePath: string;
  code: string;
  language: string;
}

/**
 * AI 응답 텍스트에서 파일 경로가 포함된 코드 블록을 모두 파싱합니다.
 */
export function parseCodeBlocks(text: string): ParsedCodeBlock[] {
  const results: ParsedCodeBlock[] = [];

  // 정규식: ``` 뒤에 언어/메타 정보가 올 수 있고, 코드 내용, 그리고 닫는 ```
  const codeBlockRegex = /```([^\n]*)\n([\s\S]*?)```/g;

  let match: RegExpExecArray | null;
  while ((match = codeBlockRegex.exec(text)) !== null) {
    const meta = match[1].trim();
    let code = match[2];
    let filePath: string | null = null;
    let language = '';

    // 패턴 1: ```filepath:src/app/page.tsx
    const filepathPrefixMatch = meta.match(/^filepath:(.+)$/i);
    if (filepathPrefixMatch) {
      filePath = filepathPrefixMatch[1].trim();
      language = inferLanguage(filePath);
    }

    // 패턴 2: ```tsx title="src/app/page.tsx"
    if (!filePath) {
      const titleMatch = meta.match(/^(\w+)\s+title=["']([^"']+)["']/);
      if (titleMatch) {
        language = titleMatch[1];
        filePath = titleMatch[2].trim();
      }
    }

    // 패턴 2b: ```tsx file="src/app/page.tsx"
    if (!filePath) {
      const fileAttrMatch = meta.match(/^(\w+)\s+file=["']([^"']+)["']/);
      if (fileAttrMatch) {
        language = fileAttrMatch[1];
        filePath = fileAttrMatch[2].trim();
      }
    }

    // 패턴 3: 코드 블록 첫 줄에 "// file: src/app/page.tsx" 또는 "# file: ..."
    if (!filePath) {
      const lines = code.split('\n');
      if (lines.length > 0) {
        const firstLine = lines[0].trim();
        const fileCommentMatch = firstLine.match(
          /^(?:\/\/|#|\/\*|<!--)\s*file:\s*(.+?)(?:\s*\*\/|\s*-->)?$/i
        );
        if (fileCommentMatch) {
          filePath = fileCommentMatch[1].trim();
          // 첫 줄(파일 경로 주석)을 코드에서 제거
          code = lines.slice(1).join('\n');
          language = meta || inferLanguage(filePath);
        }
      }
    }

    // 언어가 아직 비어있으면 meta에서 추출
    if (!language && meta && !meta.includes(' ')) {
      language = meta;
    }

    // filePath가 있는 경우에만 결과에 추가
    if (filePath) {
      // 앞뒤 빈 줄 정리
      code = code.replace(/^\n+/, '').replace(/\n+$/, '');
      // 끝에 newline 하나 보장
      if (!code.endsWith('\n')) {
        code += '\n';
      }
      results.push({
        filePath,
        code,
        language: language || inferLanguage(filePath),
      });
    }
  }

  return results;
}

/**
 * 파일 경로에서 언어를 추론합니다.
 */
function inferLanguage(filePath: string): string {
  const ext = filePath.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    ts: 'typescript',
    tsx: 'typescriptreact',
    js: 'javascript',
    jsx: 'javascriptreact',
    json: 'json',
    md: 'markdown',
    css: 'css',
    scss: 'scss',
    less: 'less',
    html: 'html',
    xml: 'xml',
    svg: 'xml',
    py: 'python',
    rb: 'ruby',
    go: 'go',
    rs: 'rust',
    java: 'java',
    kt: 'kotlin',
    c: 'c',
    cpp: 'cpp',
    h: 'c',
    sh: 'shell',
    bash: 'shell',
    zsh: 'shell',
    yml: 'yaml',
    yaml: 'yaml',
    toml: 'toml',
    sql: 'sql',
    graphql: 'graphql',
    dockerfile: 'dockerfile',
    env: 'plaintext',
    gitignore: 'plaintext',
  };
  return map[ext] || 'plaintext';
}

/**
 * AI 응답에 파일 경로가 포함된 코드 블록이 있는지 빠르게 확인합니다.
 */
export function hasCodeBlocksWithFiles(text: string): boolean {
  // 빠른 휴리스틱 체크
  if (!text.includes('```')) return false;
  const blocks = parseCodeBlocks(text);
  return blocks.length > 0;
}
