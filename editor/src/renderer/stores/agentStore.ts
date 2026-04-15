import { create } from 'zustand';

export type AgentStatus = 'idle' | 'running' | 'done' | 'error';

export interface AgentDef {
  id: string;
  name: string;
  role: string;
  description: string;
  emoji: string;
  color: string;
}

export interface AgentState {
  status: AgentStatus;
  lastOutput: string;
  logs: string[];
  startedAt?: number;
  completedAt?: number;
  progress?: number;
}

export const AGENT_DEFINITIONS: AgentDef[] = [
  { id: 'product_owner',     name: 'Product Owner',      role: 'Planning',     description: 'Defines requirements, sprint goals, and prioritizes the backlog.',                  emoji: '📋', color: '#6366f1' },
  { id: 'business_analyst',  name: 'Business Analyst',   role: 'Planning',     description: 'Analyzes business needs and translates them into technical requirements.',          emoji: '📊', color: '#8b5cf6' },
  { id: 'scrum_master',      name: 'Scrum Master',       role: 'Planning',     description: 'Facilitates agile ceremonies, removes blockers, drives retrospectives.',            emoji: '🏃', color: '#a78bfa' },
  { id: 'ux_designer',       name: 'UX Designer',        role: 'Design',       description: 'Designs user flows, wireframes, and ensures optimal user experience.',             emoji: '🎨', color: '#ec4899' },
  { id: 'architect',         name: 'Architect',          role: 'Design',       description: 'Designs system architecture, selects tech stack, reviews code structure.',         emoji: '🏗️', color: '#f59e0b' },
  { id: 'developer',         name: 'Developer',          role: 'Development',  description: 'Implements features, writes production code, fixes bugs.',                         emoji: '💻', color: '#10b981' },
  { id: 'tech_lead',         name: 'Tech Lead',          role: 'Development',  description: 'Reviews code, mentors developers, ensures technical standards.',                   emoji: '🔧', color: '#14b8a6' },
  { id: 'qa_engineer',       name: 'QA Engineer',        role: 'Quality',      description: 'Writes test cases, performs quality assurance, finds and reports bugs.',           emoji: '🧪', color: '#22c55e' },
  { id: 'hacker_agent',      name: 'Security Hacker',    role: 'Security',     description: 'White-hat security testing, vulnerability scanning, penetration testing.',        emoji: '🔐', color: '#ef4444' },
  { id: 'devops_engineer',   name: 'DevOps Engineer',    role: 'Release',      description: 'Manages CI/CD pipelines, infrastructure, and deployment automation.',             emoji: '🚀', color: '#3b82f6' },
  { id: 'sre',               name: 'SRE',                role: 'Release',      description: 'Site reliability engineering, SLO/SLI monitoring, postmortem analysis.',         emoji: '📡', color: '#0ea5e9' },
  { id: 'release_manager',   name: 'Release Manager',    role: 'Release',      description: 'Coordinates releases, manages changelogs, controls deployment gates.',            emoji: '📦', color: '#06b6d4' },
  { id: 'technical_writer',  name: 'Technical Writer',   role: 'Release',      description: 'Writes documentation, API references, user guides, and release notes.',          emoji: '📝', color: '#64748b' },
];

// Pipeline stage → agents mapping
export const PIPELINE_STAGES = [
  { id: 'planning',    label: 'Planning',    agents: ['product_owner', 'business_analyst', 'scrum_master'] },
  { id: 'design',      label: 'Design',      agents: ['ux_designer', 'architect'] },
  { id: 'development', label: 'Development', agents: ['developer', 'tech_lead'] },
  { id: 'quality',     label: 'Quality',     agents: ['qa_engineer'] },
  { id: 'security',    label: 'Security',    agents: ['hacker_agent'] },
  { id: 'release',     label: 'Release',     agents: ['devops_engineer', 'sre', 'release_manager', 'technical_writer'] },
];

// Edge connections between pipeline stages
export const PIPELINE_EDGES: Array<{ from: string; to: string }> = [
  { from: 'product_owner',    to: 'business_analyst' },
  { from: 'business_analyst', to: 'scrum_master' },
  { from: 'scrum_master',     to: 'ux_designer' },
  { from: 'scrum_master',     to: 'architect' },
  { from: 'ux_designer',      to: 'developer' },
  { from: 'architect',        to: 'developer' },
  { from: 'developer',        to: 'tech_lead' },
  { from: 'tech_lead',        to: 'qa_engineer' },
  { from: 'qa_engineer',      to: 'hacker_agent' },
  { from: 'hacker_agent',     to: 'devops_engineer' },
  { from: 'devops_engineer',  to: 'sre' },
  { from: 'sre',              to: 'release_manager' },
  { from: 'release_manager',  to: 'technical_writer' },
];

interface AgentStoreState {
  agents: Record<string, AgentState>;
  sprintRunning: boolean;
  sprintProject: string;
  sprintRequest: string;
  boardOutput: string;
  binaryAvailable: boolean | null;

  setAgentStatus: (id: string, status: AgentStatus) => void;
  appendAgentLog: (id: string, text: string) => void;
  setAgentProgress: (id: string, progress: number) => void;
  setAgentOutput: (id: string, output: string) => void;
  resetAgent: (id: string) => void;
  resetAll: () => void;

  setSprintRunning: (v: boolean) => void;
  setSprintProject: (v: string) => void;
  setSprintRequest: (v: string) => void;
  setBoardOutput: (v: string) => void;
  setBinaryAvailable: (v: boolean) => void;
}

const defaultAgentState = (): AgentState => ({
  status: 'idle',
  lastOutput: '',
  logs: [],
});

const initialAgents: Record<string, AgentState> = {};
for (const def of AGENT_DEFINITIONS) {
  initialAgents[def.id] = defaultAgentState();
}

export const useAgentStore = create<AgentStoreState>((set) => ({
  agents: initialAgents,
  sprintRunning: false,
  sprintProject: 'my_project',
  sprintRequest: '',
  boardOutput: '',
  binaryAvailable: null,

  setAgentStatus: (id, status) =>
    set((s) => ({
      agents: {
        ...s.agents,
        [id]: {
          ...s.agents[id],
          status,
          startedAt: status === 'running' ? Date.now() : s.agents[id]?.startedAt,
          completedAt: (status === 'done' || status === 'error') ? Date.now() : s.agents[id]?.completedAt,
        },
      },
    })),

  appendAgentLog: (id, text) =>
    set((s) => ({
      agents: {
        ...s.agents,
        [id]: {
          ...s.agents[id],
          logs: [...(s.agents[id]?.logs ?? []).slice(-199), text],
          lastOutput: text,
        },
      },
    })),

  setAgentProgress: (id, progress) =>
    set((s) => ({
      agents: { ...s.agents, [id]: { ...s.agents[id], progress } },
    })),

  setAgentOutput: (id, output) =>
    set((s) => ({
      agents: { ...s.agents, [id]: { ...s.agents[id], lastOutput: output } },
    })),

  resetAgent: (id) =>
    set((s) => ({ agents: { ...s.agents, [id]: defaultAgentState() } })),

  resetAll: () => set({ agents: { ...initialAgents } }),

  setSprintRunning: (v) => set({ sprintRunning: v }),
  setSprintProject: (v) => set({ sprintProject: v }),
  setSprintRequest: (v) => set({ sprintRequest: v }),
  setBoardOutput: (v) => set({ boardOutput: v }),
  setBinaryAvailable: (v) => set({ binaryAvailable: v }),
}));
