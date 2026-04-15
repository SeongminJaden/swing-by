import React, { useState, lazy, Suspense } from 'react';
import {
  AlertTriangle,
  Bug,
  CircleX,
  Globe,
  Lock,
  Terminal as TerminalIcon,
  Loader2,
} from 'lucide-react';
import Tabs from '../common/Tabs';
import type { Tab } from '../common/Tabs';

const TerminalPanel = lazy(() => import('../terminal/TerminalPanel'));

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

interface DebugPanelProps {
  className?: string;
}

/* ------------------------------------------------------------------ */
/*  Tab definitions                                                    */
/* ------------------------------------------------------------------ */

const tabs: Tab[] = [
  { id: 'console', label: 'Console', icon: Bug },
  { id: 'network', label: 'Network', icon: Globe },
  { id: 'problems', label: 'Problems', icon: AlertTriangle },
  { id: 'terminal', label: 'Terminal', icon: TerminalIcon },
];

/* ------------------------------------------------------------------ */
/*  Console tab                                                        */
/* ------------------------------------------------------------------ */

interface LogEntry {
  id: string;
  time: string;
  level: 'info' | 'warn' | 'error';
  message: string;
  stack?: string;
}

const consoleLogs: LogEntry[] = [];

const levelBadgeClasses: Record<string, string> = {
  info: 'debug-log-info',
  warn: 'debug-log-warn',
  error: 'debug-log-error',
};

const levelMessageClasses: Record<string, string> = {
  info: 'debug-log-message-info',
  warn: 'debug-log-message-warn',
  error: 'debug-log-message-error',
};

function ConsoleTab() {
  return (
    <div className="debug-console">
      {consoleLogs.length === 0 ? (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--color-text-tertiary)', fontSize: '13px' }}>
          콘솔 출력이 없습니다. 프로젝트를 실행하면 로그가 표시됩니다.
        </div>
      ) : (
        consoleLogs.map((log) => (
          <div key={log.id} className="group">
            <div className="debug-log-entry">
              <span className="debug-log-time">{log.time}</span>
              <span className={`debug-log-badge ${levelBadgeClasses[log.level]}`}>
                {log.level}
              </span>
              <span className={levelMessageClasses[log.level]}>
                {log.message}
              </span>
            </div>

            {log.stack && (
              <div className="debug-log-stack">
                <pre className="whitespace-pre-wrap">{log.stack}</pre>
                <button className="debug-ai-fix-btn">
                  🤖 AI에게 수정 요청
                </button>
              </div>
            )}
          </div>
        ))
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Network tab                                                        */
/* ------------------------------------------------------------------ */

interface NetworkEntry {
  id: string;
  method: string;
  path: string;
  status: number;
  time: string;
  size: string;
}

const networkEntries: NetworkEntry[] = [];

const methodColors: Record<string, string> = {
  GET: 'text-accent-success',
  POST: 'text-accent-primary',
  PUT: 'text-accent-warning',
  DELETE: 'text-accent-error',
};

function NetworkTab() {
  return (
    <div className="debug-network">
      {networkEntries.length === 0 ? (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--color-text-tertiary)', fontSize: '13px' }}>
          네트워크 요청이 없습니다. 프로젝트를 실행하면 요청이 표시됩니다.
        </div>
      ) : (
        <>
          <div className="debug-network-header">
            <span>Method</span>
            <span>Path</span>
            <span>Status</span>
            <span>Time</span>
            <span>Size</span>
          </div>
          {networkEntries.map((entry) => {
            const isError = entry.status >= 400;
            return (
              <div key={entry.id} className="group">
                <div
                  className={`debug-network-row ${isError ? 'debug-network-row-error' : ''}`}
                >
                  <span className={`font-semibold ${methodColors[entry.method] ?? 'text-text-primary'}`}>
                    {entry.method}
                  </span>
                  <span className="text-text-primary truncate">{entry.path}</span>
                  <span className={isError ? 'text-accent-error font-semibold' : 'text-accent-success'}>
                    {entry.status}
                  </span>
                  <span className={`text-text-secondary ${isError ? 'text-accent-warning' : ''}`}>
                    {entry.time}
                  </span>
                  <span className="text-text-secondary">{entry.size}</span>
                </div>
                {isError && (
                  <div className="debug-network-error-actions">
                    <button className="debug-ai-fix-btn-error">
                      🤖 AI에게 원인 분석 요청
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Problems tab                                                       */
/* ------------------------------------------------------------------ */

interface Problem {
  id: string;
  type: 'warning' | 'error' | 'security';
  file: string;
  line: number;
  message: string;
}

const problems: Problem[] = [];

const problemIcons: Record<string, React.ReactNode> = {
  warning: <AlertTriangle size={14} className="text-accent-warning" />,
  error: <CircleX size={14} className="text-accent-error" />,
  security: <Lock size={14} className="text-accent-warning" />,
};

const problemPrefixes: Record<string, string> = {
  warning: '\u26A0',
  error: '\u274C',
  security: '\uD83D\uDD12',
};

function ProblemsTab() {
  return (
    <div className="debug-problems">
      {problems.length === 0 ? (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--color-text-tertiary)', fontSize: '13px' }}>
          발견된 문제가 없습니다.
        </div>
      ) : (
        <>
          {problems.map((p) => (
            <div key={p.id} className="debug-problem-item group">
              <span className="shrink-0 mt-0.5">{problemIcons[p.type]}</span>
              <div className="flex-1 min-w-0">
                <div className="flex items-baseline gap-2">
                  <span className="text-text-secondary font-mono">
                    {p.file}:{p.line}
                  </span>
                  <span className="text-text-primary">{p.message}</span>
                </div>
                {p.type === 'security' && (
                  <button className="debug-ai-fix-btn">
                    🤖 AI 자동 수정
                  </button>
                )}
              </div>
              <span className="text-text-tertiary text-[10px] shrink-0 uppercase">
                {problemPrefixes[p.type]} {p.type}
              </span>
            </div>
          ))}

          <div className="debug-problem-summary">
            총 {problems.length}개 문제 (경고 {problems.filter((p) => p.type === 'warning').length}, 오류{' '}
            {problems.filter((p) => p.type === 'error').length}, 보안{' '}
            {problems.filter((p) => p.type === 'security').length})
          </div>
        </>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Terminal tab                                                       */
/* ------------------------------------------------------------------ */

function TerminalTab() {
  return (
    <Suspense fallback={
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--color-text-tertiary)' }}>
        <Loader2 size={20} className="animate-spin" />
      </div>
    }>
      <TerminalPanel />
    </Suspense>
  );
}

/* ------------------------------------------------------------------ */
/*  Main component                                                     */
/* ------------------------------------------------------------------ */

const tabContent: Record<string, React.FC> = {
  console: ConsoleTab,
  network: NetworkTab,
  problems: ProblemsTab,
  terminal: TerminalTab,
};

export default function DebugPanel({ className = '' }: DebugPanelProps) {
  const [activeTab, setActiveTab] = useState('console');
  const ActiveContent = tabContent[activeTab];

  return (
    <div className={`debug-panel ${className}`}>
      {/* Tab bar */}
      <Tabs tabs={tabs} activeTab={activeTab} onTabChange={setActiveTab} />

      {/* Tab content */}
      <div className="debug-content">
        <ActiveContent />
      </div>
    </div>
  );
}
