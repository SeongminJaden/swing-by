import { ipcMain, BrowserWindow } from 'electron';
import http from 'http';
import https from 'https';
import { randomUUID } from 'crypto';

// Types
interface DataPoint {
  timestamp: number;
  responseTime: number;
  statusCode: number;
  ok: boolean;
}

interface AlertRule {
  type: 'responseTime' | 'statusCode' | 'down';
  threshold: number;
}

interface AlertEntry {
  triggeredAt: number;
  rule: AlertRule;
  value: number;
}

interface MonitorEntry {
  id: string;
  url: string;
  intervalMs: number;
  timer: ReturnType<typeof setInterval> | null;
  dataPoints: DataPoint[];
  alertRules: AlertRule[];
  alerts: AlertEntry[];
  status: 'running' | 'stopped';
  lastCheck: number | null;
}

const MAX_DATA_POINTS = 100;
const DEFAULT_INTERVAL_MS = 30000;

const monitors = new Map<string, MonitorEntry>();

function emitToAllWindows(channel: string, ...args: unknown[]) {
  for (const win of BrowserWindow.getAllWindows()) {
    if (!win.isDestroyed()) {
      win.webContents.send(channel, ...args);
    }
  }
}

function pingUrl(url: string): Promise<DataPoint> {
  return new Promise((resolve) => {
    const start = Date.now();
    const mod = url.startsWith('https') ? https : http;

    const req = mod.get(url, { timeout: 10000 }, (res) => {
      // Consume the response data to free up memory
      res.resume();
      res.on('end', () => {
        const responseTime = Date.now() - start;
        resolve({
          timestamp: Date.now(),
          responseTime,
          statusCode: res.statusCode ?? 0,
          ok: (res.statusCode ?? 0) >= 200 && (res.statusCode ?? 0) < 400,
        });
      });
    });

    req.on('error', () => {
      resolve({
        timestamp: Date.now(),
        responseTime: Date.now() - start,
        statusCode: 0,
        ok: false,
      });
    });

    req.on('timeout', () => {
      req.destroy();
      resolve({
        timestamp: Date.now(),
        responseTime: Date.now() - start,
        statusCode: 0,
        ok: false,
      });
    });
  });
}

function checkAlerts(monitor: MonitorEntry, dataPoint: DataPoint) {
  for (const rule of monitor.alertRules) {
    let triggered = false;
    let value = 0;

    switch (rule.type) {
      case 'responseTime':
        value = dataPoint.responseTime;
        triggered = dataPoint.responseTime > rule.threshold;
        break;
      case 'statusCode':
        value = dataPoint.statusCode;
        triggered = dataPoint.statusCode !== rule.threshold;
        break;
      case 'down':
        value = dataPoint.ok ? 1 : 0;
        triggered = !dataPoint.ok;
        break;
    }

    if (triggered) {
      const alert: AlertEntry = { triggeredAt: Date.now(), rule, value };
      monitor.alerts.push(alert);
      emitToAllWindows('monitoring:alert', { id: monitor.id, url: monitor.url, alert });
    }
  }
}

async function performCheck(monitor: MonitorEntry) {
  const dataPoint = await pingUrl(monitor.url);

  monitor.dataPoints.push(dataPoint);
  if (monitor.dataPoints.length > MAX_DATA_POINTS) {
    monitor.dataPoints.shift();
  }
  monitor.lastCheck = dataPoint.timestamp;

  checkAlerts(monitor, dataPoint);

  emitToAllWindows('monitoring:update', {
    id: monitor.id,
    url: monitor.url,
    dataPoint,
  });
}

function computeMetrics(monitor: MonitorEntry) {
  const points = monitor.dataPoints;
  if (points.length === 0) {
    return { dataPoints: [], uptime: 100, avgResponseTime: 0 };
  }

  const okCount = points.filter((p) => p.ok).length;
  const uptime = (okCount / points.length) * 100;
  const avgResponseTime =
    points.reduce((sum, p) => sum + p.responseTime, 0) / points.length;

  return {
    dataPoints: points.map((p) => ({ ...p })),
    uptime: Math.round(uptime * 100) / 100,
    avgResponseTime: Math.round(avgResponseTime),
  };
}

export function registerMonitoringHandlers() {
  ipcMain.handle('monitoring:start', async (_event, url: string, intervalMs?: number) => {
    const id = randomUUID();
    const interval = intervalMs ?? DEFAULT_INTERVAL_MS;

    const monitor: MonitorEntry = {
      id,
      url,
      intervalMs: interval,
      timer: null,
      dataPoints: [],
      alertRules: [],
      alerts: [],
      status: 'running',
      lastCheck: null,
    };

    monitors.set(id, monitor);

    // Perform initial check immediately
    await performCheck(monitor);

    // Start periodic checks
    monitor.timer = setInterval(() => {
      performCheck(monitor);
    }, interval);

    return { id };
  });

  ipcMain.handle('monitoring:stop', async (_event, id: string) => {
    const monitor = monitors.get(id);
    if (!monitor) return;

    if (monitor.timer) {
      clearInterval(monitor.timer);
      monitor.timer = null;
    }
    monitor.status = 'stopped';
  });

  ipcMain.handle('monitoring:getMetrics', async (_event, id: string) => {
    const monitor = monitors.get(id);
    if (!monitor) {
      return { dataPoints: [], uptime: 0, avgResponseTime: 0 };
    }
    return computeMetrics(monitor);
  });

  ipcMain.handle(
    'monitoring:addAlert',
    async (_event, id: string, rule: AlertRule) => {
      const monitor = monitors.get(id);
      if (!monitor) return;
      monitor.alertRules.push(rule);
    }
  );

  ipcMain.handle('monitoring:getAlerts', async (_event, id: string) => {
    const monitor = monitors.get(id);
    if (!monitor) return [];
    return monitor.alerts.map((a) => ({ ...a }));
  });

  ipcMain.handle('monitoring:listActive', async () => {
    const active: Array<{ id: string; url: string; status: string; lastCheck: number | null }> = [];
    for (const monitor of monitors.values()) {
      active.push({
        id: monitor.id,
        url: monitor.url,
        status: monitor.status,
        lastCheck: monitor.lastCheck,
      });
    }
    return active;
  });
}
