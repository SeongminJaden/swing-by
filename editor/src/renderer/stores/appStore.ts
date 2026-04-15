import { create } from 'zustand';

export type AppView = 'ide' | 'settings' | 'deploy' | 'watchdog' | 'newService' | 'agents';
export type SidebarTab = 'files' | 'git' | 'deploy' | 'watchdog' | 'devenv' | 'agents' | 'settings';
export type BottomTab = 'console' | 'network' | 'problems' | 'terminal';
export type Theme = 'dark' | 'light' | 'monokai';
export type Language = 'ko' | 'en';

interface AppState {
  // Navigation
  currentView: AppView;
  setCurrentView: (view: AppView) => void;

  // Sidebar
  sidebarTab: SidebarTab;
  setSidebarTab: (tab: SidebarTab) => void;
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;

  // Bottom panel
  bottomTab: BottomTab;
  setBottomTab: (tab: BottomTab) => void;
  bottomPanelVisible: boolean;
  toggleBottomPanel: () => void;

  // AI Chat panel
  chatPanelVisible: boolean;
  toggleChatPanel: () => void;

  // Preview panel
  previewVisible: boolean;
  togglePreview: () => void;

  // Theme & Language
  theme: Theme;
  setTheme: (theme: Theme) => void;
  language: Language;
  setLanguage: (lang: Language) => void;

  // Workflow view toggle
  workflowView: boolean;
  toggleWorkflowView: () => void;

  // Agent detail overlay
  selectedAgentId: string | null;
  setSelectedAgentId: (id: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentView: 'ide',
  setCurrentView: (view) => set({ currentView: view }),

  sidebarTab: 'agents',
  setSidebarTab: (tab) => set({ sidebarTab: tab }),
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),

  bottomTab: 'console',
  setBottomTab: (tab) => set({ bottomTab: tab }),
  bottomPanelVisible: false,
  toggleBottomPanel: () => set((s) => ({ bottomPanelVisible: !s.bottomPanelVisible })),

  chatPanelVisible: true,
  toggleChatPanel: () => set((s) => ({ chatPanelVisible: !s.chatPanelVisible })),

  previewVisible: false,
  togglePreview: () => set((s) => ({ previewVisible: !s.previewVisible })),

  theme: 'dark',
  setTheme: (theme) => set({ theme }),
  language: 'en',
  setLanguage: (lang) => set({ language: lang }),

  workflowView: false,
  toggleWorkflowView: () => set((s) => ({ workflowView: !s.workflowView })),

  selectedAgentId: null,
  setSelectedAgentId: (id) => set({ selectedAgentId: id }),
}));

// Expose store for testing/debugging
(window as any).__appStore = useAppStore;
