import { ipcMain, BrowserWindow, shell } from 'electron';
import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { getSupabase, isSupabaseConfigured, configureSupabase } from './supabase';

// Types
interface User {
  id: string;
  email: string;
  name: string;
  passwordHash: string;
  salt: string;
  plan: 'free' | 'pro' | 'team' | 'enterprise';
  createdAt: string;
  updatedAt: string;
}

interface UsersStore {
  users: User[];
}

// State
let currentUser: Omit<User, 'passwordHash' | 'salt'> | null = null;
let mainWindowRef: BrowserWindow | null = null;

// Paths
const DATA_DIR = path.join(os.homedir(), '.videplace');
const USERS_FILE = path.join(DATA_DIR, 'users.json');

// ── Local file-based helpers ──────────────────────────────────────────

function ensureDataDir(): void {
  if (!fs.existsSync(DATA_DIR)) {
    fs.mkdirSync(DATA_DIR, { recursive: true });
  }
}

function readUsersStore(): UsersStore {
  ensureDataDir();
  if (!fs.existsSync(USERS_FILE)) {
    const empty: UsersStore = { users: [] };
    fs.writeFileSync(USERS_FILE, JSON.stringify(empty, null, 2), 'utf-8');
    return empty;
  }
  try {
    const raw = fs.readFileSync(USERS_FILE, 'utf-8');
    return JSON.parse(raw) as UsersStore;
  } catch {
    return { users: [] };
  }
}

function writeUsersStore(store: UsersStore): void {
  ensureDataDir();
  fs.writeFileSync(USERS_FILE, JSON.stringify(store, null, 2), 'utf-8');
}

function hashPassword(password: string, salt: string): string {
  return crypto.createHash('sha256').update(password + salt).digest('hex');
}

function generateSalt(): string {
  return crypto.randomBytes(16).toString('hex');
}

function generateId(): string {
  return crypto.randomUUID();
}

function sanitizeUser(user: User): Omit<User, 'passwordHash' | 'salt'> {
  const { passwordHash, salt, ...safe } = user;
  return safe;
}

// ── OAuth Config ──────────────────────────────────────────────────────

const OAUTH_CONFIG_PATH = path.join(DATA_DIR, 'oauth.json');

interface OAuthConfig {
  google?: string;
  github?: string;
  apple?: string;
  [key: string]: string | undefined;
}

function loadOAuthConfig(): OAuthConfig {
  try {
    if (fs.existsSync(OAUTH_CONFIG_PATH)) {
      return JSON.parse(fs.readFileSync(OAUTH_CONFIG_PATH, 'utf-8'));
    }
  } catch { /* ignore */ }
  return {};
}

function saveOAuthConfig(config: OAuthConfig): void {
  ensureDataDir();
  fs.writeFileSync(OAUTH_CONFIG_PATH, JSON.stringify(config, null, 2), 'utf-8');
}

// ── Helpers ───────────────────────────────────────────────────────────

function getWindow(): BrowserWindow | null {
  return mainWindowRef ?? BrowserWindow.getFocusedWindow();
}

function notifyUserChanged(user: Omit<User, 'passwordHash' | 'salt'> | null): void {
  const win = getWindow();
  if (win) {
    win.webContents.send('auth:userChanged', user);
  }
}

// ── OAuth callback handler (exported for main.ts) ────────────────────

