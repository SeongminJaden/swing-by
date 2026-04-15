import { app, BrowserWindow, ipcMain, Menu } from 'electron';
import path from 'path';
import { execSync } from 'child_process';
import { registerFileSystemHandlers } from './services/fileSystem';
import { registerTerminalHandlers } from './services/terminal';
import { registerAIHandlers } from './services/ai';
import { registerGitHandlers } from './services/git';
import { registerSecurityHandlers } from './services/security';
import { registerDeployHandlers } from './services/deploy';
import { registerMonitoringHandlers } from './services/monitoring';
import { registerErrorTrackingHandlers } from './services/errorTracking';
import { registerCostTrackingHandlers } from './services/costTracker';
import { registerAuthHandlers, handleAuthCallback, setMainWindow } from './services/auth';
import { registerTeamHandlers } from './services/team';
import { registerUpdaterHandlers } from './services/updater';
import { registerPaymentHandlers } from './services/payment';
import { registerConnectionHandlers } from './services/connections';
import { registerAgentBridgeHandlers, setAgentBridgeWindow } from './services/agentBridge';

if (process.env.NODE_ENV === 'development') {
  app.commandLine.appendSwitch('remote-debugging-port', '9222');
}

// Register custom protocol for OAuth callbacks
app.setAsDefaultProtocolClient('videplace');

// Request single instance lock so second-instance event fires on Linux/Windows
const gotTheLock = app.requestSingleInstanceLock();
if (!gotTheLock) {
  app.quit();
}

let mainWindow: BrowserWindow | null = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 1024,
    minHeight: 700,
    title: 'VidEplace',
    frame: false,
    backgroundColor: '#0d1117',
    webPreferences: {
      preload: path.join(__dirname, '../preload/preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      webviewTag: true,
    },
  });

  Menu.setApplicationMenu(null);
  mainWindow.maximize();
  mainWindow.focus();

  if (process.env.NODE_ENV === 'development') {
    mainWindow.loadURL('http://localhost:5173');
  } else {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
  }

  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  // Pass window reference to auth service for event emission
  setMainWindow(mainWindow);
  setAgentBridgeWindow(mainWindow);
}

// Handle OAuth callback on macOS (open-url)
app.on('open-url', (_event, url) => {
  if (url.startsWith('videplace://')) {
    handleAuthCallback(url);
  }
});

// Handle OAuth callback on Linux/Windows (second-instance)
app.on('second-instance', (_event, argv) => {
  const url = argv.find((a) => a.startsWith('videplace://'));
  if (url) {
    handleAuthCallback(url);
  }
  // Focus the existing window
  if (mainWindow) {
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.focus();
  }
});

app.whenReady().then(() => {
  registerFileSystemHandlers();
  registerTerminalHandlers();
  registerAIHandlers();
  registerGitHandlers();
  registerSecurityHandlers();
  registerDeployHandlers();
  registerMonitoringHandlers();
  registerErrorTrackingHandlers();
  registerCostTrackingHandlers();
  registerAuthHandlers();
  registerTeamHandlers();
  registerUpdaterHandlers();
  registerPaymentHandlers();
  registerConnectionHandlers();
  registerAgentBridgeHandlers();
  createWindow();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('activate', () => {
  if (mainWindow === null) createWindow();
});

// IPC handlers
ipcMain.handle('get-app-info', () => ({
  version: app.getVersion(),
  platform: process.platform,
  appPath: app.getAppPath(),
}));

// Window controls
ipcMain.on('window-minimize', () => mainWindow?.minimize());
ipcMain.on('window-maximize', () => {
  if (mainWindow?.isMaximized()) {
    mainWindow.unmaximize();
  } else {
    mainWindow?.maximize();
  }
});
ipcMain.on('window-close', () => mainWindow?.close());

// Dev test: execute JS in renderer
ipcMain.handle('test:eval', (_event, code: string) => {
  try {
    mainWindow?.webContents.executeJavaScript(code);
    return { success: true };
  } catch (e: any) {
    return { success: false, error: e.message };
  }
});

// Execute shell command (for dev environment detection)
ipcMain.handle('exec:command', (_event, cmd: string) => {
  try {
    const output = execSync(cmd, { encoding: 'utf-8', timeout: 5000 }).trim();
    return { success: true, output };
  } catch {
    return { success: false, output: '' };
  }
});
