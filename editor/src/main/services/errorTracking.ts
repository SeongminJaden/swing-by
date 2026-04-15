import { ipcMain } from 'electron';
import { randomUUID } from 'crypto';

// Types
type Severity = 'critical' | 'error' | 'warning' | 'info';

interface ErrorReport {
  message: string;
  stack?: string;
  source?: string;
  severity: Severity;
  projectId: string;
}

interface Occurrence {
  timestamp: number;
}

interface TrackedError {
  id: string;
  message: string;
  stack?: string;
  source?: string;
  severity: Severity;
  projectId: string;
  occurrences: Occurrence[];
  firstSeen: number;
  lastSeen: number;
  resolved: boolean;
}

// Stores: errorId -> TrackedError
const errors = new Map<string, TrackedError>();
// Index: "projectId::message" -> errorId (for deduplication)
const errorIndex = new Map<string, string>();

function deduplicationKey(projectId: string, message: string): string {
  return `${projectId}::${message}`;
}

export function registerErrorTrackingHandlers() {
  ipcMain.handle(
    'errors:report',
    async (_event, report: ErrorReport) => {
      const key = deduplicationKey(report.projectId, report.message);
      const now = Date.now();

      const existingId = errorIndex.get(key);
      if (existingId) {
        const existing = errors.get(existingId);
        if (existing) {
          existing.occurrences.push({ timestamp: now });
          existing.lastSeen = now;
          // If it was resolved, reopen on new occurrence
          if (existing.resolved) {
            existing.resolved = false;
          }
          // Update stack/source if provided and previously missing
          if (report.stack && !existing.stack) existing.stack = report.stack;
          if (report.source && !existing.source) existing.source = report.source;
          // Escalate severity if higher
          if (severityRank(report.severity) > severityRank(existing.severity)) {
            existing.severity = report.severity;
          }
          return { id: existingId };
        }
      }

      // New error
      const id = randomUUID();
      const tracked: TrackedError = {
        id,
        message: report.message,
        stack: report.stack,
        source: report.source,
        severity: report.severity,
        projectId: report.projectId,
        occurrences: [{ timestamp: now }],
        firstSeen: now,
        lastSeen: now,
        resolved: false,
      };

      errors.set(id, tracked);
      errorIndex.set(key, id);

      return { id };
    }
  );

  ipcMain.handle('errors:list', async (_event, projectId: string) => {
    const results: Array<{
      id: string;
      message: string;
      count: number;
      severity: Severity;
      firstSeen: number;
      lastSeen: number;
      resolved: boolean;
    }> = [];

    for (const err of errors.values()) {
      if (err.projectId === projectId) {
        results.push({
          id: err.id,
          message: err.message,
          count: err.occurrences.length,
          severity: err.severity,
          firstSeen: err.firstSeen,
          lastSeen: err.lastSeen,
          resolved: err.resolved,
        });
      }
    }

    // Sort by lastSeen descending
    results.sort((a, b) => b.lastSeen - a.lastSeen);
    return results;
  });

  ipcMain.handle('errors:resolve', async (_event, errorId: string) => {
    const err = errors.get(errorId);
    if (err) {
      err.resolved = true;
    }
  });

  ipcMain.handle('errors:getDetail', async (_event, errorId: string) => {
    const err = errors.get(errorId);
    if (!err) return null;

    return {
      id: err.id,
      message: err.message,
      stack: err.stack,
      source: err.source,
      severity: err.severity,
      projectId: err.projectId,
      occurrences: err.occurrences.map((o) => ({ ...o })),
      firstSeen: err.firstSeen,
      lastSeen: err.lastSeen,
      resolved: err.resolved,
    };
  });
}

function severityRank(severity: Severity): number {
  switch (severity) {
    case 'info': return 0;
    case 'warning': return 1;
    case 'error': return 2;
    case 'critical': return 3;
  }
}
