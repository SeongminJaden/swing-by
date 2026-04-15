import { useState, useRef, useCallback, useEffect } from 'react';
import { ZoomIn, ZoomOut, Maximize2, Play, RotateCcw, Loader2 } from 'lucide-react';
import { useAgentStore, AGENT_DEFINITIONS, PIPELINE_EDGES, PIPELINE_STAGES, AgentStatus } from '../../stores/agentStore';
import { useAppStore } from '../../stores/appStore';

// ─── Layout ───────────────────────────────────────────────────────────────────

const NODE_W = 160;
const NODE_H = 64;
const COLS_X: Record<string, number> = {
  planning: 60,
  design: 280,
  development: 500,
  quality: 720,
  security: 940,
  release: 1160,
};
const STAGE_AGENT_Y: Record<string, number[]> = {
  planning:    [80, 200, 320],
  design:      [130, 250],
  development: [130, 250],
  quality:     [190],
  security:    [190],
  release:     [80, 180, 280, 380],
};

// Compute node positions
const NODE_POSITIONS: Record<string, { x: number; y: number }> = {};
for (const stage of PIPELINE_STAGES) {
  const x = COLS_X[stage.id] ?? 60;
  const ys = STAGE_AGENT_Y[stage.id] ?? [190];
  stage.agents.forEach((agentId, i) => {
    NODE_POSITIONS[agentId] = { x, y: ys[i] ?? 190 + i * 110 };
  });
}

const CANVAS_W = 1440;
const CANVAS_H = 560;
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 2.0;
const ZOOM_STEP = 0.1;

// ─── Status helpers ───────────────────────────────────────────────────────────

function statusColor(s: AgentStatus): string {
  switch (s) {
    case 'running': return 'var(--color-accent-primary)';
    case 'done':    return 'var(--color-accent-success, #22c55e)';
    case 'error':   return 'var(--color-accent-error, #ef4444)';
    default:        return 'var(--color-border-primary)';
  }
}

function nodeClass(s: AgentStatus, selected: boolean): React.CSSProperties {
  const border = selected ? 'var(--color-accent-primary)' : statusColor(s);
  const bg = s === 'running' ? 'rgba(99,102,241,0.08)' : s === 'done' ? 'rgba(34,197,94,0.06)' : s === 'error' ? 'rgba(239,68,68,0.06)' : 'var(--color-bg-elevated)';
  return {
    position: 'absolute',
    width: NODE_W,
    height: NODE_H,
    borderRadius: 12,
    background: bg,
    border: `1.5px solid ${border}`,
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    padding: '0 12px',
    cursor: 'pointer',
    boxShadow: selected ? `0 0 0 2px var(--color-accent-primary)44` : s === 'running' ? `0 0 12px ${statusColor(s)}44` : 'none',
    transition: 'all 0.15s',
    userSelect: 'none',
  };
}

// ─── Connection line ──────────────────────────────────────────────────────────

function Edge({ fromId, toId, agentStatuses }: {
  fromId: string;
  toId: string;
  agentStatuses: Record<string, AgentStatus>;
}) {
  const from = NODE_POSITIONS[fromId];
  const to = NODE_POSITIONS[toId];
  if (!from || !to) return null;

  const x1 = from.x + NODE_W;
  const y1 = from.y + NODE_H / 2;
  const x2 = to.x;
  const y2 = to.y + NODE_H / 2;
  const cpx = (x1 + x2) / 2;

  const fromStatus = agentStatuses[fromId] ?? 'idle';
  const toStatus = agentStatuses[toId] ?? 'idle';
  const active = fromStatus === 'done' || fromStatus === 'running';
  const color = fromStatus === 'done' && toStatus === 'done'
    ? 'var(--color-accent-success, #22c55e)'
    : active ? 'var(--color-accent-primary)' : 'var(--color-border-primary)';

  const d = `M ${x1} ${y1} C ${cpx} ${y1}, ${cpx} ${y2}, ${x2} ${y2}`;

  return (
    <g>
      <path d={d} fill="none" stroke={color} strokeWidth={1.5}
        strokeDasharray="6 4"
        opacity={active ? 1 : 0.35}
        className={active ? 'workflow-connection-animated' : ''} />
      <circle cx={x2} cy={y2} r={3} fill={color} opacity={active ? 1 : 0.35} />
    </g>
  );
}

// ─── Single node ─────────────────────────────────────────────────────────────

