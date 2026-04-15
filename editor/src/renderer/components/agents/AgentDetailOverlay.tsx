import React, { useRef, useEffect } from 'react';
import { X, Activity, CheckCircle2, XCircle, Clock, Bot, Terminal, RotateCcw } from 'lucide-react';
import { useAppStore } from '../../stores/appStore';
import { useAgentStore, AGENT_DEFINITIONS, AgentStatus } from '../../stores/agentStore';

function statusLabel(s: AgentStatus): string {
  return { idle: 'Idle', running: 'Running', done: 'Done', error: 'Error' }[s];
}
function statusColor(s: AgentStatus): string {
  return { idle: 'var(--color-text-tertiary)', running: 'var(--color-accent-primary)', done: 'var(--color-accent-success, #22c55e)', error: 'var(--color-accent-error, #ef4444)' }[s];
}
function StatusIcon({ status }: { status: AgentStatus }) {
  const color = statusColor(status);
  const size = 14;
  switch (status) {
    case 'running': return <Activity size={size} style={{ color, animation: 'spin 1s linear infinite' }} />;
    case 'done':    return <CheckCircle2 size={size} style={{ color }} />;
    case 'error':   return <XCircle size={size} style={{ color }} />;
    default:        return <Clock size={size} style={{ color }} />;
  }
}

function formatTime(ts?: number): string {
  if (!ts) return '—';
  return new Date(ts).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
}

function formatDuration(start?: number, end?: number): string {
  if (!start) return '—';
  const diff = Math.max(0, (end ?? Date.now()) - start);
  if (diff < 1000) return `${diff}ms`;
  return `${(diff / 1000).toFixed(1)}s`;
}