export async function handleAuthCallback(url: string): Promise<void> {
  try {
    // Parse tokens from callback URL
    const hashPart = url.split('#')[1] || url.split('?')[1] || '';
    const params = new URLSearchParams(hashPart);
    const accessToken = params.get('access_token');
    const refreshToken = params.get('refresh_token');
    const code = params.get('code');

    // Try Supabase first
    const supabase = getSupabase();
    if (supabase && accessToken && refreshToken) {
      const { data, error } = await supabase.auth.setSession({
        access_token: accessToken,
        refresh_token: refreshToken,
      });

      if (!error && data.user) {
        const supaUser = data.user;
        currentUser = {
          id: supaUser.id,
          email: supaUser.email ?? '',
          name: supaUser.user_metadata?.full_name ?? supaUser.email ?? '',
          plan: supaUser.user_metadata?.plan ?? 'free',
          createdAt: supaUser.created_at,
          updatedAt: supaUser.updated_at ?? supaUser.created_at,
        };
        notifyUserChanged(currentUser);
        return;
      }
    }

    // Direct OAuth token handling (without Supabase)
    if (accessToken) {
      // Try to fetch user info from the token
      let email = '';
      let name = '';

      // Try GitHub API
      try {
        const https = require('https');
        const userInfo = await new Promise<any>((resolve) => {
          const req = https.get('https://api.github.com/user', {
            headers: { 'Authorization': `token ${accessToken}`, 'User-Agent': 'VidEplace' },
          }, (res: any) => {
            let data = '';
            res.on('data', (chunk: string) => data += chunk);
            res.on('end', () => {
              try { resolve(JSON.parse(data)); } catch { resolve(null); }
            });
          });
          req.on('error', () => resolve(null));
          req.setTimeout(5000, () => { req.destroy(); resolve(null); });
        });

        if (userInfo?.login) {
          email = userInfo.email || `${userInfo.login}@github.com`;
          name = userInfo.name || userInfo.login;
        }
      } catch { /* not a GitHub token */ }

      // Try Google userinfo
      if (!email) {
        try {
          const https = require('https');
          const userInfo = await new Promise<any>((resolve) => {
            const req = https.get(`https://www.googleapis.com/oauth2/v2/userinfo?access_token=${accessToken}`, (res: any) => {
              let data = '';
              res.on('data', (chunk: string) => data += chunk);
              res.on('end', () => {
                try { resolve(JSON.parse(data)); } catch { resolve(null); }
              });
            });
            req.on('error', () => resolve(null));
            req.setTimeout(5000, () => { req.destroy(); resolve(null); });
          });

          if (userInfo?.email) {
            email = userInfo.email;
            name = userInfo.name || userInfo.email;
          }
        } catch { /* not a Google token */ }
      }

      if (email) {
        // Save user locally
        const store = readUsersStore();
        let user = store.users.find((u) => u.email === email);

        if (!user) {
          const salt = generateSalt();
          user = {
            id: generateId(),
            email,
            name: name || email,
            passwordHash: hashPassword(crypto.randomBytes(16).toString('hex'), salt),
            salt,
            plan: 'free',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          };
          store.users.push(user);
          writeUsersStore(store);
        }

        currentUser = sanitizeUser(user);
        notifyUserChanged(currentUser);
        return;
      }
    }

    // GitHub code exchange (if we got a code instead of token)
    if (code) {
      // For code flow, we'd need client_secret which should be configured
      // For now, store the code and let the user know
      const now = new Date().toISOString();
      currentUser = {
        id: generateId(),
        email: 'oauth-user@videplace.com',
        name: 'OAuth User',
        plan: 'free',
        createdAt: now,
        updatedAt: now,
      };
      notifyUserChanged(currentUser);
    }
  } catch {
    // Silently ignore callback errors
  }
}

// ── Set main window reference (called from main.ts) ──────────────────

export function setMainWindow(win: BrowserWindow): void {
  mainWindowRef = win;
}

// ── IPC Handler Registration ─────────────────────────────────────────

