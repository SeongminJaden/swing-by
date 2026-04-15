import { useState, useRef, useCallback, useEffect, useMemo } from 'react';
import {
  FileText, Bot, Shield, Rocket, Eye, Cpu, HardDrive, ClipboardCheck,
  Check, ZoomIn, ZoomOut, Maximize2, X, Clock, AlertTriangle, CheckCircle2,
  Loader2, Circle, Play,
} from 'lucide-react';
import { useWorkflowStore, type NodeStatus, type NodeState } from '../../stores/workflowStore';

interface WorkflowNode {
  id: string;
  title: string;
  subtitle?: string;
  icon: React.ReactNode;
  x: number;
  y: number;
}

interface WorkflowConnection {
  from: string;
  to: string;
}

// Static node definitions (no status -- status comes from store)
const nodeDefinitions: WorkflowNode[] = [
  { id: 'prd', title: 'PRD 입력', subtitle: '요구사항 문서', icon: <FileText size={20} />, x: 80, y: 200 },
  { id: 'codegen', title: 'AI 코드 생성', subtitle: 'Claude Opus 4', icon: <Bot size={20} />, x: 360, y: 200 },
  { id: 'ai-model', title: 'Claude Opus', subtitle: 'AI 모델', icon: <Cpu size={18} />, x: 300, y: 400 },
  { id: 'filesystem', title: '파일 시스템', subtitle: '소스 코드 출력', icon: <HardDrive size={18} />, x: 490, y: 400 },
  { id: 'security', title: '보안 검증', subtitle: '취약점 스캔', icon: <Shield size={20} />, x: 680, y: 200 },
  { id: 'report', title: '검증 리포트', subtitle: '보안 결과', icon: <ClipboardCheck size={18} />, x: 680, y: 400 },
  { id: 'deploy', title: '출시', subtitle: '프로덕션 출시', icon: <Rocket size={20} />, x: 960, y: 200 },
  { id: 'preview', title: '미리보기', subtitle: '실시간 프리뷰', icon: <Eye size={18} />, x: 960, y: 400 },
];

const connections: WorkflowConnection[] = [
  { from: 'prd', to: 'codegen' },
  { from: 'codegen', to: 'security' },
  { from: 'codegen', to: 'ai-model' },
  { from: 'codegen', to: 'filesystem' },
  { from: 'security', to: 'deploy' },
  { from: 'security', to: 'report' },
  { from: 'deploy', to: 'preview' },
];

const NODE_WIDTH = 180;
const NODE_HEIGHT = 72;

// 노드별 AI 상태 정보 (fallback / static descriptions)
interface NodeDetail {
  summary: string;
  items: { label: string; value: string; status?: 'ok' | 'warn' | 'error' | 'info' }[];
  aiNote?: string;
}

