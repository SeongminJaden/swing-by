import { create } from 'zustand';

export interface FileNode {
  name: string;
  path: string;
  type: 'file' | 'directory';
  children?: FileNode[];
}

export interface OpenFile {
  path: string;
  name: string;
  content: string;
  language: string;
  modified: boolean;
}

interface FileState {
  // Workspace
  workspacePath: string | null;
  fileTree: FileNode[];
  loading: boolean;

  // Open files / tabs
  openFiles: OpenFile[];
  activeFilePath: string | null;

  // Actions
  openFolder: () => Promise<void>;
  loadFolder: (dirPath: string) => Promise<void>;
  refreshTree: () => Promise<void>;
  openFile: (filePath: string) => Promise<void>;
  closeFile: (filePath: string) => void;
  setActiveFile: (filePath: string) => void;
  updateFileContent: (filePath: string, content: string) => void;
  saveFile: (filePath: string) => Promise<boolean>;
  saveActiveFile: () => Promise<boolean>;
}

function getLanguage(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescriptreact',
    js: 'javascript', jsx: 'javascriptreact',
    json: 'json', md: 'markdown',
    css: 'css', scss: 'scss', less: 'less',
    html: 'html', xml: 'xml', svg: 'xml',
    py: 'python', rb: 'ruby', go: 'go',
    rs: 'rust', java: 'java', kt: 'kotlin',
    c: 'c', cpp: 'cpp', h: 'c',
    sh: 'shell', bash: 'shell', zsh: 'shell',
    yml: 'yaml', yaml: 'yaml', toml: 'toml',
    sql: 'sql', graphql: 'graphql',
    dockerfile: 'dockerfile',
    env: 'plaintext', gitignore: 'plaintext',
  };
  return map[ext] || 'plaintext';
}

export const useFileStore = create<FileState>((set, get) => ({
  workspacePath: null,
  fileTree: [],
  loading: false,
  openFiles: [],
  activeFilePath: null,

  openFolder: async () => {
    const api = window.electronAPI;
    if (!api) return;

    const dirPath = await api.openFolder();
    if (!dirPath) return;

    await get().loadFolder(dirPath);
  },

  loadFolder: async (dirPath: string) => {
    const api = window.electronAPI;
    if (!api) return;

    set({ loading: true, workspacePath: dirPath });
    const tree = await api.readDir(dirPath);
    set({ fileTree: tree, loading: false, openFiles: [], activeFilePath: null });
  },

  refreshTree: async () => {
    const { workspacePath } = get();
    const api = window.electronAPI;
    if (!api || !workspacePath) return;

    const tree = await api.readDir(workspacePath);
    set({ fileTree: tree });
  },

  openFile: async (filePath: string) => {
    const { openFiles } = get();
    const api = window.electronAPI;
    if (!api) return;

    // Already open → just switch tab
    const existing = openFiles.find(f => f.path === filePath);
    if (existing) {
      set({ activeFilePath: filePath });
      return;
    }

    // Read file content
    const content = await api.readFile(filePath);
    if (content === null) return;

    const name = filePath.split('/').pop() || filePath;
    const newFile: OpenFile = {
      path: filePath,
      name,
      content,
      language: getLanguage(name),
      modified: false,
    };

    set({
      openFiles: [...openFiles, newFile],
      activeFilePath: filePath,
    });
  },

  closeFile: (filePath: string) => {
    const { openFiles, activeFilePath } = get();
    const filtered = openFiles.filter(f => f.path !== filePath);
    let newActive = activeFilePath;

    if (activeFilePath === filePath) {
      newActive = filtered.length > 0 ? filtered[filtered.length - 1].path : null;
    }

    set({ openFiles: filtered, activeFilePath: newActive });
  },

  setActiveFile: (filePath: string) => {
    set({ activeFilePath: filePath });
  },

  updateFileContent: (filePath: string, content: string) => {
    set({
      openFiles: get().openFiles.map(f =>
        f.path === filePath ? { ...f, content, modified: true } : f
      ),
    });
  },

  saveFile: async (filePath: string) => {
    const api = window.electronAPI;
    if (!api) return false;

    const file = get().openFiles.find(f => f.path === filePath);
    if (!file) return false;

    const success = await api.writeFile(filePath, file.content);
    if (success) {
      set({
        openFiles: get().openFiles.map(f =>
          f.path === filePath ? { ...f, modified: false } : f
        ),
      });
    }
    return success;
  },

  saveActiveFile: async () => {
    const { activeFilePath } = get();
    if (!activeFilePath) return false;
    return get().saveFile(activeFilePath);
  },
}));
