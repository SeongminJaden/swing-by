/**
 * Rust ai_agent IPC bridge
 *
 * Spawns `ai_agent --ipc-stdio` as a child process and communicates
 * via JSON-RPC 2.0 over stdin/stdout.
 */

import { ipcMain, BrowserWindow } from 'electron';
import { spawn, ChildProcess } from 'child_process';
import path from 'path';

interface PendingRequest {
  resolve: (value: any) => void;
  reject: (err: Error) => void;
}

let agentProcess: ChildProcess | null = null;
let requestId = 1;
const pending = new Map<number, PendingRequest>();
let lineBuffer = '';
let mainWin: BrowserWindow | null = null;

function getAgentBinaryPath(): string {
  const isWin = process.platform === 'win32';
  const bin = isWin ? 'ai_agent.exe' : 'ai_agent';
  const { existsSync } = require('fs');

  // Packaged app: binary is in extraResources
  const prodPath = path.join(process.resourcesPath ?? '', bin);
  if (existsSync(prodPath)) return prodPath;

  // Dev: cargo build output
  const devRelease = path.join(__dirname, '../../../../target/release', bin);
  if (existsSync(devRelease)) return devRelease;

  const devDebug = path.join(__dirname, '../../../../target/debug', bin);
  if (existsSync(devDebug)) return devDebug;

  return prodPath; // fallback (will fail gracefully)
}

function ensureAgent(): boolean {
  if (agentProcess && !agentProcess.killed) return true;

  const binaryPath = getAgentBinaryPath();

  // Ensure binary is executable (needed after extraResources unpack)
  if (process.platform !== 'win32') {
    try { require('fs').chmodSync(binaryPath, 0o755); } catch {}
  }

  console.log('[agentBridge] spawning:', binaryPath);
  try {
    agentProcess = spawn(binaryPath, ['--ipc-stdio'], {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env },
    });

    agentProcess.stdout?.on('data', (chunk: Buffer) => {
      lineBuffer += chunk.toString();
      const lines = lineBuffer.split('\n');
      lineBuffer = lines.pop() ?? '';
      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const msg = JSON.parse(trimmed);
          // JSON-RPC response
          if (msg.id !== undefined) {
            const req = pending.get(Number(msg.id));
            if (req) {
              pending.delete(Number(msg.id));
              if (msg.error) {
                req.reject(new Error(msg.error.message));
              } else {
                req.resolve(msg.result);
              }
            }
          }
          // Streaming/notification (no id)
          if (msg.method === 'stream') {
            mainWin?.webContents.send('agent:stream', msg.params?.text ?? '');
          }
          if (msg.method === 'sprint_progress') {
            mainWin?.webContents.send('agent:sprintProgress', msg.params);
          }
        } catch {
          // Non-JSON line — forward as log
          mainWin?.webContents.send('agent:log', trimmed);
        }
      }
    });

    agentProcess.stderr?.on('data', (chunk: Buffer) => {
      const text = chunk.toString();
      mainWin?.webContents.send('agent:log', text);
    });

    agentProcess.on('exit', (code) => {
      agentProcess = null;
      mainWin?.webContents.send('agent:exit', code);
      // Reject any pending requests
      for (const [, req] of pending) {
        req.reject(new Error('Agent process exited'));
      }
      pending.clear();
    });

    return true;
  } catch (e: any) {
    console.error('[agentBridge] Failed to spawn agent:', binaryPath, e.message);
    mainWin?.webContents.send('agent:log', `[ERROR] Failed to start agent: ${e.message} (path: ${binaryPath})`);
    return false;
  }
}

function sendRpc(method: string, params: Record<string, unknown>): Promise<any> {
  return new Promise((resolve, reject) => {
    if (!ensureAgent()) {
      return reject(new Error('AI Agent binary not found. Please reinstall the app.'));
    }

    const id = requestId++;
    pending.set(id, { resolve, reject });

    const req = JSON.stringify({ jsonrpc: '2.0', id, method, params });
    agentProcess!.stdin!.write(req + '\n');

    // Timeout
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error(`RPC timeout: ${method}`));
      }
    }, 60_000);
  });
}

export function setAgentBridgeWindow(win: BrowserWindow) {
  mainWin = win;
}

export function registerAgentBridgeHandlers() {
  // Ping / health check
  ipcMain.handle('agent:ping', async () => {
    try {
      const result = await sendRpc('ping', {});
      return { success: true, ...result };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Initialize handshake
  ipcMain.handle('agent:init', async () => {
    try {
      const result = await sendRpc('initialize', {});
      return { success: true, ...result };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // General chat
  ipcMain.handle('agent:chat', async (_event, prompt: string, callerId = 'editor-ui') => {
    try {
      const result = await sendRpc('chat', { prompt, caller_id: callerId });
      return { success: true, content: result?.content ?? '' };
    } catch (e: any) {
      return { success: false, content: '', error: e.message };
    }
  });

  // Run agile sprint
  ipcMain.handle('agent:sprintRun', async (_event, project: string, request: string) => {
    try {
      const result = await sendRpc('agile_sprint', { project, request });
      return { success: true, ...result };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Board status
  ipcMain.handle('agent:boardStatus', async (_event, project: string) => {
    try {
      const result = await sendRpc('board_status', { project });
      return { success: true, board: result?.board ?? '' };
    } catch (e: any) {
      return { success: false, board: '', error: e.message };
    }
  });

  // Capabilities list
  ipcMain.handle('agent:capabilities', async () => {
    try {
      const result = await sendRpc('capabilities', {});
      return { success: true, capabilities: result?.capabilities ?? [] };
    } catch (e: any) {
      return { success: false, capabilities: [], error: e.message };
    }
  });

  // Kill agent process
  ipcMain.handle('agent:kill', async () => {
    if (agentProcess && !agentProcess.killed) {
      agentProcess.kill();
      agentProcess = null;
    }
    return { success: true };
  });

  // Check if binary exists
  ipcMain.handle('agent:checkBinary', async () => {
    const { existsSync } = require('fs');
    const p = getAgentBinaryPath();
    return { exists: existsSync(p), path: p };
  });
}