const staticNodeDetails: Record<string, NodeDetail> = {
  prd: {
    summary: 'PRD 분석이 완료되었습니다.',
    items: [
      { label: '서비스 타입', value: '쇼핑몰 (E-commerce)', status: 'info' },
      { label: '프레임워크', value: 'Next.js 14 + Supabase', status: 'info' },
      { label: '주요 기능', value: '상품 목록, 장바구니, 결제', status: 'info' },
      { label: '분석 시간', value: '2.3초', status: 'ok' },
    ],
    aiNote: 'PRD에서 12개의 기능 요구사항과 3개의 비기능 요구사항을 추출했습니다. 결제 기능에 Stripe 연동이 필요합니다.',
  },
  codegen: {
    summary: 'AI가 코드를 생성하고 있습니다...',
    items: [
      { label: '진행률', value: '68% (17/25 파일)', status: 'info' },
      { label: '생성된 코드', value: '2,847 줄', status: 'ok' },
      { label: '토큰 사용량', value: '12,340 / ~$0.18', status: 'info' },
      { label: '예상 남은 시간', value: '~45초', status: 'info' },
    ],
    aiNote: '현재 결제 모듈(Stripe Checkout)을 생성 중입니다. app/api/checkout 라우트와 webhook 핸들러를 작성하고 있습니다.',
  },
  'ai-model': {
    summary: 'Claude Opus 4 모델이 활성 상태입니다.',
    items: [
      { label: '모델', value: 'claude-opus-4-6', status: 'ok' },
      { label: '상태', value: '스트리밍 중...', status: 'info' },
      { label: '컨텍스트', value: '48K / 200K 토큰', status: 'ok' },
      { label: '응답 속도', value: '52 tok/s', status: 'ok' },
    ],
    aiNote: '코드 생성에 최적화된 시스템 프롬프트가 적용되어 있습니다. TypeScript + Next.js 전문 모드로 동작 중입니다.',
  },
  filesystem: {
    summary: '파일 시스템 출력 대기 중입니다.',
    items: [
      { label: '대상 경로', value: '~/projects/shopping-mall', status: 'info' },
      { label: '생성 예정', value: '25개 파일', status: 'info' },
      { label: '디렉토리', value: '8개 폴더', status: 'info' },
      { label: '상태', value: '코드 생성 완료 후 시작', status: 'warn' },
    ],
  },
  security: {
    summary: '보안 검증이 대기 중입니다.',
    items: [
      { label: '검증 엔진', value: 'ESLint + Semgrep + AI', status: 'info' },
      { label: 'OWASP Top 10', value: '대기', status: 'warn' },
      { label: '의존성 CVE', value: '대기', status: 'warn' },
      { label: '시크릿 노출', value: '대기', status: 'warn' },
    ],
    aiNote: '코드 생성이 완료되면 자동으로 보안 검증이 시작됩니다. HIGH 이슈가 발견되면 배포가 차단됩니다.',
  },
  report: {
    summary: '검증 리포트가 대기 중입니다.',
    items: [
      { label: '보안 점수', value: '—', status: 'info' },
      { label: '품질 점수', value: '—', status: 'info' },
      { label: 'HIGH 이슈', value: '—', status: 'info' },
      { label: '리포트 형식', value: 'PDF / 인라인', status: 'info' },
    ],
  },
  deploy: {
    summary: '출시가 대기 중입니다.',
    items: [
      { label: '플랫폼', value: 'Vercel (자동 감지)', status: 'info' },
      { label: '환경', value: 'Production', status: 'info' },
      { label: '도메인', value: 'shop.vercel.app', status: 'info' },
      { label: '상태', value: '보안 검증 통과 필요', status: 'warn' },
    ],
    aiNote: '보안 검증을 통과해야 출시가 가능합니다. Vercel 계정이 연결되어 있어야 합니다.',
  },
  preview: {
    summary: '미리보기가 대기 중입니다.',
    items: [
      { label: 'URL', value: 'localhost:3000', status: 'info' },
      { label: '핫 리로드', value: '대기', status: 'info' },
      { label: '뷰포트', value: '데스크톱', status: 'info' },
    ],
  },
};

function getNodeCenter(node: WorkflowNode) {
  return { cx: node.x + NODE_WIDTH / 2, cy: node.y + NODE_HEIGHT / 2 };
}

function getConnectionColor(fromStatus: NodeStatus, toStatus: NodeStatus): string {
  if (fromStatus === 'completed' && toStatus === 'completed') return 'var(--color-accent-success)';
  if (fromStatus === 'completed' || fromStatus === 'in-progress' || toStatus === 'in-progress')
    return 'var(--color-accent-primary)';
  return 'var(--color-border-primary)';
}

function getStatusClass(status: NodeStatus): string {
  switch (status) {
    case 'completed': return 'workflow-node-completed';
    case 'in-progress': return 'workflow-node-active';
    case 'pending': case 'idle': return 'workflow-node-pending';
    case 'error': return 'workflow-node-error';
    default: return '';
  }
}

