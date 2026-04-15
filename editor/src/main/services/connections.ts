import { ipcMain } from 'electron';
import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import os from 'os';
import https from 'https';
import { setAIKey } from './ai';

// Types
interface StoredConnection {
  serviceId: string;
  credentials: Record<string, string>; // encrypted
  connectedAt: string;
}

interface ConnectionsStore {
  connections: StoredConnection[];
}

// Paths
const DATA_DIR = path.join(os.homedir(), '.videplace');
const CONNECTIONS_FILE = path.join(DATA_DIR, 'connections.json');

// Encryption helpers (AES-256-GCM with machine-derived key)
function deriveKey(): Buffer {
  const machineId = os.hostname() + os.userInfo().username + os.homedir();
  return crypto.scryptSync(machineId, 'videplace-connections-salt', 32);
}

function encrypt(text: string): string {
  const key = deriveKey();
  const iv = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);
  let encrypted = cipher.update(text, 'utf8', 'hex');
  encrypted += cipher.final('hex');
  const authTag = cipher.getAuthTag().toString('hex');
  return iv.toString('hex') + ':' + authTag + ':' + encrypted;
}

function decrypt(encryptedText: string): string {
  const key = deriveKey();
  const parts = encryptedText.split(':');
  const iv = Buffer.from(parts[0], 'hex');
  const authTag = Buffer.from(parts[1], 'hex');
  const encrypted = parts[2];
  const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv);
  decipher.setAuthTag(authTag);
  let decrypted = decipher.update(encrypted, 'hex', 'utf8');
  decrypted += decipher.final('utf8');
  return decrypted;
}

// Store helpers
function ensureDataDir(): void {
  if (!fs.existsSync(DATA_DIR)) {
    fs.mkdirSync(DATA_DIR, { recursive: true });
  }
}

function readStore(): ConnectionsStore {
  ensureDataDir();
  if (!fs.existsSync(CONNECTIONS_FILE)) {
    const empty: ConnectionsStore = { connections: [] };
    fs.writeFileSync(CONNECTIONS_FILE, JSON.stringify(empty, null, 2), 'utf-8');
    return empty;
  }
  try {
    const raw = fs.readFileSync(CONNECTIONS_FILE, 'utf-8');
    return JSON.parse(raw) as ConnectionsStore;
  } catch {
    return { connections: [] };
  }
}

function writeStore(store: ConnectionsStore): void {
  ensureDataDir();
  fs.writeFileSync(CONNECTIONS_FILE, JSON.stringify(store, null, 2), 'utf-8');
}

// Mask a key for safe display
function maskKey(key: string): string {
  if (key.length <= 8) return '****';
  return key.substring(0, 4) + '...' + key.substring(key.length - 4);
}

// Get the primary key from credentials for masking
function getPrimaryKey(credentials: Record<string, string>): string {
  return credentials['apiKey'] || credentials['token'] || credentials['secretKey'] || credentials['anonKey'] || Object.values(credentials)[0] || '';
}

// HTTPS request helper (no fetch)
function httpsRequest(options: https.RequestOptions, body?: string): Promise<{ statusCode: number; body: string }> {
  return new Promise((resolve, reject) => {
    const req = https.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => { data += chunk; });
      res.on('end', () => {
        resolve({ statusCode: res.statusCode || 0, body: data });
      });
    });
    req.on('error', (err) => reject(err));
    req.setTimeout(10000, () => {
      req.destroy();
      reject(new Error('Request timeout'));
    });
    if (body) req.write(body);
    req.end();
  });
}