function AgentNode({ agentId, selected, onClick }: {
  agentId: string;
  selected: boolean;
  onClick: () => void;
}) {
  const def = AGENT_DEFINITIONS.find((d) => d.id === agentId);
  const status = useAgentStore((s) => s.agents[agentId]?.status ?? 'idle');
  const pos = NODE_POSITIONS[agentId];
  if (!def || !pos) return null;

  return (
    <div
      style={{ ...nodeClass(status, selected), left: pos.x, top: pos.y }}
      onClick={(e) => { e.stopPropagation(); onClick(); }}
    >
      <div style={{
        width: 32, height: 32, borderRadius: 8, flexShrink: 0,
        background: `${def.color}22`, border: `1px solid ${def.color}44`,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: 16,
      }}>
        {def.emoji}
      </div>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--color-text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {def.name}
        </div>
        <div style={{ fontSize: 9, color: statusColor(status), marginTop: 1 }}>
          {status === 'running' ? (
            <span style={{ display: 'flex', alignItems: 'center', gap: 3 }}>
              <Loader2 size={9} style={{ animation: 'spin 1s linear infinite' }} /> Running
            </span>
          ) : { idle: def.role, done: 'Done', error: 'Error' }[status]}
        </div>
      </div>
      {status === 'running' && (
        <div style={{
          position: 'absolute', inset: -1, borderRadius: 12,
          border: `1.5px solid var(--color-accent-primary)`,
          animation: 'agentPulse 1.2s ease-in-out infinite',
          pointerEvents: 'none',
        }} />
      )}
    </div>
  );
}

// ─── Stage label ─────────────────────────────────────────────────────────────

function StageLabel({ stageId, label }: { stageId: string; label: string }) {
  const x = COLS_X[stageId];
  const ys = STAGE_AGENT_Y[stageId] ?? [190];
  const yStart = Math.min(...ys) - 32;

  return (
    <div style={{
      position: 'absolute', left: x, top: yStart, width: NODE_W,
      textAlign: 'center', fontSize: 9, fontWeight: 700,
      color: 'var(--color-text-tertiary)', textTransform: 'uppercase',
      letterSpacing: '0.08em',
    }}>
      {label}
    </div>
  );
}

// ─── Canvas ───────────────────────────────────────────────────────────────────

