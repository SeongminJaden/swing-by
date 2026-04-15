import React, { useEffect } from 'react';
import { Play, RotateCcw, Activity, CheckCircle2, XCircle, Clock, ChevronRight } from 'lucide-react';
import { useAgentStore, AGENT_DEFINITIONS, PIPELINE_STAGES, AgentStatus } from '../../stores/agentStore';
import { useAppStore } from '../../stores/appStore';

function statusColor(s: AgentStatus): string {
  switch (s) {
    case 'running': return 'var(--color-accent-primary)';
    case 'done':    return 'var(--color-accent-success, #22c55e)';
    case 'error':   return 'var(--color-accent-error, #ef4444)';
    default:        return 'var(--color-text-tertiary)';
  }
}

function StatusDot({ status }: { status: AgentStatus }) {
  const color = statusColor(status);
  return (
    <span style={{
      display: 'inline-block',
      width: 7,
      height: 7,
      borderRadius: '50%',
      background: color,
      flexShrink: 0,
      boxShadow: status === 'running' ? `0 0 6px ${color}` : 'none',
      animation: status === 'running' ? 'agentPulse 1.2s ease-in-out infinite' : 'none',
    }} />
  );
}

function StatusBadge({ status }: { status: AgentStatus }) {
  const labels: Record<AgentStatus, string> = {
    idle: 'Idle', running: 'Running', done: 'Done', error: 'Error',
  };
  const icons: Record<AgentStatus, React.ReactNode> = {
    idle:    <Clock size={10} />,
    running: <Activity size={10} style={{ animation: 'spin 1s linear infinite' }} />,
    done:    <CheckCircle2 size={10} />,
    error:   <XCircle size={10} />,
  };
  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: 3,
      fontSize: 10, fontWeight: 500, padding: '1px 6px', borderRadius: 8,
      color: statusColor(status),
      background: `${statusColor(status)}1a`,
      border: `1px solid ${statusColor(status)}33`,
    }}>
      {icons[status]}
      {labels[status]}
    </span>
  );
}

function AgentCard({ agentId, onClick }: { agentId: string; onClick: () => void }) {
  const def = AGENT_DEFINITIONS.find((d) => d.id === agentId);
  const state = useAgentStore((s) => s.agents[agentId]);
  if (!def) return null;

  const isSelected = useAppStore((s) => s.selectedAgentId) === agentId;

  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex', alignItems: 'center', gap: 10,
        width: '100%', padding: '8px 10px', borderRadius: 8,
        background: isSelected ? 'var(--color-bg-elevated)' : 'transparent',
        border: isSelected ? `1px solid ${def.color}44` : '1px solid transparent',
        cursor: 'pointer', textAlign: 'left',
        transition: 'background 0.15s, border-color 0.15s',
      }}
      onMouseEnter={(e) => {
        if (!isSelected) (e.currentTarget as HTMLElement).style.background = 'var(--color-bg-secondary)';
      }}
      onMouseLeave={(e) => {
        if (!isSelected) (e.currentTarget as HTMLElement).style.background = 'transparent';
      }}
    >
      {/* Emoji avatar */}
      <div style={{
        width: 32, height: 32, borderRadius: 8, flexShrink: 0,
        background: `${def.color}22`,
        border: `1px solid ${def.color}44`,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: 16,
      }}>
        {def.emoji}
      </div>

      {/* Text */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 4,
        }}>
          <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--color-text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {def.name}
          </span>
          <StatusDot status={state?.status ?? 'idle'} />
        </div>
        <div style={{ fontSize: 10, color: 'var(--color-text-tertiary)', marginTop: 1, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {state?.status !== 'idle' && state?.lastOutput
            ? state.lastOutput.slice(0, 50)
            : def.role}
        </div>
      </div>

      <ChevronRight size={12} style={{ color: 'var(--color-text-tertiary)', flexShrink: 0 }} />
    </button>
  );
}

