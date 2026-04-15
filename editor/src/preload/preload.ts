import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electronAPI', {
  // App info
  getAppInfo: () => ipcRenderer.invoke('get-app-info'),
  onNavigate: (callback: (route: string) => void) =>
    ipcRenderer.on('navigate', (_event, route) => callback(route)),

  // Window controls
  windowMinimize: () => ipcRenderer.send('window-minimize'),
  windowMaximize: () => ipcRenderer.send('window-maximize'),
  windowClose: () => ipcRenderer.send('window-close'),

  // File system
  openFolder: () => ipcRenderer.invoke('fs:openFolder'),
  readDir: (dirPath: string) => ipcRenderer.invoke('fs:readDir', dirPath),
  readFile: (filePath: string) => ipcRenderer.invoke('fs:readFile', filePath),
  writeFile: (filePath: string, content: string) => ipcRenderer.invoke('fs:writeFile', filePath, content),
  mkdir: (dirPath: string) => ipcRenderer.invoke('fs:mkdir', dirPath),
  deleteFile: (targetPath: string) => ipcRenderer.invoke('fs:delete', targetPath),
  fileExists: (targetPath: string) => ipcRenderer.invoke('fs:exists', targetPath),
  fileStat: (targetPath: string) => ipcRenderer.invoke('fs:stat', targetPath),

  // Dev environment
  execCommand: (cmd: string) => ipcRenderer.invoke('exec:command', cmd),

  // Deploy
  deployCheckVercel: () => ipcRenderer.invoke('deploy:checkVercel'),
  deployVercel: (cwd: string, options?: { prod?: boolean; token?: string }) => ipcRenderer.invoke('deploy:vercel', cwd, options),
  deployRailway: (cwd: string) => ipcRenderer.invoke('deploy:railway', cwd),
  deployNetlify: (cwd: string, token?: string) => ipcRenderer.invoke('deploy:netlify', cwd, token),
  deployCloudflare: (cwd: string, token?: string) => ipcRenderer.invoke('deploy:cloudflare', cwd, token),
  deployDetectFramework: (cwd: string) => ipcRenderer.invoke('deploy:detectFramework', cwd),
  deployListPlatforms: () => ipcRenderer.invoke('deploy:listPlatforms'),
  deployStatus: (url: string) => ipcRenderer.invoke('deploy:status', url),
  deploySetVercelToken: (token: string) => ipcRenderer.invoke('deploy:setVercelToken', token),
  deployVercelProjects: (token: string) => ipcRenderer.invoke('deploy:vercelProjects', token),

  // Security
  securityScan: (cwd: string) => ipcRenderer.invoke('security:scan', cwd),
  securityCheckSecrets: (content: string) => ipcRenderer.invoke('security:checkSecrets', content),

  // Git
  gitIsRepo: (cwd: string) => ipcRenderer.invoke('git:isRepo', cwd),
  gitInit: (cwd: string) => ipcRenderer.invoke('git:init', cwd),
  gitStatus: (cwd: string) => ipcRenderer.invoke('git:status', cwd),
  gitAdd: (cwd: string, files: string[]) => ipcRenderer.invoke('git:add', cwd, files),
  gitReset: (cwd: string, files: string[]) => ipcRenderer.invoke('git:reset', cwd, files),
  gitCommit: (cwd: string, message: string) => ipcRenderer.invoke('git:commit', cwd, message),
  gitPush: (cwd: string, remote?: string, branch?: string) => ipcRenderer.invoke('git:push', cwd, remote, branch),
  gitPull: (cwd: string, remote?: string, branch?: string) => ipcRenderer.invoke('git:pull', cwd, remote, branch),
  gitLog: (cwd: string, maxCount?: number) => ipcRenderer.invoke('git:log', cwd, maxCount),
  gitDiff: (cwd: string, file?: string) => ipcRenderer.invoke('git:diff', cwd, file),
  gitBranches: (cwd: string) => ipcRenderer.invoke('git:branches', cwd),
  gitCreateBranch: (cwd: string, name: string) => ipcRenderer.invoke('git:createBranch', cwd, name),
  gitCheckout: (cwd: string, branch: string) => ipcRenderer.invoke('git:checkout', cwd, branch),
  gitRemotes: (cwd: string) => ipcRenderer.invoke('git:remotes', cwd),

  // AI
  aiSetKey: (provider: string, key: string) => ipcRenderer.invoke('ai:setKey', provider, key),
  aiGetKey: (provider: string) => ipcRenderer.invoke('ai:getKey', provider),
  aiHasKey: (provider: string) => ipcRenderer.invoke('ai:hasKey', provider),
  aiChatClaude: (messages: { role: string; content: string }[], model?: string) =>
    ipcRenderer.invoke('ai:chatClaude', messages, model),
  aiChatOpenAI: (messages: { role: string; content: string }[], model?: string) =>
    ipcRenderer.invoke('ai:chatOpenAI', messages, model),
  onAIStream: (callback: (text: string) => void) =>
    ipcRenderer.on('ai:stream', (_event, text) => callback(text)),
  onAIStreamEnd: (callback: () => void) =>
    ipcRenderer.on('ai:streamEnd', () => callback()),

  // Terminal
  terminalCreate: (cwd?: string) => ipcRenderer.invoke('terminal:create', cwd),
  terminalWrite: (id: string, data: string) => ipcRenderer.send('terminal:write', id, data),
  terminalResize: (id: string, cols: number, rows: number) => ipcRenderer.send('terminal:resize', id, cols, rows),
  terminalKill: (id: string) => ipcRenderer.send('terminal:kill', id),
  onTerminalData: (callback: (id: string, data: string) => void) =>
    ipcRenderer.on('terminal:data', (_event, id, data) => callback(id, data)),
  onTerminalExit: (callback: (id: string, exitCode: number) => void) =>
    ipcRenderer.on('terminal:exit', (_event, id, exitCode) => callback(id, exitCode)),

  // Monitoring
  monitoringStart: (url: string, intervalMs?: number) =>
    ipcRenderer.invoke('monitoring:start', url, intervalMs),
  monitoringStop: (id: string) => ipcRenderer.invoke('monitoring:stop', id),
  monitoringGetMetrics: (id: string) => ipcRenderer.invoke('monitoring:getMetrics', id),
  monitoringAddAlert: (id: string, rule: { type: string; threshold: number }) =>
    ipcRenderer.invoke('monitoring:addAlert', id, rule),
  monitoringGetAlerts: (id: string) => ipcRenderer.invoke('monitoring:getAlerts', id),
  monitoringListActive: () => ipcRenderer.invoke('monitoring:listActive'),
  onMonitoringUpdate: (callback: (data: any) => void) =>
    ipcRenderer.on('monitoring:update', (_event, data) => callback(data)),
  onMonitoringAlert: (callback: (data: any) => void) =>
    ipcRenderer.on('monitoring:alert', (_event, data) => callback(data)),

  // Error Tracking
  errorsReport: (error: { message: string; stack?: string; source?: string; severity: string; projectId: string }) =>
    ipcRenderer.invoke('errors:report', error),
  errorsList: (projectId: string) => ipcRenderer.invoke('errors:list', projectId),
  errorsResolve: (errorId: string) => ipcRenderer.invoke('errors:resolve', errorId),
  errorsGetDetail: (errorId: string) => ipcRenderer.invoke('errors:getDetail', errorId),

  // Cost Tracking
  costsRecord: (entry: { provider: string; model: string; inputTokens: number; outputTokens: number; cost: number }) =>
    ipcRenderer.invoke('costs:record', entry),
  costsGetSummary: (period?: string) => ipcRenderer.invoke('costs:getSummary', period),
  costsGetBudget: () => ipcRenderer.invoke('costs:getBudget'),
  costsSetBudget: (limit: number) => ipcRenderer.invoke('costs:setBudget', limit),
  onCostsBudgetWarning: (callback: (data: any) => void) =>
    ipcRenderer.on('costs:budgetWarning', (_event, data) => callback(data)),
  onCostsBudgetExceeded: (callback: (data: any) => void) =>
    ipcRenderer.on('costs:budgetExceeded', (_event, data) => callback(data)),

  // Auth
  authRegister: (email: string, password: string, name: string) =>
    ipcRenderer.invoke('auth:register', email, password, name),
  authLogin: (email: string, password: string) =>
    ipcRenderer.invoke('auth:login', email, password),
  authLogout: () => ipcRenderer.invoke('auth:logout'),
  authGetCurrentUser: () => ipcRenderer.invoke('auth:getCurrentUser'),
  authUpdatePlan: (plan: string) => ipcRenderer.invoke('auth:updatePlan', plan),
  authUpdateProfile: (data: { name?: string; email?: string }) =>
    ipcRenderer.invoke('auth:updateProfile', data),
  authSocialLogin: (provider: string) => ipcRenderer.invoke('auth:socialLogin', provider),
  authConfigureOAuth: (provider: string, clientId: string) => ipcRenderer.invoke('auth:configureOAuth', provider, clientId),
  authGetOAuthConfig: () => ipcRenderer.invoke('auth:getOAuthConfig'),
  authGetSession: () => ipcRenderer.invoke('auth:getSession'),
  authConfigureSupabase: (url: string, anonKey: string) =>
    ipcRenderer.invoke('auth:configureSupabase', url, anonKey),
  onAuthUserChanged: (callback: (user: any) => void) =>
    ipcRenderer.on('auth:userChanged', (_event, user) => callback(user)),

  // Team
  teamCreate: (name: string) => ipcRenderer.invoke('team:create', name),
  teamInvite: (teamId: string, email: string) =>
    ipcRenderer.invoke('team:invite', teamId, email),
  teamGetMyTeams: () => ipcRenderer.invoke('team:getMyTeams'),
  teamGetMembers: (teamId: string) => ipcRenderer.invoke('team:getMembers', teamId),
  teamRemoveMember: (teamId: string, userId: string) =>
    ipcRenderer.invoke('team:removeMember', teamId, userId),

  // Updater
  updaterCheckForUpdates: () => ipcRenderer.invoke('updater:checkForUpdates'),
  updaterDownloadUpdate: () => ipcRenderer.invoke('updater:downloadUpdate'),
  updaterInstallUpdate: () => ipcRenderer.invoke('updater:installUpdate'),
  updaterGetVersion: () => ipcRenderer.invoke('updater:getVersion'),
  updaterGetChangelog: () => ipcRenderer.invoke('updater:getChangelog'),
  updaterGetAutoUpdateEnabled: () => ipcRenderer.invoke('updater:getAutoUpdateEnabled'),
  updaterSetAutoUpdateEnabled: (enabled: boolean) => ipcRenderer.invoke('updater:setAutoUpdateEnabled', enabled),
  onUpdaterChecking: (callback: () => void) =>
    ipcRenderer.on('updater:checking', () => callback()),
  onUpdaterAvailable: (callback: (info: any) => void) =>
    ipcRenderer.on('updater:available', (_event, info) => callback(info)),
  onUpdaterNotAvailable: (callback: (info: any) => void) =>
    ipcRenderer.on('updater:notAvailable', (_event, info) => callback(info)),
  onUpdaterDownloadProgress: (callback: (progress: any) => void) =>
    ipcRenderer.on('updater:downloadProgress', (_event, progress) => callback(progress)),
  onUpdaterDownloaded: (callback: (info: any) => void) =>
    ipcRenderer.on('updater:downloaded', (_event, info) => callback(info)),
  onUpdaterError: (callback: (error: any) => void) =>
    ipcRenderer.on('updater:error', (_event, error) => callback(error)),

  // Payment
  paymentGetPlans: () => ipcRenderer.invoke('payment:getPlans'),
  paymentGetCurrentSubscription: () => ipcRenderer.invoke('payment:getCurrentSubscription'),
  paymentSubscribe: (planId: string) => ipcRenderer.invoke('payment:subscribe', planId),
  paymentCancel: () => ipcRenderer.invoke('payment:cancel'),
  paymentGetInvoices: () => ipcRenderer.invoke('payment:getInvoices'),
  paymentSetStripeKey: (key: string) => ipcRenderer.invoke('payment:setStripeKey', key),
  paymentCreateCustomerPortalSession: () => ipcRenderer.invoke('payment:createCustomerPortalSession'),
  paymentSyncSubscriptionStatus: () => ipcRenderer.invoke('payment:syncSubscriptionStatus'),
  onPaymentSubscriptionUpdated: (callback: (subscription: any) => void) =>
    ipcRenderer.on('payment:subscriptionUpdated', (_event, subscription) => callback(subscription)),

  // Connections
  connectionsSave: (serviceId: string, credentials: Record<string, string>) =>
    ipcRenderer.invoke('connections:save', serviceId, credentials),
  connectionsVerify: (serviceId: string, credentials: Record<string, string>) =>
    ipcRenderer.invoke('connections:verify', serviceId, credentials),
  connectionsGet: (serviceId: string) =>
    ipcRenderer.invoke('connections:get', serviceId),
  connectionsGetAll: () =>
    ipcRenderer.invoke('connections:getAll'),
  connectionsDelete: (serviceId: string) =>
    ipcRenderer.invoke('connections:delete', serviceId),

  // Rust ai_agent IPC bridge
  agentPing: () => ipcRenderer.invoke('agent:ping'),
  agentInit: () => ipcRenderer.invoke('agent:init'),
  agentChat: (prompt: string, callerId?: string) =>
    ipcRenderer.invoke('agent:chat', prompt, callerId),
  agentSprintRun: (project: string, request: string) =>
    ipcRenderer.invoke('agent:sprintRun', project, request),
  agentBoardStatus: (project: string) =>
    ipcRenderer.invoke('agent:boardStatus', project),
  agentCapabilities: () => ipcRenderer.invoke('agent:capabilities'),
  agentKill: () => ipcRenderer.invoke('agent:kill'),
  agentCheckBinary: () => ipcRenderer.invoke('agent:checkBinary'),
  onAgentStream: (callback: (text: string) => void) =>
    ipcRenderer.on('agent:stream', (_event, text) => callback(text)),
  onAgentSprintProgress: (callback: (data: any) => void) =>
    ipcRenderer.on('agent:sprintProgress', (_event, data) => callback(data)),
  onAgentLog: (callback: (text: string) => void) =>
    ipcRenderer.on('agent:log', (_event, text) => callback(text)),
  onAgentExit: (callback: (code: number | null) => void) =>
    ipcRenderer.on('agent:exit', (_event, code) => callback(code)),
});
