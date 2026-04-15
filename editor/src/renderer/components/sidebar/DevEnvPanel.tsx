import React, { useState, useEffect, useCallback } from 'react';
import {
  CheckCircle2,
  Download,
  RefreshCw,
  Box,
  Wrench,
  Database,
  Code2,
  Loader2,
} from 'lucide-react';

interface DevTool {
  name: string;
  command: string; // command to check version
  version: string | null;
  installed: boolean;
}

interface DevSection {
  id: string;
  title: string;
  icon: React.ElementType;
  items: DevTool[];
}

// Commands to detect each tool
const toolCommands: { section: string; sectionTitle: string; icon: React.ElementType; tools: { name: string; cmd: string }[] }[] = [
  {
    section: 'runtime',
    sectionTitle: '런타임 & 패키지 매니저',
    icon: Box,
    tools: [
      { name: 'Node.js', cmd: 'node --version' },
      { name: 'Python', cmd: 'python3 --version 2>&1 || python --version 2>&1' },
      { name: 'Go', cmd: 'go version' },
      { name: 'Rust', cmd: 'rustc --version' },
      { name: 'Java', cmd: 'java --version 2>&1 | head -1' },
      { name: 'npm', cmd: 'npm --version' },
      { name: 'pnpm', cmd: 'pnpm --version' },
      { name: 'yarn', cmd: 'yarn --version' },
      { name: 'pip', cmd: 'pip3 --version 2>&1 || pip --version 2>&1' },
      { name: 'Bun', cmd: 'bun --version' },
    ],
  },
  {
    section: 'framework',
    sectionTitle: '프레임워크 CLI',
    icon: Code2,
    tools: [
      { name: 'create-next-app', cmd: 'npx create-next-app --version 2>/dev/null || echo ""' },
      { name: 'Vite', cmd: 'npx vite --version 2>/dev/null' },
      { name: 'Vue CLI', cmd: 'vue --version 2>/dev/null' },
      { name: 'Angular CLI', cmd: 'ng version 2>/dev/null | grep "Angular CLI" | awk \'{print $3}\'' },
      { name: 'Flutter', cmd: 'flutter --version 2>/dev/null | head -1' },
      { name: 'Expo', cmd: 'expo --version 2>/dev/null' },
    ],
  },
  {
    section: 'devtools',
    sectionTitle: '개발 도구',
    icon: Wrench,
    tools: [
      { name: 'Git', cmd: 'git --version' },
      { name: 'Docker', cmd: 'docker --version' },
      { name: 'Docker Compose', cmd: 'docker compose version 2>/dev/null || docker-compose --version 2>/dev/null' },
      { name: 'kubectl', cmd: 'kubectl version --client --short 2>/dev/null || kubectl version --client 2>/dev/null | head -1' },
      { name: 'Terraform', cmd: 'terraform --version 2>/dev/null | head -1' },
      { name: 'AWS CLI', cmd: 'aws --version 2>/dev/null' },
      { name: 'Vercel CLI', cmd: 'vercel --version 2>/dev/null' },
      { name: 'Railway CLI', cmd: 'railway --version 2>/dev/null' },
    ],
  },
  {
    section: 'database',
    sectionTitle: '데이터베이스',
    icon: Database,
    tools: [
      { name: 'PostgreSQL', cmd: 'psql --version 2>/dev/null' },
      { name: 'MySQL', cmd: 'mysql --version 2>/dev/null' },
      { name: 'MongoDB', cmd: 'mongod --version 2>/dev/null | head -1' },
      { name: 'Redis', cmd: 'redis-server --version 2>/dev/null' },
      { name: 'SQLite', cmd: 'sqlite3 --version 2>/dev/null' },
    ],
  },
];

function parseVersion(output: string): string | null {
  if (!output || output.trim() === '') return null;
  // Extract version-like pattern (e.g., v22.22.0, 3.11.2, etc.)
  const match = output.match(/v?(\d+\.\d+[\.\d]*)/);
  return match ? `v${match[1]}` : output.trim().substring(0, 30);
}

export function DevEnvPanel() {
  const [sections, setSections] = useState<DevSection[]>([]);
  const [loading, setLoading] = useState(true);

  const detectAll = useCallback(async () => {
    const api = window.electronAPI;
    if (!api) {
      // Fallback: mock data if no electron API
      setSections(toolCommands.map(s => ({
        id: s.section,
        title: s.sectionTitle,
        icon: s.icon,
        items: s.tools.map(t => ({ name: t.name, command: t.cmd, version: null, installed: false })),
      })));
      setLoading(false);
      return;
    }

    setLoading(true);

    const results: DevSection[] = [];

    for (const sec of toolCommands) {
      const items: DevTool[] = [];

      for (const tool of sec.tools) {
        const result = await api.execCommand(tool.cmd);
        const version = result.success ? parseVersion(result.output) : null;
        items.push({
          name: tool.name,
          command: tool.cmd,
          version,
          installed: result.success && !!version,
        });
      }

      results.push({
        id: sec.section,
        title: sec.sectionTitle,
        icon: sec.icon,
        items,
      });
    }

    setSections(results);
    setLoading(false);
  }, []);

  useEffect(() => {
    detectAll();
  }, [detectAll]);

  const installedCount = sections.reduce((acc, s) => acc + s.items.filter(i => i.installed).length, 0);
  const totalCount = sections.reduce((acc, s) => acc + s.items.length, 0);

  return (
    <div className="devenv-panel">
      {/* Header with refresh */}
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '8px 12px', borderBottom: '1px solid var(--color-border-primary)',
      }}>
        <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--color-text-tertiary)' }}>
          {loading ? '감지 중...' : `${installedCount}/${totalCount} 설치됨`}
        </span>
        <button
          onClick={detectAll}
          disabled={loading}
          style={{
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            width: '24px', height: '24px', borderRadius: '6px',
            color: 'var(--color-text-tertiary)', background: 'none', border: 'none', cursor: 'pointer',
          }}
          title="다시 감지"
        >
          {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
        </button>
      </div>

      {/* Sections */}
      {sections.map((section) => {
        const Icon = section.icon;
        const sectionInstalled = section.items.filter(i => i.installed).length;

        return (
          <div key={section.id} className="devenv-section">
            <div className="devenv-section-header">
              <Icon size={13} style={{ color: 'var(--color-text-tertiary)' }} />
              <span className="devenv-section-title">{section.title}</span>
              <span className="devenv-section-count">{sectionInstalled}/{section.items.length}</span>
            </div>

            <div className="devenv-section-items">
              {section.items.map((tool) => (
                <div
                  key={tool.name}
                  className={`devenv-item ${tool.installed ? 'devenv-item-installed' : 'devenv-item-missing'}`}
                >
                  <div className="devenv-item-left">
                    <span className={`devenv-status-dot ${tool.installed ? 'devenv-status-dot-active' : ''}`} />
                    <span className="devenv-item-name">{tool.name}</span>
                  </div>
                  <div className="devenv-item-right">
                    {tool.installed ? (
                      <span className="devenv-version">{tool.version}</span>
                    ) : (
                      <span className="devenv-version-missing">미설치</span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export default DevEnvPanel;
