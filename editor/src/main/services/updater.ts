import { ipcMain, app, BrowserWindow } from 'electron';
import https from 'https';
import fs from 'fs';
import path from 'path';
import os from 'os';

// Types
interface ReleaseInfo {
  version: string;
  date: string;
  notes: string;
  url: string;
}

interface ChangelogStore {
  entries: ReleaseInfo[];
  lastChecked: string | null;
}

interface UpdaterSettings {
  autoUpdateEnabled: boolean;
}

// Constants
const DATA_DIR = path.join(os.homedir(), '.videplace');
const CHANGELOG_FILE = path.join(DATA_DIR, 'changelog.json');
const SETTINGS_FILE = path.join(DATA_DIR, 'settings.json');
const GITHUB_REPO = 'SeongminJaden/videplace';
const RELEASES_URL = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
const ALL_RELEASES_URL = `https://api.github.com/repos/${GITHUB_REPO}/releases`;

// Helpers
function ensureDataDir(): void {
  if (!fs.existsSync(DATA_DIR)) {
    fs.mkdirSync(DATA_DIR, { recursive: true });
  }
}

function readChangelogStore(): ChangelogStore {
  ensureDataDir();
  if (!fs.existsSync(CHANGELOG_FILE)) {
    const empty: ChangelogStore = { entries: [], lastChecked: null };
    fs.writeFileSync(CHANGELOG_FILE, JSON.stringify(empty, null, 2), 'utf-8');
    return empty;
  }
  try {
    const raw = fs.readFileSync(CHANGELOG_FILE, 'utf-8');
    return JSON.parse(raw) as ChangelogStore;
  } catch {
    return { entries: [], lastChecked: null };
  }
}

function writeChangelogStore(store: ChangelogStore): void {
  ensureDataDir();
  fs.writeFileSync(CHANGELOG_FILE, JSON.stringify(store, null, 2), 'utf-8');
}

function readSettings(): UpdaterSettings {
  ensureDataDir();
  if (!fs.existsSync(SETTINGS_FILE)) {
    return { autoUpdateEnabled: true };
  }
  try {
    const raw = fs.readFileSync(SETTINGS_FILE, 'utf-8');
    const parsed = JSON.parse(raw);
    return {
      autoUpdateEnabled: parsed.autoUpdateEnabled !== false,
    };
  } catch {
    return { autoUpdateEnabled: true };
  }
}

function writeSettings(settings: UpdaterSettings): void {
  ensureDataDir();
  let existing: Record<string, any> = {};
  if (fs.existsSync(SETTINGS_FILE)) {
    try {
      existing = JSON.parse(fs.readFileSync(SETTINGS_FILE, 'utf-8'));
    } catch {
      // ignore
    }
  }
  const merged = { ...existing, ...settings };
  fs.writeFileSync(SETTINGS_FILE, JSON.stringify(merged, null, 2), 'utf-8');
}

function fetchJSON(url: string): Promise<any> {
  return new Promise((resolve, reject) => {
    const options = {
      headers: {
        'User-Agent': `VidEplace/${app.getVersion()}`,
        Accept: 'application/vnd.github.v3+json',
      },
    };

    https
      .get(url, options, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          if (res.headers.location) {
            fetchJSON(res.headers.location).then(resolve).catch(reject);
            return;
          }
        }

        let data = '';
        res.on('data', (chunk: Buffer) => {
          data += chunk.toString();
        });
        res.on('end', () => {
          try {
            resolve(JSON.parse(data));
          } catch (e) {
            reject(new Error(`Failed to parse response: ${data.substring(0, 200)}`));
          }
        });
      })
      .on('error', reject);
  });
}

