import { create } from 'zustand';

export interface Project {
  id: string;
  name: string;
  path: string;
  framework: string;
  status: 'dev' | 'verifying' | 'deployed' | 'error';
  deployUrl?: string;
  lastActivity: string;
  createdAt: string;
}

interface ProjectState {
  projects: Project[];
  currentProject: Project | null;
  loaded: boolean;
  setCurrentProject: (project: Project | null) => void;
  addProject: (project: Project) => void;
  removeProject: (id: string) => void;
  loadProjects: () => Promise<void>;
  saveProjects: () => Promise<void>;
}


const PROJECTS_PATH = '~/.videplace/projects.json';
const SETTINGS_PATH = '~/.videplace/settings.json';

function getAPI(): any {
  return (window as any).electronAPI;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projects: [],
  currentProject: null,
  loaded: false,

  loadProjects: async () => {
    const api = getAPI();
    if (!api?.readFile) {
      set({ loaded: true });
      return;
    }

    try {
      // Ensure directory exists
      if (api.mkdir) {
        await api.mkdir('~/.videplace').catch(() => {});
      }

      const raw = await api.readFile(PROJECTS_PATH);
      if (raw) {
        const data = JSON.parse(raw);
        if (Array.isArray(data) && data.length > 0) {
          set({ projects: data, loaded: true });
        } else {
          set({ loaded: true });
        }
      } else {
        set({ loaded: true });
      }

      // Restore last opened project
      const settingsRaw = await api.readFile(SETTINGS_PATH).catch(() => null);
      if (settingsRaw) {
        const settings = JSON.parse(settingsRaw);
        if (settings.lastProjectId) {
          const projects = get().projects;
          const last = projects.find((p) => p.id === settings.lastProjectId);
          if (last) {
            set({ currentProject: last });
          }
        }
      }
    } catch {
      // On any error, keep demo projects
      set({ loaded: true });
    }
  },

  saveProjects: async () => {
    const api = getAPI();
    if (!api?.writeFile) return;

    try {
      if (api.mkdir) {
        await api.mkdir('~/.videplace').catch(() => {});
      }
      const data = JSON.stringify(get().projects, null, 2);
      await api.writeFile(PROJECTS_PATH, data);
    } catch {
      // Best-effort save
    }
  },

  setCurrentProject: (project) => {
    set({ currentProject: project });
    // Persist last opened project
    const api = getAPI();
    if (api?.readFile && api?.writeFile && project) {
      (async () => {
        try {
          const raw = await api.readFile(SETTINGS_PATH).catch(() => null);
          const settings = raw ? JSON.parse(raw) : {};
          settings.lastProjectId = project.id;
          await api.writeFile(SETTINGS_PATH, JSON.stringify(settings, null, 2));
        } catch {
          // ignore
        }
      })();
    }
  },

  addProject: (project) => {
    set((s) => ({ projects: [...s.projects, project] }));
    // Save after state update
    setTimeout(() => get().saveProjects(), 0);
  },

  removeProject: (id) => {
    set((s) => ({ projects: s.projects.filter((p) => p.id !== id) }));
    // Save after state update
    setTimeout(() => get().saveProjects(), 0);
  },
}));