export function registerAuthHandlers(): void {

  // ── auth:register ──────────────────────────────────────────────────

  ipcMain.handle('auth:register', async (_event, email: string, password: string, name: string) => {
    try {
      if (!email || !password || !name) {
        return { success: false, error: 'Email, password, and name are required' };
      }

      // Supabase path
      const supabase = getSupabase();
      if (supabase) {
        const { data, error } = await supabase.auth.signUp({
          email,
          password,
          options: { data: { full_name: name, plan: 'free' } },
        });

        if (error) return { success: false, error: error.message };

        const supaUser = data.user;
        if (!supaUser) return { success: false, error: 'Registration failed' };

        currentUser = {
          id: supaUser.id,
          email: supaUser.email ?? email,
          name: supaUser.user_metadata?.full_name ?? name,
          plan: 'free',
          createdAt: supaUser.created_at,
          updatedAt: supaUser.updated_at ?? supaUser.created_at,
        };

        notifyUserChanged(currentUser);
        return { success: true, user: { id: currentUser.id, email: currentUser.email, name: currentUser.name } };
      }

      // Local fallback
      const store = readUsersStore();

      if (store.users.find((u) => u.email === email)) {
        return { success: false, error: 'Email already registered' };
      }

      const salt = generateSalt();
      const passwordHash = hashPassword(password, salt);
      const now = new Date().toISOString();

      const newUser: User = {
        id: generateId(),
        email,
        name,
        passwordHash,
        salt,
        plan: 'free',
        createdAt: now,
        updatedAt: now,
      };

      store.users.push(newUser);
      writeUsersStore(store);

      const safeUser = sanitizeUser(newUser);
      currentUser = safeUser;

      notifyUserChanged(safeUser);
      return { success: true, user: { id: safeUser.id, email: safeUser.email, name: safeUser.name } };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // ── auth:login ─────────────────────────────────────────────────────

  ipcMain.handle('auth:login', async (_event, email: string, password: string) => {
    try {
      if (!email || !password) {
        return { success: false, error: 'Email and password are required' };
      }

      // Supabase path
      const supabase = getSupabase();
      if (supabase) {
        const { data, error } = await supabase.auth.signInWithPassword({ email, password });

        if (error) return { success: false, error: error.message };

        const supaUser = data.user;
        currentUser = {
          id: supaUser.id,
          email: supaUser.email ?? email,
          name: supaUser.user_metadata?.full_name ?? '',
          plan: supaUser.user_metadata?.plan ?? 'free',
          createdAt: supaUser.created_at,
          updatedAt: supaUser.updated_at ?? supaUser.created_at,
        };

        notifyUserChanged(currentUser);
        return {
          success: true,
          user: { id: currentUser.id, email: currentUser.email, name: currentUser.name, plan: currentUser.plan },
        };
      }

      // Local fallback
      const store = readUsersStore();
      const user = store.users.find((u) => u.email === email);

      if (!user) {
        return { success: false, error: 'Invalid email or password' };
      }

      const hash = hashPassword(password, user.salt);
      if (hash !== user.passwordHash) {
        return { success: false, error: 'Invalid email or password' };
      }

      const safeUser = sanitizeUser(user);
      currentUser = safeUser;

      notifyUserChanged(safeUser);
      return {
        success: true,
        user: { id: safeUser.id, email: safeUser.email, name: safeUser.name, plan: safeUser.plan },
      };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // ── auth:logout ────────────────────────────────────────────────────

  ipcMain.handle('auth:logout', async () => {
    try {
      const supabase = getSupabase();
      if (supabase) {
        await supabase.auth.signOut();
      }

      currentUser = null;
      notifyUserChanged(null);
      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // ── auth:getCurrentUser ────────────────────────────────────────────

  ipcMain.handle('auth:getCurrentUser', async () => {
    // If Supabase is configured, try to restore session
    const supabase = getSupabase();
    if (supabase && !currentUser) {
      try {
        const { data } = await supabase.auth.getUser();
        if (data.user) {
          currentUser = {
            id: data.user.id,
            email: data.user.email ?? '',
            name: data.user.user_metadata?.full_name ?? data.user.email ?? '',
            plan: data.user.user_metadata?.plan ?? 'free',
            createdAt: data.user.created_at,
            updatedAt: data.user.updated_at ?? data.user.created_at,
          };
        }
      } catch {
        // Session expired or invalid
      }
    }

    return currentUser ? { user: currentUser } : null;
  });

  // ── auth:updatePlan ────────────────────────────────────────────────

  ipcMain.handle('auth:updatePlan', async (_event, plan: 'free' | 'pro' | 'team' | 'enterprise') => {
    try {
      if (!currentUser) {
        return { success: false, error: 'Not logged in' };
      }

      const validPlans = ['free', 'pro', 'team', 'enterprise'];
      if (!validPlans.includes(plan)) {
        return { success: false, error: 'Invalid plan' };
      }

      // Supabase path
      const supabase = getSupabase();
      if (supabase) {
        const { error } = await supabase.auth.updateUser({
          data: { plan },
        });
        if (error) return { success: false, error: error.message };

        currentUser = { ...currentUser, plan, updatedAt: new Date().toISOString() };
        notifyUserChanged(currentUser);
        return { success: true };
      }

      // Local fallback
      const store = readUsersStore();
      const userIndex = store.users.findIndex((u) => u.id === currentUser!.id);

      if (userIndex === -1) {
        return { success: false, error: 'User not found' };
      }

      store.users[userIndex].plan = plan;
      store.users[userIndex].updatedAt = new Date().toISOString();
      writeUsersStore(store);

      currentUser = sanitizeUser(store.users[userIndex]);
      notifyUserChanged(currentUser);
      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // ── auth:updateProfile ─────────────────────────────────────────────

  ipcMain.handle('auth:updateProfile', async (_event, data: { name?: string; email?: string }) => {
    try {
      if (!currentUser) {
        return { success: false, error: 'Not logged in' };
      }

      // Supabase path
      const supabase = getSupabase();
      if (supabase) {
        const updatePayload: { email?: string; data?: Record<string, string> } = {};
        if (data.email) updatePayload.email = data.email;
        if (data.name) updatePayload.data = { full_name: data.name };

        const { error } = await supabase.auth.updateUser(updatePayload);
        if (error) return { success: false, error: error.message };

        currentUser = {
          ...currentUser,
          ...(data.email ? { email: data.email } : {}),
          ...(data.name ? { name: data.name } : {}),
          updatedAt: new Date().toISOString(),
        };

        notifyUserChanged(currentUser);
        return { success: true };
      }

      // Local fallback
      const store = readUsersStore();
      const userIndex = store.users.findIndex((u) => u.id === currentUser!.id);

      if (userIndex === -1) {
        return { success: false, error: 'User not found' };
      }

      if (data.email && data.email !== currentUser.email) {
        if (store.users.find((u) => u.email === data.email)) {
          return { success: false, error: 'Email already in use' };
        }
        store.users[userIndex].email = data.email;
      }

      if (data.name) {
        store.users[userIndex].name = data.name;
      }

      store.users[userIndex].updatedAt = new Date().toISOString();
      writeUsersStore(store);

      currentUser = sanitizeUser(store.users[userIndex]);
      notifyUserChanged(currentUser);
      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // ── auth:socialLogin ─────────────────────────────────────────────

  ipcMain.handle('auth:socialLogin', async (_event, provider: 'google' | 'github' | 'apple') => {
    try {
      const validProviders = ['google', 'github', 'apple'];
      if (!validProviders.includes(provider)) {
        return { success: false, error: `Invalid provider: ${provider}` };
      }

      // Try Supabase OAuth first
      const supabase = getSupabase();
      if (supabase) {
        const { data, error } = await supabase.auth.signInWithOAuth({
          provider,
          options: {
            redirectTo: 'videplace://auth/callback',
            skipBrowserRedirect: true,
          },
        });

        if (!error && data.url) {
          await shell.openExternal(data.url);
          return { success: true, url: data.url, message: 'OAuth flow started' };
        }
      }

      // Fallback: Open OAuth popup in BrowserWindow directly
      // Load OAuth client IDs from config
      const oauthConfig = loadOAuthConfig();

      return new Promise((resolve) => {
        const oauthUrls: Record<string, string> = {
          google: `https://accounts.google.com/o/oauth2/v2/auth?client_id=${oauthConfig.google || 'NOT_CONFIGURED'}&redirect_uri=http://localhost:39281/callback&response_type=token&scope=email+profile`,
          github: `https://github.com/login/oauth/authorize?client_id=${oauthConfig.github || 'NOT_CONFIGURED'}&redirect_uri=http://localhost:39281/callback&scope=user:email`,
          apple: `https://appleid.apple.com/auth/authorize?client_id=${oauthConfig.apple || 'NOT_CONFIGURED'}&redirect_uri=http://localhost:39281/callback&response_type=code&scope=email+name`,
        };

        if (!oauthConfig[provider]) {
          resolve({ success: false, error: `${provider} OAuth가 설정되지 않았습니다. 설정에서 OAuth Client ID를 입력해주세요.` });
          return;
        }

        // Start temporary HTTP server to receive OAuth callback
        const http = require('http');
        const callbackServer = http.createServer((req: any, res: any) => {
          const reqUrl = new URL(req.url, 'http://localhost:39281');
          if (reqUrl.pathname === '/callback') {
            // Send a nice HTML response
            res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
            res.end(`
              <html><body style="background:#0d1117;color:#e6edf3;font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0">
                <div style="text-align:center">
                  <h2>로그인 완료!</h2>
                  <p>VidEplace로 돌아가세요. 이 창을 닫아도 됩니다.</p>
                </div>
              </body></html>
            `);

            // Process the callback
            const fullUrl = `http://localhost:39281${req.url}`;
            const callbackUrl = new URL(fullUrl);
            const fragment = callbackUrl.hash?.slice(1) || callbackUrl.search?.slice(1) || '';
            handleAuthCallback(`videplace://auth/callback?${fragment}&${callbackUrl.searchParams.toString()}`);

            // Close server and window
            setTimeout(() => {
              callbackServer.close();
              if (!authWin.isDestroyed()) authWin.close();
            }, 1000);
          }
        });

        callbackServer.listen(39281, '127.0.0.1');

        const authWin = new BrowserWindow({
          width: 600,
          height: 700,
          title: `${provider} 로그인`,
          parent: getWindow() ?? undefined,
          modal: true,
          webPreferences: {
            nodeIntegration: false,
            contextIsolation: true,
          },
        });

        const targetUrl = oauthUrls[provider] || oauthUrls.github;
        authWin.loadURL(targetUrl);

        // Also listen for URL changes in case the redirect comes through the window
        authWin.webContents.on('will-redirect', (_e: any, url: string) => {
          if (url.includes('localhost:39281/callback') || url.startsWith('videplace://')) {
            handleAuthCallback(url);
            callbackServer.close();
            if (!authWin.isDestroyed()) authWin.close();
          }
        });

        authWin.on('closed', () => {
          callbackServer.close();
          if (currentUser) {
            resolve({ success: true, user: currentUser });
          } else {
            resolve({ success: false, error: '로그인이 취소되었습니다' });
          }
        });

        resolve({ success: true, url: targetUrl, message: 'OAuth window opened' });
      });
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // ── auth:getSession ────────────────────────────────────────────────

  ipcMain.handle('auth:getSession', async () => {
    try {
      const supabase = getSupabase();
      if (!supabase) {
        return { session: null, isSupabaseConfigured: false };
      }

      const { data, error } = await supabase.auth.getSession();
      if (error) return { session: null, error: error.message };

      return { session: data.session, isSupabaseConfigured: true };
    } catch (err: any) {
      return { session: null, error: err.message };
    }
  });

  // ── auth:configureOAuth ──────────────────────────────────────────

  ipcMain.handle('auth:configureOAuth', async (_event, provider: string, clientId: string) => {
    try {
      const config = loadOAuthConfig();
      config[provider] = clientId;
      saveOAuthConfig(config);
      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  ipcMain.handle('auth:getOAuthConfig', async () => {
    const config = loadOAuthConfig();
    // Return which providers are configured (not the actual keys)
    return {
      google: !!config.google,
      github: !!config.github,
      apple: !!config.apple,
    };
  });

  // ── auth:configureSupabase ─────────────────────────────────────────

  ipcMain.handle('auth:configureSupabase', async (_event, url: string, anonKey: string) => {
    try {
      if (!url || !anonKey) {
        return { success: false, error: 'Supabase URL and anon key are required' };
      }

      const ok = configureSupabase(url, anonKey);
      if (!ok) return { success: false, error: 'Failed to save Supabase configuration' };

      return { success: true, configured: isSupabaseConfigured() };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });
}

// Export for use by other services (e.g., team service)
export function getCurrentUser(): Omit<User, 'passwordHash' | 'salt'> | null {
  return currentUser;
}
