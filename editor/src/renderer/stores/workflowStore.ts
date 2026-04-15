import { create } from 'zustand';

export type NodeStatus = 'idle' | 'completed' | 'in-progress' | 'pending' | 'error';

export interface NodeState {
  status: NodeStatus;
  progress?: number;
  message?: string;
  startedAt?: number;
  completedAt?: number;
}

interface WorkflowState {
  nodes: Record<string, NodeState>;

  // Actions
  setNodeStatus: (nodeId: string, status: NodeStatus, message?: string) => void;
  setNodeProgress: (nodeId: string, progress: number) => void;
  startNode: (nodeId: string) => void;
  completeNode: (nodeId: string) => void;
  failNode: (nodeId: string, error: string) => void;
  resetAll: () => void;

  // Pipeline execution
  isRunning: boolean;
  startPipeline: () => void;

  // Overall stats (computed via getters)
  completedCount: number;
  totalCount: number;
}

const DEFAULT_NODES: Record<string, NodeState> = {
  prd: { status: 'completed', completedAt: Date.now() - 60000 },
  codegen: { status: 'in-progress', progress: 68, message: '17/25 파일 생성 중...', startedAt: Date.now() - 30000 },
  'ai-model': { status: 'in-progress', message: '스트리밍 중...', startedAt: Date.now() - 30000 },
  filesystem: { status: 'pending', message: '코드 생성 완료 후 시작' },
  security: { status: 'pending' },
  report: { status: 'pending' },
  deploy: { status: 'pending' },
  preview: { status: 'idle' },
};

function countCompleted(nodes: Record<string, NodeState>): number {
  return Object.values(nodes).filter((n) => n.status === 'completed').length;
}

function countTotal(nodes: Record<string, NodeState>): number {
  return Object.keys(nodes).length;
}

export const useWorkflowStore = create<WorkflowState>((set, get) => ({
  nodes: { ...DEFAULT_NODES },
  isRunning: false,
  completedCount: countCompleted(DEFAULT_NODES),
  totalCount: countTotal(DEFAULT_NODES),

  setNodeStatus: (nodeId, status, message) => {
    set((state) => {
      const prev = state.nodes[nodeId] || { status: 'idle' };
      const updated = { ...state.nodes, [nodeId]: { ...prev, status, ...(message !== undefined ? { message } : {}) } };
      return { nodes: updated, completedCount: countCompleted(updated), totalCount: countTotal(updated) };
    });
  },

  setNodeProgress: (nodeId, progress) => {
    set((state) => {
      const prev = state.nodes[nodeId] || { status: 'idle' };
      const updated = { ...state.nodes, [nodeId]: { ...prev, progress } };
      return { nodes: updated };
    });
  },

  startNode: (nodeId) => {
    set((state) => {
      const prev = state.nodes[nodeId] || { status: 'idle' };
      const updated = {
        ...state.nodes,
        [nodeId]: { ...prev, status: 'in-progress' as NodeStatus, startedAt: Date.now(), completedAt: undefined, progress: 0, message: '처리 중...' },
      };
      return { nodes: updated, completedCount: countCompleted(updated), totalCount: countTotal(updated) };
    });
  },

  completeNode: (nodeId) => {
    set((state) => {
      const prev = state.nodes[nodeId] || { status: 'idle' };
      const updated = {
        ...state.nodes,
        [nodeId]: { ...prev, status: 'completed' as NodeStatus, completedAt: Date.now(), progress: 100, message: '완료' },
      };
      return { nodes: updated, completedCount: countCompleted(updated), totalCount: countTotal(updated) };
    });
  },

  failNode: (nodeId, error) => {
    set((state) => {
      const prev = state.nodes[nodeId] || { status: 'idle' };
      const updated = {
        ...state.nodes,
        [nodeId]: { ...prev, status: 'error' as NodeStatus, message: error },
      };
      return { nodes: updated, completedCount: countCompleted(updated), totalCount: countTotal(updated) };
    });
  },

  resetAll: () => {
    const reset: Record<string, NodeState> = {};
    for (const key of Object.keys(get().nodes)) {
      reset[key] = { status: 'idle' };
    }
    set({ nodes: reset, isRunning: false, completedCount: 0, totalCount: countTotal(reset) });
  },

  startPipeline: () => {
    const { isRunning } = get();
    if (isRunning) return;

    // 1. Set all nodes to pending
    const allPending: Record<string, NodeState> = {};
    for (const key of Object.keys(get().nodes)) {
      allPending[key] = { status: 'pending' };
    }
    set({ nodes: allPending, isRunning: true, completedCount: 0, totalCount: countTotal(allPending) });

    const { startNode, completeNode } = get();

    // 2. After 500ms, start 'prd'
    setTimeout(() => {
      startNode('prd');
    }, 500);

    // 3. After 2s, complete 'prd', start 'codegen' and 'ai-model'
    setTimeout(() => {
      completeNode('prd');
      startNode('codegen');
      startNode('ai-model');
    }, 2500);

    // Simulate codegen progress
    const progressIntervals = [
      { delay: 3000, progress: 15 },
      { delay: 3500, progress: 30 },
      { delay: 4000, progress: 45 },
      { delay: 4500, progress: 58 },
      { delay: 5000, progress: 72 },
      { delay: 5500, progress: 85 },
      { delay: 6500, progress: 95 },
    ];
    for (const { delay, progress } of progressIntervals) {
      setTimeout(() => {
        get().setNodeProgress('codegen', progress);
        get().setNodeStatus('codegen', 'in-progress', `${Math.round(progress * 25 / 100)}/25 파일 생성 중...`);
      }, delay);
    }

    // 4. After 5s total (2.5s + 2.5s), complete 'codegen' and 'ai-model', start 'filesystem'
    setTimeout(() => {
      completeNode('codegen');
      completeNode('ai-model');
      startNode('filesystem');
    }, 7500);

    // 5. After 2s, complete 'filesystem', start 'security'
    setTimeout(() => {
      completeNode('filesystem');
      startNode('security');
    }, 9500);

    // 6. After 3s, complete 'security', start 'report'
    setTimeout(() => {
      completeNode('security');
      startNode('report');
    }, 12500);

    // 7. After 1s, complete 'report', start 'deploy'
    setTimeout(() => {
      completeNode('report');
      startNode('deploy');
    }, 13500);

    // 8. After 2s, complete 'deploy', start 'preview'
    setTimeout(() => {
      completeNode('deploy');
      startNode('preview');
    }, 15500);

    // 9. After 1s, complete 'preview'
    setTimeout(() => {
      completeNode('preview');
      set({ isRunning: false });
    }, 16500);
  },
}));
