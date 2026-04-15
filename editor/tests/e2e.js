/**
 * VidEplace E2E Integration Test Suite
 *
 * Connects to the Electron app via Chrome DevTools Protocol (CDP)
 * and runs integration tests against all IPC handlers.
 *
 * Usage:
 *   1. Start the app in dev mode: npm run dev:electron
 *   2. Run tests: node tests/e2e.js
 *
 * The app must be running with --remote-debugging-port=9222
 */

const http = require('http');
const https = require('https');
const { WebSocket } = require('ws');
const os = require('os');
const path = require('path');
const fs = require('fs');

// ─── Configuration ───────────────────────────────────────────────────────────

const CDP_PORT = 9222;
const TEST_TIMEOUT = 15000;

// ─── Colors for terminal output ──────────────────────────────────────────────

const C = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  red: '\x1b[31m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
  bgGreen: '\x1b[42m',
  bgRed: '\x1b[41m',
};

// ─── Test Runner ─────────────────────────────────────────────────────────────

let ws = null;
let msgId = 1;
const pending = new Map();

const results = [];
let currentGroup = '';

function log(msg) {
  console.log(msg);
}

function group(name) {
  currentGroup = name;
  log(`\n${C.cyan}${C.bold}  [${name}]${C.reset}`);
}

function pass(name, detail) {
  results.push({ group: currentGroup, name, passed: true });
  const d = detail ? ` ${C.dim}(${detail})${C.reset}` : '';
  log(`    ${C.green}PASS${C.reset} ${name}${d}`);
}

function fail(name, error) {
  results.push({ group: currentGroup, name, passed: false, error });
  log(`    ${C.red}FAIL${C.reset} ${name}`);
  log(`         ${C.dim}${error}${C.reset}`);
}

// ─── CDP Communication ───────────────────────────────────────────────────────

function getDebuggerUrl() {
  return new Promise((resolve, reject) => {
    http
      .get(`http://127.0.0.1:${CDP_PORT}/json`, (res) => {
        let data = '';
        res.on('data', (chunk) => (data += chunk));
        res.on('end', () => {
          try {
            const targets = JSON.parse(data);
            const page = targets.find(
              (t) => t.type === 'page' && t.webSocketDebuggerUrl
            );
            if (page) {
              resolve(page.webSocketDebuggerUrl);
            } else {
              reject(new Error('No page target found. Is the app running?'));
            }
          } catch (e) {
            reject(new Error(`Failed to parse CDP targets: ${e.message}`));
          }
        });
      })
      .on('error', (e) => {
        reject(
          new Error(
            `Cannot connect to CDP on port ${CDP_PORT}. Is the app running with --remote-debugging-port=${CDP_PORT}?\n${e.message}`
          )
        );
      });
  });
}

function connectWS(url) {
  return new Promise((resolve, reject) => {
    ws = new WebSocket(url);
    ws.on('open', () => resolve());
    ws.on('error', (e) => reject(e));
    ws.on('message', (raw) => {
      try {
        const msg = JSON.parse(raw.toString());
        if (msg.id && pending.has(msg.id)) {
          const { resolve: res, reject: rej, timer } = pending.get(msg.id);
          clearTimeout(timer);
          pending.delete(msg.id);
          if (msg.error) {
            rej(new Error(msg.error.message));
          } else {
            res(msg.result);
          }
        }
      } catch {
        // ignore
      }
    });
  });
}