export const AgentDetailOverlay: React.FC = () => {
  const selectedAgentId = useAppStore((s) => s.selectedAgentId);
  const setSelectedAgentId = useAppStore((s) => s.setSelectedAgentId);
  const agentState = useAgentStore((s) => selectedAgentId ? s.agents[selectedAgentId] : null);
  const resetAgent = useAgentStore((s) => s.resetAgent);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const def = selectedAgentId ? AGENT_DEFINITIONS.find((d) => d.id === selectedAgentId) : null;

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [agentState?.logs]);

  if (!selectedAgentId || !def) return null;

  const state = agentState ?? { status: 'idle' as AgentStatus, lastOutput: '', logs: [] };
  const color = statusColor(state.status);

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={() => setSelectedAgentId(null)}
        style={{
          position: 'fixed', inset: 0, zIndex: 49,
          background: 'rgba(0,0,0,0.3)',
        }}
      />

      {/* Panel */}
      <div style={{
        position: 'fixed', top: 0, right: 0, bottom: 0, zIndex: 50,
        width: 420, maxWidth: '90vw',
        background: 'var(--color-bg-primary)',
        borderLeft: '1px solid var(--color-border-primary)',
        display: 'flex', flexDirection: 'column',
        boxShadow: '-8px 0 32px rgba(0,0,0,0.4)',
        animation: 'slideInRight 0.2s ease-out',
      }}>
        <style>{`
          @keyframes slideInRight {
            from { transform: translateX(100%); opacity: 0; }
            to   { transform: translateX(0);    opacity: 1; }
          }
        `}</style>

        {/* Header */}
        <div style={{
          display: 'flex', alignItems: 'center', gap: 12,
          padding: '14px 16px',
          borderBottom: '1px solid var(--color-border-primary)',
          flexShrink: 0,
        }}>
          <div style={{
            width: 40, height: 40, borderRadius: 10,
            background: `${def.color}22`,
            border: `1px solid ${def.color}55`,
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            fontSize: 20, flexShrink: 0,
          }}>
            {def.emoji}
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--color-text-primary)' }}>{def.name}</div>
            <div style={{ fontSize: 11, color: 'var(--color-text-tertiary)' }}>{def.role}</div>
          </div>
          {/* Status badge */}
          <div style={{
            display: 'flex', alignItems: 'center', gap: 5,
            padding: '3px 8px', borderRadius: 20,
            background: `${color}1a`, border: `1px solid ${color}33`,
            fontSize: 11, fontWeight: 600, color, flexShrink: 0,
          }}>
            <StatusIcon status={state.status} />
            {statusLabel(state.status)}
          </div>
          <button
            onClick={() => setSelectedAgentId(null)}
            style={{
              padding: 4, borderRadius: 6, background: 'transparent',
              border: 'none', cursor: 'pointer', color: 'var(--color-text-tertiary)',
              display: 'flex', alignItems: 'center',
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Meta info */}
        <div style={{
          padding: '12px 16px',
          borderBottom: '1px solid var(--color-border-primary)',
          flexShrink: 0,
        }}>
          <p style={{ fontSize: 12, color: 'var(--color-text-secondary)', lineHeight: 1.5, margin: 0 }}>
            {def.description}
          </p>
          {(state.startedAt || state.completedAt) && (
            <div style={{ display: 'flex', gap: 16, marginTop: 10 }}>
              {state.startedAt && (
                <div>
                  <div style={{ fontSize: 9, color: 'var(--color-text-tertiary)', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Started</div>
                  <div style={{ fontSize: 11, color: 'var(--color-text-secondary)' }}>{formatTime(state.startedAt)}</div>
                </div>
              )}
              {state.completedAt && (
                <div>
                  <div style={{ fontSize: 9, color: 'var(--color-text-tertiary)', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Finished</div>
                  <div style={{ fontSize: 11, color: 'var(--color-text-secondary)' }}>{formatTime(state.completedAt)}</div>
                </div>
              )}
              <div>
                <div style={{ fontSize: 9, color: 'var(--color-text-tertiary)', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em' }}>Duration</div>
                <div style={{ fontSize: 11, color: 'var(--color-text-secondary)' }}>{formatDuration(state.startedAt, state.completedAt)}</div>
              </div>
            </div>
          )}
          {state.progress !== undefined && state.status === 'running' && (
            <div style={{ marginTop: 10 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ fontSize: 10, color: 'var(--color-text-tertiary)' }}>Progress</span>
                <span style={{ fontSize: 10, color: 'var(--color-accent-primary)', fontWeight: 600 }}>{state.progress}%</span>
              </div>
              <div style={{ height: 4, borderRadius: 4, background: 'var(--color-bg-elevated)' }}>
                <div style={{
                  height: '100%', borderRadius: 4,
                  width: `${state.progress}%`,
                  background: 'var(--color-accent-primary)',
                  transition: 'width 0.3s ease',
                }} />
              </div>
            </div>
          )}
        </div>

        {/* Last output */}
        {state.lastOutput && (
          <div style={{
            padding: '10px 16px',
            borderBottom: '1px solid var(--color-border-primary)',
            flexShrink: 0,
          }}>
            <div style={{
              display: 'flex', alignItems: 'center', gap: 6,
              fontSize: 10, color: 'var(--color-text-tertiary)', fontWeight: 700,
              textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: 6,
            }}>
              <Bot size={11} /> Latest Output
            </div>
            <div style={{
              fontSize: 12, color: 'var(--color-text-secondary)',
              lineHeight: 1.5, maxHeight: 80, overflow: 'hidden',
              textOverflow: 'ellipsis', display: '-webkit-box',
              WebkitLineClamp: 4, WebkitBoxOrient: 'vertical',
            }}>
              {state.lastOutput}
            </div>
          </div>
        )}

        {/* Logs */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', padding: '10px 16px' }}>
          <div style={{
            display: 'flex', alignItems: 'center', justifyContent: 'space-between',
            marginBottom: 8, flexShrink: 0,
          }}>
            <div style={{
              display: 'flex', alignItems: 'center', gap: 6,
              fontSize: 10, color: 'var(--color-text-tertiary)', fontWeight: 700,
              textTransform: 'uppercase', letterSpacing: '0.06em',
            }}>
              <Terminal size={11} /> Logs ({state.logs.length})
            </div>
            <button
              onClick={() => resetAgent(selectedAgentId)}
              title="Reset agent"
              style={{
                display: 'flex', alignItems: 'center', gap: 4,
                padding: '2px 6px', borderRadius: 4, fontSize: 10,
                background: 'transparent', border: '1px solid var(--color-border-primary)',
                color: 'var(--color-text-tertiary)', cursor: 'pointer',
              }}
            >
              <RotateCcw size={10} /> Reset
            </button>
          </div>
          <div style={{
            flex: 1, overflowY: 'auto',
            background: 'var(--color-bg-secondary, #0d1117)',
            borderRadius: 8, padding: '8px 10px',
            fontFamily: 'monospace', fontSize: 11,
            color: 'var(--color-text-secondary)',
            lineHeight: 1.6,
            border: '1px solid var(--color-border-primary)',
          }}>
            {state.logs.length === 0 ? (
              <span style={{ color: 'var(--color-text-tertiary)' }}>No logs yet.</span>
            ) : (
              state.logs.map((log, i) => (
                <div key={i} style={{ wordBreak: 'break-word' }}>{log}</div>
              ))
            )}
            <div ref={logsEndRef} />
          </div>
        </div>
      </div>
    </>
  );
};

export default AgentDetailOverlay;