function getStatusIconColor(status: NodeStatus): string {
  switch (status) {
    case 'completed': return 'var(--color-accent-success)';
    case 'in-progress': return 'var(--color-accent-primary)';
    case 'error': return 'var(--color-accent-error)';
    default: return 'var(--color-text-tertiary)';
  }
}

function ConnectionLine({
  from,
  to,
  allNodes,
  nodeStates,
}: {
  from: string;
  to: string;
  allNodes: WorkflowNode[];
  nodeStates: Record<string, NodeState>;
}) {
  const fromNode = allNodes.find((n) => n.id === from);
  const toNode = allNodes.find((n) => n.id === to);
  if (!fromNode || !toNode) return null;

  const fromStatus = nodeStates[from]?.status || 'idle';
  const toStatus = nodeStates[to]?.status || 'idle';

  const { cx: x1, cy: y1 } = getNodeCenter(fromNode);
  const { cx: x2, cy: y2 } = getNodeCenter(toNode);
  const color = getConnectionColor(fromStatus, toStatus);
  const isActive = fromStatus === 'in-progress' || fromStatus === 'completed';

  const dx = Math.abs(x2 - x1);
  const dy = Math.abs(y2 - y1);
  const cpOffset = Math.max(dx, dy) * 0.4;

  const d = Math.abs(y2 - y1) < 20
    ? `M ${x1} ${y1} C ${x1 + cpOffset} ${y1}, ${x2 - cpOffset} ${y2}, ${x2} ${y2}`
    : `M ${x1} ${y1} C ${x1} ${y1 + cpOffset}, ${x2} ${y2 - cpOffset}, ${x2} ${y2}`;

  return (
    <g>
      <path d={d} fill="none" stroke={color} strokeWidth={2} strokeDasharray="8 4"
        className={isActive ? 'workflow-connection-animated' : ''} opacity={isActive ? 1 : 0.4} />
      <circle cx={x2} cy={y2} r={3} fill={color} opacity={isActive ? 1 : 0.4} />
    </g>
  );
}

function WorkflowNodeCard({
  node,
  status,
  isSelected,
  onClick,
}: {
  node: WorkflowNode;
  status: NodeStatus;
  isSelected: boolean;
  onClick: () => void;
}) {
  const statusClass = getStatusClass(status);
  const iconColor = getStatusIconColor(status);
  const isActive = status === 'in-progress';

  return (
    <div
      className={`workflow-node ${statusClass} ${isActive ? 'workflow-float' : ''} ${isSelected ? 'workflow-node-selected' : ''}`}
      style={{ left: node.x, top: node.y, width: NODE_WIDTH, height: NODE_HEIGHT }}
      onClick={(e) => { e.stopPropagation(); onClick(); }}
    >
      <div className="workflow-node-icon" style={{ color: iconColor }}>{node.icon}</div>
      <div className="workflow-node-text">
        <div className="workflow-node-title">{node.title}</div>
        {node.subtitle && <div className="workflow-node-subtitle">{node.subtitle}</div>}
      </div>
      {status === 'completed' && (
        <div className="workflow-node-badge-completed"><Check size={12} /></div>
      )}
      {isActive && <div className="workflow-pulse" />}
    </div>
  );
}

// ========================================
// 노드 상태 인포 패널
// ========================================
function StatusIcon({ status }: { status?: string }) {
  switch (status) {
    case 'ok': return <CheckCircle2 size={14} className="text-accent-success shrink-0" />;
    case 'warn': return <AlertTriangle size={14} className="text-accent-warning shrink-0" />;
    case 'error': return <AlertTriangle size={14} className="text-accent-error shrink-0" />;
    default: return <Circle size={14} className="text-text-tertiary shrink-0" />;
  }
}

function formatTimestamp(ts?: number): string {
  if (!ts) return '—';
  const d = new Date(ts);
  return `${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}:${d.getSeconds().toString().padStart(2, '0')}`;
}