// Verification logic
async function verifyCredentials(serviceId: string, credentials: Record<string, string>): Promise<{ success: boolean; error?: string }> {
  try {
    switch (serviceId) {
      case 'claude': {
        const apiKey = credentials['apiKey'];
        if (!apiKey) return { success: false, error: 'API key is required' };
        const body = JSON.stringify({
          model: 'claude-sonnet-4-20250514',
          max_tokens: 1,
          messages: [{ role: 'user', content: 'hi' }],
        });
        const res = await httpsRequest({
          hostname: 'api.anthropic.com',
          path: '/v1/messages',
          method: 'POST',
          headers: {
            'x-api-key': apiKey,
            'anthropic-version': '2023-06-01',
            'content-type': 'application/json',
          },
        }, body);
        return res.statusCode < 500
          ? { success: true }
          : { success: false, error: `API returned status ${res.statusCode}` };
      }

      case 'openai': {
        const apiKey = credentials['apiKey'];
        if (!apiKey) return { success: false, error: 'API key is required' };
        const res = await httpsRequest({
          hostname: 'api.openai.com',
          path: '/v1/models',
          method: 'GET',
          headers: {
            'Authorization': 'Bearer ' + apiKey,
          },
        });
        return res.statusCode === 200
          ? { success: true }
          : { success: false, error: `API returned status ${res.statusCode}` };
      }

      case 'github': {
        const token = credentials['token'];
        if (!token) return { success: false, error: 'Token is required' };
        const res = await httpsRequest({
          hostname: 'api.github.com',
          path: '/user',
          method: 'GET',
          headers: {
            'Authorization': 'token ' + token,
            'User-Agent': 'VidEplace',
          },
        });
        return res.statusCode === 200
          ? { success: true }
          : { success: false, error: `API returned status ${res.statusCode}` };
      }

      case 'vercel': {
        const token = credentials['token'];
        if (!token) return { success: false, error: 'Token is required' };
        const res = await httpsRequest({
          hostname: 'api.vercel.com',
          path: '/v2/user',
          method: 'GET',
          headers: {
            'Authorization': 'Bearer ' + token,
          },
        });
        return res.statusCode === 200
          ? { success: true }
          : { success: false, error: `API returned status ${res.statusCode}` };
      }

      case 'stripe': {
        const secretKey = credentials['secretKey'];
        if (!secretKey) return { success: false, error: 'Secret key is required' };
        const res = await httpsRequest({
          hostname: 'api.stripe.com',
          path: '/v1/balance',
          method: 'GET',
          headers: {
            'Authorization': 'Basic ' + Buffer.from(secretKey + ':').toString('base64'),
          },
        });
        return res.statusCode === 200
          ? { success: true }
          : { success: false, error: `API returned status ${res.statusCode}` };
      }

      case 'supabase': {
        const projectUrl = credentials['projectUrl'];
        const anonKey = credentials['anonKey'];
        if (!projectUrl || !anonKey) return { success: false, error: 'Project URL and anon key are required' };
        // Parse the project URL to extract hostname and path
        const url = new URL(projectUrl + '/rest/v1/');
        const res = await httpsRequest({
          hostname: url.hostname,
          path: url.pathname,
          method: 'GET',
          headers: {
            'apikey': anonKey,
          },
        });
        return res.statusCode === 200
          ? { success: true }
          : { success: false, error: `API returned status ${res.statusCode}` };
      }

      // Services that save without verification
      case 'netlify':
      case 'railway':
      case 'cloudflare':
      case 'firebase':
      case 'slack':
      case 'discord':
      case 'apple-developer':
      case 'google-developer':
        return { success: true };

      default:
        return { success: true };
    }
  } catch (err: any) {
    return { success: false, error: err.message || 'Verification failed' };
  }
}

// Sync AI keys to the in-memory ai.ts store
function syncAIKey(serviceId: string, credentials: Record<string, string>): void {
  if (serviceId === 'claude') {
    const apiKey = credentials['apiKey'];
    if (apiKey) setAIKey('anthropic', apiKey);
  } else if (serviceId === 'openai') {
    const apiKey = credentials['apiKey'];
    if (apiKey) setAIKey('openai', apiKey);
  }
}

export function registerConnectionHandlers(): void {
  // Save connection
  ipcMain.handle('connections:save', async (_event, serviceId: string, credentials: Record<string, string>) => {
    try {
      if (!serviceId || !credentials) {
        return { success: false, error: 'Service ID and credentials are required' };
      }

      const store = readStore();

      // Encrypt credentials
      const encryptedCredentials: Record<string, string> = {};
      for (const [key, value] of Object.entries(credentials)) {
        encryptedCredentials[key] = encrypt(value);
      }

      // Remove existing connection for this service
      store.connections = store.connections.filter((c) => c.serviceId !== serviceId);

      // Add new connection
      store.connections.push({
        serviceId,
        credentials: encryptedCredentials,
        connectedAt: new Date().toISOString(),
      });

      writeStore(store);

      // Sync AI keys if applicable
      syncAIKey(serviceId, credentials);

      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // Verify credentials
  ipcMain.handle('connections:verify', async (_event, serviceId: string, credentials: Record<string, string>) => {
    try {
      if (!serviceId || !credentials) {
        return { success: false, error: 'Service ID and credentials are required' };
      }
      return await verifyCredentials(serviceId, credentials);
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  // Get single connection status (never returns raw keys)
  ipcMain.handle('connections:get', async (_event, serviceId: string) => {
    try {
      const store = readStore();
      const conn = store.connections.find((c) => c.serviceId === serviceId);

      if (!conn) {
        return { connected: false };
      }

      // Decrypt to get primary key for masking
      const decryptedCredentials: Record<string, string> = {};
      for (const [key, value] of Object.entries(conn.credentials)) {
        try {
          decryptedCredentials[key] = decrypt(value);
        } catch {
          decryptedCredentials[key] = '';
        }
      }

      const primaryKey = getPrimaryKey(decryptedCredentials);

      return {
        connected: true,
        maskedKey: maskKey(primaryKey),
        connectedAt: conn.connectedAt,
      };
    } catch (err: any) {
      return { connected: false, error: err.message };
    }
  });

  // Get all connection statuses
  ipcMain.handle('connections:getAll', async () => {
    try {
      const store = readStore();
      const result: Record<string, { connected: boolean; maskedKey?: string; connectedAt?: string }> = {};

      for (const conn of store.connections) {
        const decryptedCredentials: Record<string, string> = {};
        for (const [key, value] of Object.entries(conn.credentials)) {
          try {
            decryptedCredentials[key] = decrypt(value);
          } catch {
            decryptedCredentials[key] = '';
          }
        }
        const primaryKey = getPrimaryKey(decryptedCredentials);
        result[conn.serviceId] = {
          connected: true,
          maskedKey: maskKey(primaryKey),
          connectedAt: conn.connectedAt,
        };
      }

      return result;
    } catch (err: any) {
      return {};
    }
  });

  // Delete connection
  ipcMain.handle('connections:delete', async (_event, serviceId: string) => {
    try {
      const store = readStore();
      store.connections = store.connections.filter((c) => c.serviceId !== serviceId);
      writeStore(store);
      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });
}
