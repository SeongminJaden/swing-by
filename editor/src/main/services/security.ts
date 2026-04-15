import { ipcMain } from 'electron';
import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

interface SecurityIssue {
  severity: 'high' | 'medium' | 'low' | 'info';
  type: string;
  message: string;
  file?: string;
  line?: number;
}

interface SecurityReport {
  score: number;
  issues: SecurityIssue[];
  scannedFiles: number;
  timestamp: number;
}

// Secret patterns to detect
const SECRET_PATTERNS = [
  { pattern: /(?:api[_-]?key|apikey)\s*[:=]\s*['"][A-Za-z0-9_\-]{16,}['"]/gi, type: 'API Key', severity: 'high' as const },
  { pattern: /(?:secret|password|passwd|pwd)\s*[:=]\s*['"][^'"]{8,}['"]/gi, type: 'Password/Secret', severity: 'high' as const },
  { pattern: /sk-[A-Za-z0-9]{32,}/g, type: 'OpenAI API Key', severity: 'high' as const },
  { pattern: /sk-ant-[A-Za-z0-9\-]{32,}/g, type: 'Anthropic API Key', severity: 'high' as const },
  { pattern: /ghp_[A-Za-z0-9]{36,}/g, type: 'GitHub Token', severity: 'high' as const },
  { pattern: /(?:AKIA|ASIA)[A-Z0-9]{16}/g, type: 'AWS Access Key', severity: 'high' as const },
  { pattern: /(?:mongodb\+srv|mongodb):\/\/[^\s'"]+/g, type: 'MongoDB URI', severity: 'medium' as const },
  { pattern: /postgres(?:ql)?:\/\/[^\s'"]+/g, type: 'PostgreSQL URI', severity: 'medium' as const },
  { pattern: /Bearer\s+[A-Za-z0-9\-._~+/]+=*/g, type: 'Bearer Token', severity: 'medium' as const },
];

// Dangerous patterns
const DANGER_PATTERNS = [
  { pattern: /eval\s*\(/g, type: 'eval() 사용', message: 'eval()은 코드 인젝션에 취약합니다', severity: 'high' as const },
  { pattern: /innerHTML\s*=/g, type: 'innerHTML 사용', message: 'XSS 공격에 취약할 수 있습니다', severity: 'medium' as const },
  { pattern: /dangerouslySetInnerHTML/g, type: 'dangerouslySetInnerHTML', message: 'XSS 공격에 취약할 수 있습니다', severity: 'medium' as const },
  { pattern: /document\.write/g, type: 'document.write', message: 'DOM 조작에 안전하지 않습니다', severity: 'medium' as const },
  { pattern: /exec\s*\(\s*['"`].*\$\{/g, type: 'Command Injection', message: '명령 인젝션에 취약할 수 있습니다', severity: 'high' as const },
  { pattern: /SELECT.*FROM.*WHERE.*\+|SELECT.*FROM.*WHERE.*\$\{/gi, type: 'SQL Injection', message: 'SQL 인젝션에 취약합니다. 파라미터 바인딩을 사용하세요', severity: 'high' as const },
];

const SCAN_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.py', '.go', '.java', '.rb', '.php', '.env']);
const IGNORE_DIRS = new Set(['node_modules', '.git', '.next', 'dist', 'build', '.cache', '__pycache__']);

function scanDirectory(dirPath: string, maxDepth = 5, depth = 0): string[] {
  if (depth > maxDepth) return [];
  const files: string[] = [];

  try {
    const entries = fs.readdirSync(dirPath, { withFileTypes: true });
    for (const entry of entries) {
      if (IGNORE_DIRS.has(entry.name)) continue;
      const fullPath = path.join(dirPath, entry.name);

      if (entry.isDirectory()) {
        files.push(...scanDirectory(fullPath, maxDepth, depth + 1));
      } else if (entry.isFile()) {
        const ext = path.extname(entry.name).toLowerCase();
        if (SCAN_EXTENSIONS.has(ext) || entry.name === '.env' || entry.name === '.env.local') {
          files.push(fullPath);
        }
      }
    }
  } catch {}

  return files;
}

function scanFile(filePath: string): SecurityIssue[] {
  const issues: SecurityIssue[] = [];

  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    const lines = content.split('\n');
    const relativePath = filePath;

    // Secret detection
    for (const { pattern, type, severity } of SECRET_PATTERNS) {
      pattern.lastIndex = 0;
      let match;
      while ((match = pattern.exec(content)) !== null) {
        const lineNum = content.substring(0, match.index).split('\n').length;
        issues.push({
          severity,
          type: '시크릿 노출',
          message: `${type} 감지됨: ${match[0].substring(0, 20)}...`,
          file: relativePath,
          line: lineNum,
        });
      }
    }

    // Dangerous pattern detection
    for (const { pattern, type, message, severity } of DANGER_PATTERNS) {
      pattern.lastIndex = 0;
      let match;
      while ((match = pattern.exec(content)) !== null) {
        const lineNum = content.substring(0, match.index).split('\n').length;
        issues.push({
          severity,
          type,
          message,
          file: relativePath,
          line: lineNum,
        });
      }
    }

    // Check .env files specifically
    if (path.basename(filePath).startsWith('.env')) {
      // Check if .env is in .gitignore
      const gitignorePath = path.join(path.dirname(filePath), '.gitignore');
      if (fs.existsSync(gitignorePath)) {
        const gitignore = fs.readFileSync(gitignorePath, 'utf-8');
        if (!gitignore.includes('.env')) {
          issues.push({
            severity: 'high',
            type: '.env 미보호',
            message: '.env 파일이 .gitignore에 포함되지 않았습니다',
            file: relativePath,
          });
        }
      }
    }
  } catch {}

  return issues;
}

function runEslint(cwd: string): SecurityIssue[] {
  try {
    const result = execSync('npx eslint . --format json --no-error-on-unmatched-pattern 2>/dev/null', {
      cwd,
      encoding: 'utf-8',
      timeout: 30000,
    });

    const parsed = JSON.parse(result);
    const issues: SecurityIssue[] = [];

    for (const file of parsed) {
      for (const msg of file.messages) {
        if (msg.severity >= 2) {
          issues.push({
            severity: 'medium',
            type: 'ESLint',
            message: `${msg.ruleId}: ${msg.message}`,
            file: file.filePath,
            line: msg.line,
          });
        }
      }
    }

    return issues;
  } catch {
    return [];
  }
}

function calculateScore(issues: SecurityIssue[]): number {
  let score = 100;
  for (const issue of issues) {
    if (issue.severity === 'high') score -= 15;
    else if (issue.severity === 'medium') score -= 5;
    else if (issue.severity === 'low') score -= 2;
  }
  return Math.max(0, score);
}

export function registerSecurityHandlers() {
  ipcMain.handle('security:scan', async (_event, cwd: string) => {
    try {
      const files = scanDirectory(cwd);
      const allIssues: SecurityIssue[] = [];

      // Custom scan
      for (const file of files) {
        allIssues.push(...scanFile(file));
      }

      // ESLint
      allIssues.push(...runEslint(cwd));

      // Make paths relative
      for (const issue of allIssues) {
        if (issue.file?.startsWith(cwd)) {
          issue.file = issue.file.substring(cwd.length + 1);
        }
      }

      const report: SecurityReport = {
        score: calculateScore(allIssues),
        issues: allIssues,
        scannedFiles: files.length,
        timestamp: Date.now(),
      };

      return { success: true, data: report };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Quick check for secrets only
  ipcMain.handle('security:checkSecrets', async (_event, content: string) => {
    const issues: string[] = [];
    for (const { pattern, type } of SECRET_PATTERNS) {
      pattern.lastIndex = 0;
      if (pattern.test(content)) {
        issues.push(type);
      }
    }
    return issues;
  });
}
