import { ipcMain, BrowserWindow } from 'electron';
import * as os from 'os';

// node-pty is a native module, require it dynamically
let pty: any;
try {
  pty = require('node-pty');
} catch (e) {
  console.error('node-pty not available:', e);
}

const terminals: Map<string, any> = new Map();
let terminalCounter = 0;

function getShell(): string {
  if (process.platform === 'win32') return 'powershell.exe';
  return process.env.SHELL || '/bin/bash';
}

export function registerTerminalHandlers() {
  if (!pty) {
    console.warn('Terminal: node-pty not loaded, terminal will not work');
    return;
  }

  // Create a new terminal
  ipcMain.handle('terminal:create', (_event, cwd?: string) => {
    const id = `term_${++terminalCounter}`;
    const shell = getShell();

    const term = pty.spawn(shell, [], {
      name: 'xterm-256color',
      cols: 80,
      rows: 24,
      cwd: cwd || os.homedir(),
      env: { ...process.env, TERM: 'xterm-256color' },
    });

    terminals.set(id, term);

    // Forward terminal output to renderer
    const win = BrowserWindow.getFocusedWindow();
    term.onData((data: string) => {
      win?.webContents.send('terminal:data', id, data);
    });

    term.onExit(({ exitCode }: { exitCode: number }) => {
      terminals.delete(id);
      win?.webContents.send('terminal:exit', id, exitCode);
    });

    return id;
  });

  // Write data to terminal
  ipcMain.on('terminal:write', (_event, id: string, data: string) => {
    const term = terminals.get(id);
    if (term) term.write(data);
  });

  // Resize terminal
  ipcMain.on('terminal:resize', (_event, id: string, cols: number, rows: number) => {
    const term = terminals.get(id);
    if (term) {
      try {
        term.resize(cols, rows);
      } catch {
        // Ignore resize errors
      }
    }
  });

  // Kill terminal
  ipcMain.on('terminal:kill', (_event, id: string) => {
    const term = terminals.get(id);
    if (term) {
      term.kill();
      terminals.delete(id);
    }
  });
}