function compareVersions(a: string, b: string): number {
  const cleanA = a.replace(/^v/, '');
  const cleanB = b.replace(/^v/, '');

  const partsA = cleanA.split(/[-.]/).map((p) => {
    const n = parseInt(p, 10);
    return isNaN(n) ? p : n;
  });
  const partsB = cleanB.split(/[-.]/).map((p) => {
    const n = parseInt(p, 10);
    return isNaN(n) ? p : n;
  });

  const len = Math.max(partsA.length, partsB.length);
  for (let i = 0; i < len; i++) {
    const valA = partsA[i] ?? 0;
    const valB = partsB[i] ?? 0;

    if (typeof valA === 'number' && typeof valB === 'number') {
      if (valA < valB) return -1;
      if (valA > valB) return 1;
      continue;
    }

    const priority: Record<string, number> = { alpha: 0, beta: 1, rc: 2 };
    const priA = typeof valA === 'string' ? (priority[valA] ?? -1) : 999;
    const priB = typeof valB === 'string' ? (priority[valB] ?? -1) : 999;

    if (priA < priB) return -1;
    if (priA > priB) return 1;
  }

  return 0;
}

function sendToAllWindows(channel: string, ...args: any[]): void {
  const windows = BrowserWindow.getAllWindows();
  for (const win of windows) {
    win.webContents.send(channel, ...args);
  }
}

// electron-updater integration
let autoUpdaterInstance: any = null;
let updateDownloaded = false;

function getAutoUpdater(): any | null {
  if (autoUpdaterInstance) return autoUpdaterInstance;

  try {
    const { autoUpdater } = require('electron-updater');
    autoUpdaterInstance = autoUpdater;

    // Configure for GitHub releases
    autoUpdater.setFeedURL({
      provider: 'github',
      owner: 'SeongminJaden',
      repo: 'videplace',
    });

    autoUpdater.autoDownload = false;
    autoUpdater.autoInstallOnAppQuit = true;

    // Wire up events
    autoUpdater.on('checking-for-update', () => {
      sendToAllWindows('updater:checking');
    });

    autoUpdater.on('update-available', (info: any) => {
      sendToAllWindows('updater:available', {
        version: info.version,
        releaseDate: info.releaseDate,
        releaseNotes: info.releaseNotes,
      });
    });

    autoUpdater.on('update-not-available', (info: any) => {
      sendToAllWindows('updater:notAvailable', {
        version: info.version,
      });
    });

    autoUpdater.on('download-progress', (progress: any) => {
      sendToAllWindows('updater:downloadProgress', {
        percent: progress.percent,
        bytesPerSecond: progress.bytesPerSecond,
        transferred: progress.transferred,
        total: progress.total,
      });
    });

    autoUpdater.on('update-downloaded', (info: any) => {
      updateDownloaded = true;
      sendToAllWindows('updater:downloaded', {
        version: info.version,
        releaseDate: info.releaseDate,
        releaseNotes: info.releaseNotes,
      });
    });

    autoUpdater.on('error', (err: Error) => {
      sendToAllWindows('updater:error', {
        message: err.message,
      });
    });

    return autoUpdater;
  } catch (err) {
    console.warn('electron-updater not available, using GitHub API fallback:', err);
    return null;
  }
}