function formatDuration(start?: number, end?: number): string {
  if (!start) return '—';
  const endTime = end || Date.now();
  const diff = Math.max(0, endTime - start);
  if (diff < 1000) return `${diff}ms`;
  return `${(diff / 1000).toFixed(1)}초`;
}

function NodeInfoPanel({
  node,
  nodeState,
  onClose,
}: {
  node: WorkflowNode;
  nodeState: NodeState;
  onClose: () => void;
}) {
  const detail = staticNodeDetails[node.id];
  if (!detail) return null;

  const status = nodeState.status;

  // Build dynamic items based on store data
  const dynamicItems: { label: string; value: string; status?: 'ok' | 'warn' | 'error' | 'info' }[] = [];

  if (nodeState.progress !== undefined && status === 'in-progress') {
    dynamicItems.push({ label: '진행률', value: `${nodeState.progress}%`, status: 'info' });
  }
  if (nodeState.message) {
    dynamicItems.push({ label: '메시지', value: nodeState.message, status: status === 'error' ? 'error' : 'info' });
  }
  if (nodeState.startedAt) {
    dynamicItems.push({ label: '시작 시간', value: formatTimestamp(nodeState.startedAt), status: 'ok' });
  }
  if (nodeState.completedAt) {
    dynamicItems.push({ label: '완료 시간', value: formatTimestamp(nodeState.completedAt), status: 'ok' });
  }
  if (nodeState.startedAt) {
    dynamicItems.push({
      label: '소요 시간',
      value: formatDuration(nodeState.startedAt, nodeState.completedAt),
      status: 'ok',
    });
  }

  // Merge: show dynamic items first, then static items from detail
  const allItems = [...dynamicItems, ...detail.items];

  const statusLabel = {
    completed: '완료',
    'in-progress': '진행 중',
    pending: '대기',
    error: '오류',
    idle: '비활성',
  }[status];

  const statusColor = {
    completed: 'text-accent-success',
    'in-progress': 'text-accent-primary',
    pending: 'text-text-tertiary',
    error: 'text-accent-error',
    idle: 'text-text-tertiary',
  }[status];

  const statusBg = {
    completed: 'bg-accent-success/10',
    'in-progress': 'bg-accent-primary/10',
    pending: 'bg-bg-elevated',
    error: 'bg-accent-error/10',
    idle: 'bg-bg-elevated',
  }[status];

  // Summary: use store message if available, otherwise fall back to static
  const summary = (status === 'error' && nodeState.message)
    ? nodeState.message
    : (status === 'completed' ? `${node.title} 완료되었습니다.` : detail.summary);

  return (
    <div className="node-info-panel">
      {/* Header */}
      <div className="node-info-header">
        <div className="flex items-center gap-3">
          <div className="node-info-icon" style={{ color: getStatusIconColor(status) }}>
            {node.icon}
          </div>
          <div>
            <h3 className="node-info-title">{node.title}</h3>
            {node.subtitle && <p className="node-info-subtitle">{node.subtitle}</p>}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span className={`node-info-status-badge ${statusBg} ${statusColor}`}>
            {status === 'in-progress' && <Loader2 size={12} className="animate-spin" />}
            {status === 'completed' && <CheckCircle2 size={12} />}
            {status === 'error' && <AlertTriangle size={12} />}
            {statusLabel}
          </span>
          <button onClick={onClose} className="node-info-close">
            <X size={16} />
          </button>
        </div>
      </div>

      {/* Progress bar for in-progress nodes */}
      {status === 'in-progress' && nodeState.progress !== undefined && (
        <div className="mx-4 mt-2 h-1.5 rounded-full bg-bg-tertiary overflow-hidden">
          <div
            className="h-full rounded-full bg-accent-primary transition-all duration-300 ease-out"
            style={{ width: `${nodeState.progress}%` }}
          />
        </div>
      )}

      {/* AI Summary */}
      <div className="node-info-summary">
        <Bot size={14} className="text-accent-primary shrink-0 mt-0.5" />
        <p>{summary}</p>
      </div>

      {/* Details */}
      <div className="node-info-details">
        {allItems.map((item, i) => (
          <div key={i} className="node-info-row">
            <div className="flex items-center gap-2">
              <StatusIcon status={item.status} />
              <span className="node-info-row-label">{item.label}</span>
            </div>
            <span className="node-info-row-value">{item.value}</span>
          </div>
        ))}
      </div>

      {/* AI Note */}
      {detail.aiNote && (
        <div className="node-info-ai-note">
          <div className="node-info-ai-note-header">
            <Bot size={13} className="text-accent-primary" />
            <span>AI 분석</span>
          </div>
          <p className="node-info-ai-note-text">{detail.aiNote}</p>
        </div>
      )}
    </div>
  );
}

