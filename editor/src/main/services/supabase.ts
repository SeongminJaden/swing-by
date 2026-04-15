import { createClient, SupabaseClient } from '@supabase/supabase-js';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

const CONFIG_PATH = path.join(os.homedir(), '.videplace', 'supabase.json');

// Default Supabase config (can be overridden by user)
const DEFAULT_URL = 'https://your-project.supabase.co';
const DEFAULT_ANON_KEY = 'your-anon-key';

let supabase: SupabaseClient | null = null;

export function getSupabase(): SupabaseClient | null {
  if (supabase) return supabase;

  let url = DEFAULT_URL;
  let anonKey = DEFAULT_ANON_KEY;

  // Try to load custom config
  try {
    const raw = fs.readFileSync(CONFIG_PATH, 'utf-8');
    const config = JSON.parse(raw);
    if (config.url) url = config.url;
    if (config.anonKey) anonKey = config.anonKey;
  } catch { /* use defaults */ }

  // Only create client if we have real values (not placeholder)
  if (url.includes('your-project')) return null;

  supabase = createClient(url, anonKey);
  return supabase;
}

export function isSupabaseConfigured(): boolean {
  return getSupabase() !== null;
}

export function configureSupabase(url: string, anonKey: string): boolean {
  try {
    const dir = path.dirname(CONFIG_PATH);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(CONFIG_PATH, JSON.stringify({ url, anonKey }, null, 2));
    supabase = createClient(url, anonKey);
    return true;
  } catch { return false; }
}