export function registerUpdaterHandlers(): void {
  // Check for updates - tries electron-updater first, falls back to GitHub API
  ipcMain.handle('updater:checkForUpdates', async () => {
    const currentVersion = app.getVersion();

    // Try electron-updater first
    const autoUpdater = getAutoUpdater();
    if (autoUpdater) {
      try {
        const result = await autoUpdater.checkForUpdates();
        if (result && result.updateInfo) {
          const latestVersion = result.updateInfo.version;
          const available = compareVersions(currentVersion, latestVersion) < 0;

          // Also store in changelog
          const store = readChangelogStore();
          store.lastChecked = new Date().toISOString();
          const existingIndex = store.entries.findIndex((e) => e.version === latestVersion);
          if (existingIndex === -1) {
            store.entries.unshift({
              version: latestVersion,
              date: result.updateInfo.releaseDate || new Date().toISOString(),
              notes:
                typeof result.updateInfo.releaseNotes === 'string'
                  ? result.updateInfo.releaseNotes
                  : 'No release notes.',
              url: `https://github.com/${GITHUB_REPO}/releases/tag/v${latestVersion}`,
            });
          }
          writeChangelogStore(store);

          return {
            available,
            currentVersion,
            latestVersion,
            releaseUrl: `https://github.com/${GITHUB_REPO}/releases/tag/v${latestVersion}`,
            releaseNotes:
              typeof result.updateInfo.releaseNotes === 'string'
                ? result.updateInfo.releaseNotes
                : 'No release notes.',
            canAutoUpdate: true,
          };
        }
      } catch (err: any) {
        console.warn('electron-updater check failed, falling back to GitHub API:', err.message);
      }
    }

    // Fallback: GitHub API check
    try {
      const release = await fetchJSON(RELEASES_URL);

      if (!release || !release.tag_name) {
        return {
          available: false,
          currentVersion,
          canAutoUpdate: false,
        };
      }

      const latestVersion = release.tag_name.replace(/^v/, '');
      const available = compareVersions(currentVersion, latestVersion) < 0;

      const store = readChangelogStore();
      store.lastChecked = new Date().toISOString();

      const existingIndex = store.entries.findIndex((e) => e.version === latestVersion);
      if (existingIndex === -1) {
        store.entries.unshift({
          version: latestVersion,
          date: release.published_at || new Date().toISOString(),
          notes: release.body || 'No release notes.',
          url: release.html_url || '',
        });
      }

      writeChangelogStore(store);

      return {
        available,
        currentVersion,
        latestVersion,
        releaseUrl: release.html_url,
        releaseNotes: release.body || 'No release notes.',
        canAutoUpdate: false,
      };
    } catch (err: any) {
      return {
        available: false,
        currentVersion,
        error: err.message,
        canAutoUpdate: false,
      };
    }
  });

  // Download update via electron-updater
  ipcMain.handle('updater:downloadUpdate', async () => {
    const autoUpdater = getAutoUpdater();
    if (!autoUpdater) {
      return { success: false, error: 'Auto-updater is not available in this environment' };
    }

    try {
      await autoUpdater.downloadUpdate();
      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // Install update (quit and install)
  ipcMain.handle('updater:installUpdate', async () => {
    const autoUpdater = getAutoUpdater();
    if (!autoUpdater) {
      return { success: false, error: 'Auto-updater is not available in this environment' };
    }

    if (!updateDownloaded) {
      return { success: false, error: 'No update has been downloaded yet' };
    }

    try {
      // quitAndInstall will close the app and install the update
      autoUpdater.quitAndInstall(false, true);
      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // Get current app version
  ipcMain.handle('updater:getVersion', async () => {
    return app.getVersion();
  });

  // Get stored changelog (with GitHub API fetch)
  ipcMain.handle('updater:getChangelog', async () => {
    try {
      const releases = await fetchJSON(`${ALL_RELEASES_URL}?per_page=10`);

      if (Array.isArray(releases) && releases.length > 0) {
        const store = readChangelogStore();

        for (const release of releases) {
          const version = (release.tag_name || '').replace(/^v/, '');
          if (!version) continue;

          const existingIndex = store.entries.findIndex((e) => e.version === version);
          if (existingIndex === -1) {
            store.entries.push({
              version,
              date: release.published_at || '',
              notes: release.body || 'No release notes.',
              url: release.html_url || '',
            });
          }
        }

        store.entries.sort((a, b) => compareVersions(b.version, a.version));
        writeChangelogStore(store);

        return store.entries;
      }
    } catch {
      // Fall through to local store
    }

    const store = readChangelogStore();
    return store.entries;
  });

  // Get auto-update enabled setting
  ipcMain.handle('updater:getAutoUpdateEnabled', async () => {
    const settings = readSettings();
    return settings.autoUpdateEnabled;
  });

  // Set auto-update enabled setting
  ipcMain.handle('updater:setAutoUpdateEnabled', async (_event, enabled: boolean) => {
    writeSettings({ autoUpdateEnabled: enabled });
    return { success: true };
  });

  // Auto-check for updates on startup (deferred)
  setTimeout(() => {
    const settings = readSettings();
    if (settings.autoUpdateEnabled) {
      const autoUpdater = getAutoUpdater();
      if (autoUpdater) {
        autoUpdater.checkForUpdates().catch((err: Error) => {
          console.warn('Auto-update check on startup failed:', err.message);
        });
      }
    }
  }, 10000); // Check 10 seconds after startup
}
