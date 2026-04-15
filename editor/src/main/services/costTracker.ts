import { ipcMain, BrowserWindow } from 'electron';

// Types
interface CostEntry {
  timestamp: number;
  provider: string;
  model: string;
  inputTokens: number;
  outputTokens: number;
  cost: number;
}

interface Budget {
  limit: number;
  warningThreshold: number; // fraction (e.g. 0.8 = 80%)
}

// State
const entries: CostEntry[] = [];
let budget: Budget = { limit: 0, warningThreshold: 0.8 };
let budgetWarningEmitted = false;

function emitToAllWindows(channel: string, ...args: unknown[]) {
  for (const win of BrowserWindow.getAllWindows()) {
    if (!win.isDestroyed()) {
      win.webContents.send(channel, ...args);
    }
  }
}

function totalUsed(): number {
  return entries.reduce((sum, e) => sum + e.cost, 0);
}

function filterByPeriod(period: 'day' | 'week' | 'month'): CostEntry[] {
  const now = Date.now();
  let cutoff: number;

  switch (period) {
    case 'day':
      cutoff = now - 24 * 60 * 60 * 1000;
      break;
    case 'week':
      cutoff = now - 7 * 24 * 60 * 60 * 1000;
      break;
    case 'month':
      cutoff = now - 30 * 24 * 60 * 60 * 1000;
      break;
  }

  return entries.filter((e) => e.timestamp >= cutoff);
}

function buildSummary(filtered: CostEntry[]) {
  const totalCost = filtered.reduce((sum, e) => sum + e.cost, 0);

  const byProvider: Record<string, { cost: number; inputTokens: number; outputTokens: number; count: number }> = {};
  const byModel: Record<string, { cost: number; inputTokens: number; outputTokens: number; count: number }> = {};

  for (const e of filtered) {
    // By provider
    if (!byProvider[e.provider]) {
      byProvider[e.provider] = { cost: 0, inputTokens: 0, outputTokens: 0, count: 0 };
    }
    byProvider[e.provider].cost += e.cost;
    byProvider[e.provider].inputTokens += e.inputTokens;
    byProvider[e.provider].outputTokens += e.outputTokens;
    byProvider[e.provider].count++;

    // By model
    if (!byModel[e.model]) {
      byModel[e.model] = { cost: 0, inputTokens: 0, outputTokens: 0, count: 0 };
    }
    byModel[e.model].cost += e.cost;
    byModel[e.model].inputTokens += e.inputTokens;
    byModel[e.model].outputTokens += e.outputTokens;
    byModel[e.model].count++;
  }

  return {
    totalCost: Math.round(totalCost * 1000000) / 1000000,
    byProvider,
    byModel,
    entries: filtered.map((e) => ({ ...e })),
  };
}

export function registerCostTrackingHandlers() {
  ipcMain.handle(
    'costs:record',
    async (_event, entry: { provider: string; model: string; inputTokens: number; outputTokens: number; cost: number }) => {
      const costEntry: CostEntry = {
        timestamp: Date.now(),
        provider: entry.provider,
        model: entry.model,
        inputTokens: entry.inputTokens,
        outputTokens: entry.outputTokens,
        cost: entry.cost,
      };

      entries.push(costEntry);

      // Check budget warning
      if (budget.limit > 0) {
        const used = totalUsed();
        const fraction = used / budget.limit;

        if (fraction >= budget.warningThreshold && !budgetWarningEmitted) {
          budgetWarningEmitted = true;
          emitToAllWindows('costs:budgetWarning', {
            limit: budget.limit,
            used,
            remaining: budget.limit - used,
            percentage: Math.round(fraction * 100),
          });
        }

        if (used >= budget.limit) {
          emitToAllWindows('costs:budgetExceeded', {
            limit: budget.limit,
            used,
            overage: used - budget.limit,
          });
        }
      }
    }
  );

  ipcMain.handle('costs:getSummary', async (_event, period?: 'day' | 'week' | 'month') => {
    const filtered = period ? filterByPeriod(period) : entries;
    return buildSummary(filtered);
  });

  ipcMain.handle('costs:getBudget', async () => {
    const used = totalUsed();
    return {
      limit: budget.limit,
      used: Math.round(used * 1000000) / 1000000,
      remaining: Math.max(0, Math.round((budget.limit - used) * 1000000) / 1000000),
    };
  });

  ipcMain.handle('costs:setBudget', async (_event, limit: number) => {
    budget.limit = limit;
    // Reset warning flag when budget is changed
    budgetWarningEmitted = false;
  });
}