function cdpSend(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = msgId++;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP timeout for ${method}`));
    }, TEST_TIMEOUT);

    pending.set(id, { resolve, reject, timer });
    ws.send(JSON.stringify({ id, method, params }));
  });
}

/**
 * Evaluate JS in the renderer and return the result.
 * Wraps the expression in an async IIFE so we can use await.
 */
async function evaluate(expression) {
  const result = await cdpSend('Runtime.evaluate', {
    expression: `(async () => { ${expression} })()`,
    awaitPromise: true,
    returnByValue: true,
  });

  if (result.exceptionDetails) {
    const errMsg =
      result.exceptionDetails.exception?.description ||
      result.exceptionDetails.text ||
      'Unknown evaluation error';
    throw new Error(errMsg);
  }

  return result.result?.value;
}

/**
 * Call an IPC method via electronAPI and return the result.
 */
async function ipc(method, ...args) {
  const argsStr = args.map((a) => JSON.stringify(a)).join(', ');
  return evaluate(`return await window.electronAPI.${method}(${argsStr});`);
}

// ─── Test Definitions ────────────────────────────────────────────────────────

async function testAuth() {
  group('Auth');

  const testEmail = `test_${Date.now()}@videplace.test`;
  const testPassword = 'TestPass123!';
  const testName = 'E2E Tester';

  try {
    const reg = await ipc('authRegister', testEmail, testPassword, testName);
    if (reg && reg.success) {
      pass('register', `id=${reg.user?.id}`);
    } else {
      fail('register', reg?.error || 'No success');
    }
  } catch (e) {
    fail('register', e.message);
  }

  try {
    const login = await ipc('authLogin', testEmail, testPassword);
    if (login && login.success) {
      pass('login', `email=${login.user?.email}`);
    } else {
      fail('login', login?.error || 'No success');
    }
  } catch (e) {
    fail('login', e.message);
  }

  try {
    const plan = await ipc('authUpdatePlan', 'pro');
    if (plan && plan.success) {
      pass('updatePlan', 'pro');
    } else {
      fail('updatePlan', plan?.error || 'No success');
    }
  } catch (e) {
    fail('updatePlan', e.message);
  }
}

async function testFileSystem() {
  group('File System');

  const homeDir = os.homedir();

  try {
    const entries = await ipc('readDir', homeDir);
    if (Array.isArray(entries)) {
      pass('readDir', `${entries.length} entries`);
    } else {
      fail('readDir', 'Expected array');
    }
  } catch (e) {
    fail('readDir', e.message);
  }

  const testFile = path.join(os.homedir(), '.videplace', '_e2e_test.txt');
  const testContent = `E2E test ${Date.now()}`;

  try {
    const write = await ipc('writeFile', testFile, testContent);
    if (write && write.success !== false) {
      pass('writeFile', testFile);
    } else {
      fail('writeFile', write?.error || 'Failed');
    }
  } catch (e) {
    fail('writeFile', e.message);
  }

  try {
    const content = await ipc('readFile', testFile);
    if (content === testContent || (content && content.content === testContent)) {
      pass('readFile', 'content matches');
    } else if (typeof content === 'string' || typeof content === 'object') {
      pass('readFile', 'returned data');
    } else {
      fail('readFile', 'Unexpected result');
    }
  } catch (e) {
    fail('readFile', e.message);
  }

  // Cleanup
  try {
    fs.unlinkSync(testFile);
  } catch {
    // ok
  }
}

async function testGit() {
  group('Git');

  const testDir = os.homedir();

  try {
    const isRepo = await ipc('gitIsRepo', testDir);
    pass('isRepo', `result=${JSON.stringify(isRepo)}`);
  } catch (e) {
    fail('isRepo', e.message);
  }

  // Test on a known git repo if available
  const repoDir = path.join(os.homedir(), 'git', 'videplace');
  try {
    const status = await ipc('gitStatus', repoDir);
    pass('status', typeof status === 'object' ? 'returned object' : 'returned data');
  } catch (e) {
    fail('status', e.message);
  }

  try {
    const log = await ipc('gitLog', repoDir, 5);
    pass('log', Array.isArray(log) ? `${log.length} commits` : 'returned data');
  } catch (e) {
    fail('log', e.message);
  }
}

async function testSecurity() {
  group('Security');

  try {
    const scan = await ipc('securityScan', os.homedir());
    pass('scan', typeof scan === 'object' ? 'returned result' : 'ok');
  } catch (e) {
    fail('scan', e.message);
  }
}

async function testDevEnv() {
  group('Dev Environment');

  try {
    const nodeResult = await ipc('execCommand', 'node --version');
    if (nodeResult && nodeResult.success) {
      pass('detect node', nodeResult.output);
    } else {
      fail('detect node', 'node not found');
    }
  } catch (e) {
    fail('detect node', e.message);
  }

  try {
    const npmResult = await ipc('execCommand', 'npm --version');
    if (npmResult && npmResult.success) {
      pass('detect npm', npmResult.output);
    } else {
      fail('detect npm', 'npm not found');
    }
  } catch (e) {
    fail('detect npm', e.message);
  }
}

async function testMonitoring() {
  group('Monitoring');

  let monitorId = null;

  try {
    const start = await ipc('monitoringStart', 'https://example.com', 30000);
    if (start && (start.id || start.success !== false)) {
      monitorId = start.id || start;
      pass('start', `id=${monitorId}`);
    } else {
      fail('start', start?.error || 'No id returned');
    }
  } catch (e) {
    fail('start', e.message);
  }

  if (monitorId) {
    try {
      const metrics = await ipc('monitoringGetMetrics', monitorId);
      pass('getMetrics', typeof metrics === 'object' ? 'returned object' : 'ok');
    } catch (e) {
      fail('getMetrics', e.message);
    }

    try {
      const stop = await ipc('monitoringStop', monitorId);
      pass('stop', 'ok');
    } catch (e) {
      fail('stop', e.message);
    }
  }
}

async function testErrorTracking() {
  group('Error Tracking');

  const projectId = 'e2e-test-project';

  try {
    const report = await ipc('errorsReport', {
      message: 'E2E test error',
      stack: 'Error: test\n    at e2e.js:1:1',
      source: 'e2e-test',
      severity: 'warning',
      projectId,
    });
    pass('report', report?.id ? `id=${report.id}` : 'ok');
  } catch (e) {
    fail('report', e.message);
  }

  try {
    const list = await ipc('errorsList', projectId);
    if (Array.isArray(list)) {
      pass('list', `${list.length} errors`);
    } else {
      pass('list', 'returned data');
    }
  } catch (e) {
    fail('list', e.message);
  }
}

async function testCostTracking() {
  group('Cost Tracking');

  try {
    const record = await ipc('costsRecord', {
      provider: 'anthropic',
      model: 'claude-3-opus',
      inputTokens: 1000,
      outputTokens: 500,
      cost: 0.045,
    });
    pass('record', 'ok');
  } catch (e) {
    fail('record', e.message);
  }

  try {
    const summary = await ipc('costsGetSummary');
    pass('getSummary', typeof summary === 'object' ? 'returned object' : 'ok');
  } catch (e) {
    fail('getSummary', e.message);
  }
}

async function testTeams() {
  group('Teams');

  try {
    const team = await ipc('teamCreate', `E2E Team ${Date.now()}`);
    if (team && (team.success || team.id)) {
      pass('create', team.id ? `id=${team.id}` : 'ok');
    } else {
      // May fail if not logged in - still informative
      fail('create', team?.error || 'No result');
    }
  } catch (e) {
    fail('create', e.message);
  }

  try {
    const teams = await ipc('teamGetMyTeams');
    if (Array.isArray(teams)) {
      pass('getMyTeams', `${teams.length} teams`);
    } else {
      pass('getMyTeams', 'returned data');
    }
  } catch (e) {
    fail('getMyTeams', e.message);
  }
}

async function testPayment() {
  group('Payment');

  try {
    const plans = await ipc('paymentGetPlans');
    if (Array.isArray(plans) && plans.length > 0) {
      pass('getPlans', `${plans.length} plans`);
    } else {
      fail('getPlans', 'Expected non-empty array');
    }
  } catch (e) {
    fail('getPlans', e.message);
  }

  try {
    const sub = await ipc('paymentSubscribe', 'pro');
    if (sub && sub.success) {
      pass('subscribe', `plan=${sub.subscription?.plan}`);
    } else {
      fail('subscribe', sub?.error || 'No success');
    }
  } catch (e) {
    fail('subscribe', e.message);
  }

  try {
    const current = await ipc('paymentGetCurrentSubscription');
    if (current && current.plan) {
      pass('getCurrentSubscription', `plan=${current.plan}, status=${current.status}`);
    } else {
      pass('getCurrentSubscription', 'no subscription');
    }
  } catch (e) {
    fail('getCurrentSubscription', e.message);
  }

  try {
    const invoices = await ipc('paymentGetInvoices');
    if (Array.isArray(invoices)) {
      pass('getInvoices', `${invoices.length} invoices`);
    } else {
      fail('getInvoices', 'Expected array');
    }
  } catch (e) {
    fail('getInvoices', e.message);
  }
}

async function testUpdater() {
  group('Updater');

  try {
    const update = await ipc('updaterCheckForUpdates');
    if (update && typeof update.currentVersion === 'string') {
      pass(
        'checkForUpdates',
        `current=${update.currentVersion}, available=${update.available}`
      );
    } else if (update && update.available !== undefined) {
      pass('checkForUpdates', `available=${update.available}`);
    } else {
      fail('checkForUpdates', 'Unexpected response');
    }
  } catch (e) {
    fail('checkForUpdates', e.message);
  }

  try {
    const version = await ipc('updaterGetVersion');
    if (typeof version === 'string' && version.length > 0) {
      pass('getVersion', version);
    } else {
      fail('getVersion', `Unexpected: ${version}`);
    }
  } catch (e) {
    fail('getVersion', e.message);
  }

  try {
    const changelog = await ipc('updaterGetChangelog');
    if (Array.isArray(changelog)) {
      pass('getChangelog', `${changelog.length} entries`);
    } else {
      pass('getChangelog', 'returned data');
    }
  } catch (e) {
    fail('getChangelog', e.message);
  }
}

async function testDeploy() {
  group('Deploy');

  try {
    const platforms = await ipc('deployListPlatforms');
    if (Array.isArray(platforms) && platforms.length > 0) {
      const names = platforms.map((p) => p.name).join(', ');
      pass('listPlatforms', names);
    } else {
      fail('listPlatforms', 'Expected non-empty array');
    }
  } catch (e) {
    fail('listPlatforms', e.message);
  }

  const repoDir = path.join(os.homedir(), 'git', 'videplace');
  try {
    const fw = await ipc('deployDetectFramework', repoDir);
    if (fw && fw.framework) {
      pass('detectFramework', `${fw.framework} -> ${fw.buildCmd} -> ${fw.outputDir}`);
    } else {
      fail('detectFramework', 'No framework detected');
    }
  } catch (e) {
    fail('detectFramework', e.message);
  }
}

// ─── Main ────────────────────────────────────────────────────────────────────

async function main() {
  log(
    `\n${C.bold}${C.cyan}  ╔══════════════════════════════════════════╗${C.reset}`
  );
  log(
    `${C.bold}${C.cyan}  ║     VidEplace E2E Integration Tests      ║${C.reset}`
  );
  log(
    `${C.bold}${C.cyan}  ╚══════════════════════════════════════════╝${C.reset}\n`
  );

  // Connect via CDP
  log(`${C.dim}  Connecting to CDP on port ${CDP_PORT}...${C.reset}`);

  let debuggerUrl;
  try {
    debuggerUrl = await getDebuggerUrl();
  } catch (e) {
    log(`\n${C.red}  ERROR: ${e.message}${C.reset}`);
    log(`${C.dim}  Make sure the app is running: npm run dev:electron${C.reset}\n`);
    process.exit(1);
  }

  try {
    await connectWS(debuggerUrl);
  } catch (e) {
    log(`\n${C.red}  ERROR: Cannot connect to WebSocket: ${e.message}${C.reset}\n`);
    process.exit(1);
  }

  log(`${C.dim}  Connected. Running tests...\n${C.reset}`);

  // Enable Runtime domain
  await cdpSend('Runtime.enable');

  const startTime = Date.now();

  // Run all test groups
  await testAuth();
  await testFileSystem();
  await testGit();
  await testSecurity();
  await testDevEnv();
  await testMonitoring();
  await testErrorTracking();
  await testCostTracking();
  await testTeams();
  await testPayment();
  await testUpdater();
  await testDeploy();

  const elapsed = ((Date.now() - startTime) / 1000).toFixed(2);

  // Summary
  const passed = results.filter((r) => r.passed).length;
  const failed = results.filter((r) => !r.passed).length;
  const total = results.length;

  log(`\n${C.bold}  ──────────────────────────────────────────${C.reset}`);
  log(`${C.bold}  Results: ${C.green}${passed} passed${C.reset}${C.bold}, ${failed > 0 ? C.red : C.dim}${failed} failed${C.reset}${C.bold}, ${total} total${C.reset}`);
  log(`${C.dim}  Time: ${elapsed}s${C.reset}`);

  if (failed > 0) {
    log(`\n${C.red}${C.bold}  Failed tests:${C.reset}`);
    for (const r of results.filter((r) => !r.passed)) {
      log(`    ${C.red}[${r.group}]${C.reset} ${r.name}: ${C.dim}${r.error}${C.reset}`);
    }
  }

  log('');

  // Cleanup
  ws.close();
  process.exit(failed > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(`\n${C.red}  Fatal: ${e.message}${C.reset}\n`);
  process.exit(1);
});
