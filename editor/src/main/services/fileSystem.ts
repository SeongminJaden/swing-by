import { ipcMain, dialog, BrowserWindow } from 'electron';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

function resolvePath(filePath: string): string {
  if (filePath.startsWith('~/') || filePath === '~') {
    return path.join(os.homedir(), filePath.slice(1));
  }
  return filePath;
}

export interface FileNode {
  name: string;
  path: string;
  type: 'file' | 'directory';
  children?: FileNode[];
}

const IGNORED = new Set([
  'node_modules', '.git', '.next', '.nuxt', 'dist', 'build',
  '.cache', '.turbo', '__pycache__', '.venv', 'venv',
  '.DS_Store', 'Thumbs.db', '.env.local',
]);

function readDirRecursive(dirPath: string, depth = 0, maxDepth = 5): FileNode[] {
  if (depth > maxDepth) return [];

  try {
    const entries = fs.readdirSync(dirPath, { withFileTypes: true });
    const nodes: FileNode[] = [];

    // Directories first, then files
    const dirs = entries.filter(e => e.isDirectory() && !IGNORED.has(e.name) && !e.name.startsWith('.'));
    const files = entries.filter(e => e.isFile() && !IGNORED.has(e.name));

    for (const dir of dirs.sort((a, b) => a.name.localeCompare(b.name))) {
      const fullPath = path.join(dirPath, dir.name);
      nodes.push({
        name: dir.name,
        path: fullPath,
        type: 'directory',
        children: readDirRecursive(fullPath, depth + 1, maxDepth),
      });
    }

    for (const file of files.sort((a, b) => a.name.localeCompare(b.name))) {
      nodes.push({
        name: file.name,
        path: path.join(dirPath, file.name),
        type: 'file',
      });
    }

    return nodes;
  } catch {
    return [];
  }
}

export function registerFileSystemHandlers() {
  // Open folder dialog
  ipcMain.handle('fs:openFolder', async () => {
    const win = BrowserWindow.getFocusedWindow();
    if (!win) return null;

    const result = await dialog.showOpenDialog(win, {
      properties: ['openDirectory'],
      title: '폴더 열기',
    });

    if (result.canceled || !result.filePaths[0]) return null;
    return result.filePaths[0];
  });

  // Read directory tree
  ipcMain.handle('fs:readDir', (_event, dirPath: string) => {
    const resolved = resolvePath(dirPath);
    if (!fs.existsSync(resolved)) return [];
    return readDirRecursive(resolved);
  });

  // Read file content
  ipcMain.handle('fs:readFile', (_event, filePath: string) => {
    try {
      return fs.readFileSync(resolvePath(filePath), 'utf-8');
    } catch {
      return null;
    }
  });

  // Write file
  ipcMain.handle('fs:writeFile', (_event, filePath: string, content: string) => {
    try {
      const resolved = resolvePath(filePath);
      const dir = path.dirname(resolved);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
      fs.writeFileSync(resolved, content, 'utf-8');
      return true;
    } catch {
      return false;
    }
  });

  // Create directory
  ipcMain.handle('fs:mkdir', (_event, dirPath: string) => {
    try {
      fs.mkdirSync(resolvePath(dirPath), { recursive: true });
      return true;
    } catch {
      return false;
    }
  });

  // Delete file/directory
  ipcMain.handle('fs:delete', (_event, targetPath: string) => {
    try {
      const stat = fs.statSync(targetPath);
      if (stat.isDirectory()) {
        fs.rmSync(targetPath, { recursive: true });
      } else {
        fs.unlinkSync(targetPath);
      }
      return true;
    } catch {
      return false;
    }
  });

  // Check if path exists
  ipcMain.handle('fs:exists', (_event, targetPath: string) => {
    return fs.existsSync(targetPath);
  });

  // Get file info
  ipcMain.handle('fs:stat', (_event, targetPath: string) => {
    try {
      const stat = fs.statSync(targetPath);
      return {
        size: stat.size,
        isDirectory: stat.isDirectory(),
        isFile: stat.isFile(),
        modified: stat.mtimeMs,
        created: stat.birthtimeMs,
      };
    } catch {
      return null;
    }
  });
}