export function AgentGraph() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState(0.85);
  const [pan, setPan] = useState({ x: 40, y: 40 });
  const [isPanning, setIsPanning] = useState(false);
  const panStart = useRef({ x: 0, y: 0 });
  const panOrigin = useRef({ x: 0, y: 0 });

  const agentStatuses = useAgentStore((s) =>
    Object.fromEntries(Object.entries(s.agents).map(([id, st]) => [id, st.status]))
  ) as Record<string, AgentStatus>;
  const { sprintRunning, sprintRequest, setSprintRunning, resetAll } = useAgentStore();
  const { selectedAgentId, setSelectedAgentId } = useAppStore();

  const handleWheel = useCallback((e: WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP;
    setZoom((prev) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, prev + delta)));
  }, []);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.addEventListener('wheel', handleWheel, { passive: false });
    return () => el.removeEventListener('wheel', handleWheel);
  }, [handleWheel]);

  const fitView = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    const scaleX = width / CANVAS_W;
    const scaleY = height / CANVAS_H;
    const z = Math.min(scaleX, scaleY, 1) * 0.9;
    setPan({ x: (width - CANVAS_W * z) / 2, y: (height - CANVAS_H * z) / 2 });
    setZoom(z);
    setSelectedAgentId(null);
  }, [setSelectedAgentId]);

  const handleRunSprint = async () => {
    const api = (window as any).electronAPI;
    if (!api?.agentSprintRun || !sprintRequest.trim()) return;
    setSprintRunning(true);
    try {
      await api.agentSprintRun('my_project', sprintRequest);
    } finally {
      setSprintRunning(false);
    }
  };

  return (
    <div
      ref={containerRef}
      onMouseDown={(e) => {
        if (e.button === 1) { setIsPanning(true); panStart.current = { x: e.clientX, y: e.clientY }; panOrigin.current = { ...pan }; e.preventDefault(); }
      }}
      onMouseMove={(e) => {
        if (!isPanning) return;
        setPan({ x: panOrigin.current.x + e.clientX - panStart.current.x, y: panOrigin.current.y + e.clientY - panStart.current.y });
      }}
      onMouseUp={(e) => { if (e.button === 1) setIsPanning(false); }}
      onMouseLeave={() => setIsPanning(false)}
      onClick={() => setSelectedAgentId(null)}
      style={{ width: '100%', height: '100%', position: 'relative', overflow: 'hidden', background: 'var(--color-bg-primary)', cursor: isPanning ? 'grabbing' : 'default' }}
    >
      <style>{`
        @keyframes agentPulse { 0%,100% { opacity:1; } 50% { opacity:0.3; } }
      `}</style>

      {/* Toolbar */}
      <div style={{ position: 'absolute', top: 12, left: 12, zIndex: 20, display: 'flex', gap: 6, alignItems: 'center' }}>
        <button onClick={(e) => { e.stopPropagation(); if (!sprintRunning) handleRunSprint(); }}
          disabled={sprintRunning}
          style={{
            display: 'flex', alignItems: 'center', gap: 5,
            padding: '5px 10px', borderRadius: 8, fontSize: 11, fontWeight: 600,
            background: sprintRunning ? 'var(--color-bg-elevated)' : 'var(--color-accent-primary)',
            color: sprintRunning ? 'var(--color-text-secondary)' : '#fff',
            border: 'none', cursor: sprintRunning ? 'not-allowed' : 'pointer', opacity: sprintRunning ? 0.6 : 1,
          }}>
          {sprintRunning ? <Loader2 size={12} style={{ animation: 'spin 1s linear infinite' }} /> : <Play size={12} />}
          {sprintRunning ? 'Running...' : 'Run Sprint'}
        </button>
        <button onClick={(e) => { e.stopPropagation(); resetAll(); }}
          style={{ padding: '5px 8px', borderRadius: 8, fontSize: 11, background: 'var(--color-bg-elevated)', border: '1px solid var(--color-border-primary)', color: 'var(--color-text-secondary)', cursor: 'pointer', display: 'flex', alignItems: 'center' }}>
          <RotateCcw size={12} />
        </button>
        <span style={{ fontSize: 11, color: 'var(--color-text-tertiary)', marginLeft: 4 }}>
          {Object.values(agentStatuses).filter(s => s === 'done').length}/{AGENT_DEFINITIONS.length} done
        </span>
      </div>

      {/* Zoom controls */}
      <div style={{ position: 'absolute', top: 12, right: 12, zIndex: 20, display: 'flex', gap: 4, alignItems: 'center', background: 'var(--color-bg-elevated)', border: '1px solid var(--color-border-primary)', borderRadius: 8, padding: '3px 6px' }}>
        <button onClick={() => setZoom(z => Math.min(MAX_ZOOM, z + ZOOM_STEP))} style={{ padding: 3, background: 'none', border: 'none', cursor: 'pointer', color: 'var(--color-text-secondary)', display: 'flex' }} title="Zoom in"><ZoomIn size={14} /></button>
        <span style={{ fontSize: 10, color: 'var(--color-text-tertiary)', minWidth: 36, textAlign: 'center' }}>{Math.round(zoom * 100)}%</span>
        <button onClick={() => setZoom(z => Math.max(MIN_ZOOM, z - ZOOM_STEP))} style={{ padding: 3, background: 'none', border: 'none', cursor: 'pointer', color: 'var(--color-text-secondary)', display: 'flex' }} title="Zoom out"><ZoomOut size={14} /></button>
        <button onClick={fitView} style={{ padding: 3, background: 'none', border: 'none', cursor: 'pointer', color: 'var(--color-text-secondary)', display: 'flex' }} title="Fit view"><Maximize2 size={12} /></button>
      </div>

      {/* Canvas transform layer */}
      <div style={{
        position: 'absolute', left: 0, top: 0,
        width: CANVAS_W, height: CANVAS_H,
        transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
        transformOrigin: '0 0',
      }}>
        {/* SVG edges */}
        <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', overflow: 'visible', pointerEvents: 'none' }}>
          {PIPELINE_EDGES.map((e) => (
            <Edge key={`${e.from}-${e.to}`} fromId={e.from} toId={e.to} agentStatuses={agentStatuses} />
          ))}
        </svg>

        {/* Stage labels */}
        {PIPELINE_STAGES.map((s) => <StageLabel key={s.id} stageId={s.id} label={s.label} />)}

        {/* Agent nodes */}
        {AGENT_DEFINITIONS.map((def) => (
          <AgentNode
            key={def.id}
            agentId={def.id}
            selected={selectedAgentId === def.id}
            onClick={() => setSelectedAgentId(def.id)}
          />
        ))}
      </div>
    </div>
  );
}

export default AgentGraph;