// ========================================
// 팬/줌 + 캔버스
// ========================================
const MIN_ZOOM = 0.3;
const MAX_ZOOM = 2.0;
const ZOOM_STEP = 0.1;

// 노드 영역의 바운딩 박스 계산
const PADDING = 60;
const contentBounds = {
  minX: Math.min(...nodeDefinitions.map((n) => n.x)) - PADDING,
  minY: Math.min(...nodeDefinitions.map((n) => n.y)) - PADDING,
  maxX: Math.max(...nodeDefinitions.map((n) => n.x + NODE_WIDTH)) + PADDING,
  maxY: Math.max(...nodeDefinitions.map((n) => n.y + NODE_HEIGHT)) + PADDING,
};
const contentWidth = contentBounds.maxX - contentBounds.minX;
const contentHeight = contentBounds.maxY - contentBounds.minY;
const contentCenterX = (contentBounds.minX + contentBounds.maxX) / 2;
const contentCenterY = (contentBounds.minY + contentBounds.maxY) / 2;

// 캔버스 내부 좌표계 크기 (grid-bg)
const CANVAS_W = 1400;
const CANVAS_H = 700;

function calcFitView(containerW: number, containerH: number) {
  const scaleX = containerW / contentWidth;
  const scaleY = containerH / contentHeight;
  const fitZoom = Math.min(scaleX, scaleY, MAX_ZOOM) * 0.9;

  const panX = (containerW / 2) - (contentCenterX * fitZoom);
  const panY = (containerH / 2) - (contentCenterY * fitZoom);

  return { zoom: fitZoom, pan: { x: panX, y: panY } };
}