export const AgentPanel: React.FC = () => {
  const { sprintRunning, sprintProject, sprintRequest, setSprintProject, setSprintRequest, setSprintRunning, setBinaryAvailable, binaryAvailable, resetAll } = useAgentStore();
  const setSelectedAgentId = useAppStore((s) => s.setSelectedAgentId);

  // Check binary on mount
  useEffect(() => {
    const api = (window as any).electronAPI;
    if (!api?.agentCheckBinary) return;
    api.agentCheckBinary().then((r: any) => setBinaryAvailable(r.exists));
  }, [setBinaryAvailable]);

  const handleRunSprint = async () => {
    const api = (window as any).electronAPI;
    if (!api?.agentSprintRun || !sprintRequest.trim()) return;

    setSprintRunning(true);
    try {
      await api.agentSprintRun(sprintProject, sprintRequest);
    } finally {
      setSprintRunning(false);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      <style>{`
        @keyframes agentPulse { 0%,100% { opacity:1; } 50% { opacity:0.3; } }
      `}</style>

      {/* Binary status banner */}
      {binaryAvailable === false && (
        <div style={{
          padding: '6px 10px', fontSize: 10, color: 'var(--color-accent-warning, #f0a030)',
          background: 'rgba(240,160,48,0.08)', borderBottom: '1px solid var(--color-border-primary)',
        }}>
          AI Agent를 찾을 수 없습니다. 앱을 재설치하거나 Ollama가 실행 중인지 확인하세요.
        </div>
      )}

      {/* Sprint input */}
      <div style={{ padding: '10px', borderBottom: '1px solid var(--color-border-primary)', flexShrink: 0 }}>
        <div style={{ fontSize: 10, color: 'var(--color-text-tertiary)', marginBottom: 4, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          Sprint Request
        </div>
        <input
          value={sprintProject}
          onChange={(e) => setSprintProject(e.target.value)}
          placeholder="Project name"
          style={{
            width: '100%', padding: '4px 8px', fontSize: 11, borderRadius: 6, marginBottom: 6,
            background: 'var(--color-bg-elevated)', border: '1px solid var(--color-border-primary)',
            color: 'var(--color-text-primary)', outline: 'none',
          }}
        />
        <textarea
          value={sprintRequest}
          onChange={(e) => setSprintRequest(e.target.value)}
          placeholder="Describe what to build..."
          rows={2}
          style={{
            width: '100%', padding: '4px 8px', fontSize: 11, borderRadius: 6, resize: 'none',
            background: 'var(--color-bg-elevated)', border: '1px solid var(--color-border-primary)',
            color: 'var(--color-text-primary)', outline: 'none', fontFamily: 'inherit',
          }}
        />
        <div style={{ display: 'flex', gap: 6, marginTop: 6 }}>
          <button
            onClick={handleRunSprint}
            disabled={sprintRunning || !sprintRequest.trim() || binaryAvailable === false}
            style={{
              flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 5,
              padding: '5px 8px', fontSize: 11, fontWeight: 600, borderRadius: 6,
              background: sprintRunning ? 'var(--color-bg-elevated)' : 'var(--color-accent-primary)',
              color: sprintRunning ? 'var(--color-text-secondary)' : '#fff',
              border: 'none', cursor: sprintRunning ? 'not-allowed' : 'pointer', opacity: sprintRunning ? 0.6 : 1,
            }}
          >
            <Play size={12} />
            {sprintRunning ? 'Running...' : 'Run Sprint'}
          </button>
          <button
            onClick={resetAll}
            title="Reset all agents"
            style={{
              padding: '5px 8px', borderRadius: 6, fontSize: 11,
              background: 'var(--color-bg-elevated)', border: '1px solid var(--color-border-primary)',
              color: 'var(--color-text-secondary)', cursor: 'pointer',
            }}
          >
            <RotateCcw size={12} />
          </button>
        </div>
      </div>

      {/* Agent cards grouped by stage */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '8px' }}>
        {PIPELINE_STAGES.map((stage) => (
          <div key={stage.id} style={{ marginBottom: 12 }}>
            <div style={{
              fontSize: 10, fontWeight: 700, color: 'var(--color-text-tertiary)',
              textTransform: 'uppercase', letterSpacing: '0.08em',
              padding: '0 4px', marginBottom: 4,
            }}>
              {stage.label}
            </div>
            {stage.agents.map((agentId) => (
              <AgentCard
                key={agentId}
                agentId={agentId}
                onClick={() => setSelectedAgentId(agentId)}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
};

export default AgentPanel;