export function WorkflowCanvas() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isPanning, setIsPanning] = useState(false);
  const [initialized, setInitialized] = useState(false);
  const panStart = useRef({ x: 0, y: 0 });
  const panOrigin = useRef({ x: 0, y: 0 });
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  // Connect to workflow store
  const nodeStates = useWorkflowStore((s) => s.nodes);
  const isRunning = useWorkflowStore((s) => s.isRunning);
  const startPipeline = useWorkflowStore((s) => s.startPipeline);
  const resetAll = useWorkflowStore((s) => s.resetAll);
  const completedCount = useWorkflowStore((s) => s.completedCount);
  const totalCount = useWorkflowStore((s) => s.totalCount);

  const selectedNode = selectedNodeId ? nodeDefinitions.find((n) => n.id === selectedNodeId) : null;
  const selectedNodeState = selectedNodeId ? nodeStates[selectedNodeId] : null;

  // 초기 fit-to-view 계산
  useEffect(() => {
    if (initialized) return;
    const el = containerRef.current;
    if (!el) return;

    const rect = el.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      const fit = calcFitView(rect.width, rect.height);
      setZoom(fit.zoom);
      setPan(fit.pan);
      setInitialized(true);
    }
  }, [initialized]);

  const fitToView = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const fit = calcFitView(rect.width, rect.height);
    setZoom(fit.zoom);
    setPan(fit.pan);
    setSelectedNodeId(null);
  }, []);

  const handleWheel = useCallback((e: WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP;
    setZoom((prev) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, prev + delta)));
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button === 1) {
      e.preventDefault();
      setIsPanning(true);
      panStart.current = { x: e.clientX, y: e.clientY };
      panOrigin.current = { ...pan };
    }
  }, [pan]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!isPanning) return;
    const dx = e.clientX - panStart.current.x;
    const dy = e.clientY - panStart.current.y;
    setPan({ x: panOrigin.current.x + dx, y: panOrigin.current.y + dy });
  }, [isPanning]);

  const handleMouseUp = useCallback((e: React.MouseEvent) => {
    if (e.button === 1) setIsPanning(false);
  }, []);

  const handleCanvasClick = useCallback(() => {
    setSelectedNodeId(null);
  }, []);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.addEventListener('wheel', handleWheel, { passive: false });
    return () => el.removeEventListener('wheel', handleWheel);
  }, [handleWheel]);

  const zoomIn = () => setZoom((prev) => Math.min(MAX_ZOOM, prev + ZOOM_STEP));
  const zoomOut = () => setZoom((prev) => Math.max(MIN_ZOOM, prev - ZOOM_STEP));

  return (
    <div
      ref={containerRef}
      className="workflow-canvas"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={() => setIsPanning(false)}
      onClick={handleCanvasClick}
      style={{ cursor: isPanning ? 'grabbing' : 'default' }}
    >
      {/* Zoom controls */}
      <div className="workflow-zoom-controls">
        <button onClick={zoomIn} className="workflow-zoom-btn" title="확대"><ZoomIn size={16} /></button>
        <span className="workflow-zoom-label">{Math.round(zoom * 100)}%</span>
        <button onClick={zoomOut} className="workflow-zoom-btn" title="축소"><ZoomOut size={16} /></button>
        <button onClick={fitToView} className="workflow-zoom-btn" title="화면에 맞추기"><Maximize2 size={14} /></button>
      </div>

      {/* Pipeline controls */}
      <div className="absolute top-3 left-3 z-20 flex items-center gap-2">
        <button
          onClick={(e) => {
            e.stopPropagation();
            if (isRunning) return;
            startPipeline();
          }}
          disabled={isRunning}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-accent-primary text-white hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
          title="파이프라인 실행"
        >
          {isRunning ? (
            <Loader2 size={14} className="animate-spin" />
          ) : (
            <Play size={14} />
          )}
          {isRunning ? '실행 중...' : '파이프라인 실행'}
        </button>
        {!isRunning && completedCount > 0 && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              resetAll();
            }}
            className="px-3 py-1.5 text-xs font-medium rounded-lg bg-bg-elevated text-text-secondary hover:text-text-primary transition-colors"
            title="초기화"
          >
            초기화
          </button>
        )}
        <span className="text-xs text-text-tertiary ml-1">
          {completedCount}/{totalCount} 완료
        </span>
      </div>

      {/* Node Info Panel */}
      {selectedNode && selectedNodeState && (
        <NodeInfoPanel
          node={selectedNode}
          nodeState={selectedNodeState}
          onClose={() => setSelectedNodeId(null)}
        />
      )}

      {/* Transform layer */}
      <div
        className="workflow-grid-bg"
        style={{
          width: CANVAS_W,
          height: CANVAS_H,
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          transformOrigin: '0 0',
        }}
      >
        <svg className="workflow-svg-layer" width={CANVAS_W} height={CANVAS_H}>
          {connections.map((conn) => (
            <ConnectionLine
              key={`${conn.from}-${conn.to}`}
              from={conn.from}
              to={conn.to}
              allNodes={nodeDefinitions}
              nodeStates={nodeStates}
            />
          ))}
        </svg>

        {nodeDefinitions.map((node) => (
          <WorkflowNodeCard
            key={node.id}
            node={node}
            status={nodeStates[node.id]?.status || 'idle'}
            isSelected={selectedNodeId === node.id}
            onClick={() => setSelectedNodeId(node.id)}
          />
        ))}
      </div>
    </div>
  );
}
