export {};

declare global {
  interface Window {
    electronAPI?: {
      // App info
      getAppInfo: () => Promise<any>;
      onNavigate: (callback: (route: string) => void) => void;

      // Window controls
      windowMinimize: () => void;
      windowMaximize: () => void;
      windowClose: () => void;

      // File system
      openFolder: () => Promise<string | null>;
      readDir: (dirPath: string) => Promise<any[]>;
      readFile: (filePath: string) => Promise<string | null>;
      writeFile: (filePath: string, content: string) => Promise<boolean>;
      mkdir: (dirPath: string) => Promise<boolean>;
      deleteFile: (targetPath: string) => Promise<boolean>;
      fileExists: (targetPath: string) => Promise<boolean>;
      fileStat: (targetPath: string) => Promise<any>;

      // Dev environment
      execCommand: (cmd: string) => Promise<{ success: boolean; output: string }>;

      // Deploy
      deployCheckVercel: () => Promise<any>;
      deployVercel: (cwd: string, options?: { prod?: boolean; token?: string }) => Promise<any>;
      deployRailway: (cwd: string) => Promise<any>;
      deployNetlify: (cwd: string, token?: string) => Promise<any>;
      deployCloudflare: (cwd: string, token?: string) => Promise<any>;
      deployDetectFramework: (cwd: string) => Promise<any>;
      deployListPlatforms: () => Promise<any>;
      deployStatus: (url: string) => Promise<any>;
      deploySetVercelToken: (token: string) => Promise<any>;
      deployVercelProjects: (token: string) => Promise<any>;

      // Security
      securityScan: (cwd: string) => Promise<any>;
      securityCheckSecrets: (content: string) => Promise<any>;

      // Git
      gitIsRepo: (cwd: string) => Promise<any>;
      gitInit: (cwd: string) => Promise<any>;
      gitStatus: (cwd: string) => Promise<any>;
      gitAdd: (cwd: string, files: string[]) => Promise<any>;
      gitReset: (cwd: string, files: string[]) => Promise<any>;
      gitCommit: (cwd: string, message: string) => Promise<any>;
      gitPush: (cwd: string, remote?: string, branch?: string) => Promise<any>;
      gitPull: (cwd: string, remote?: string, branch?: string) => Promise<any>;
      gitLog: (cwd: string, maxCount?: number) => Promise<any>;
      gitDiff: (cwd: string, file?: string) => Promise<any>;
      gitBranches: (cwd: string) => Promise<any>;
      gitCreateBranch: (cwd: string, name: string) => Promise<any>;
      gitCheckout: (cwd: string, branch: string) => Promise<any>;
      gitRemotes: (cwd: string) => Promise<any>;

      // AI
      aiSetKey: (provider: string, key: string) => Promise<boolean>;
      aiGetKey: (provider: string) => Promise<any>;
      aiHasKey: (provider: string) => Promise<boolean>;
      aiChatClaude: (messages: { role: string; content: string }[], model?: string) => Promise<any>;
      aiChatOpenAI: (messages: { role: string; content: string }[], model?: string) => Promise<any>;
      onAIStream: (callback: (text: string) => void) => void;
      onAIStreamEnd: (callback: () => void) => void;

      // Terminal
      terminalCreate: (cwd?: string) => Promise<string>;
      terminalWrite: (id: string, data: string) => void;
      terminalResize: (id: string, cols: number, rows: number) => void;
      terminalKill: (id: string) => void;
      onTerminalData: (callback: (id: string, data: string) => void) => void;
      onTerminalExit: (callback: (id: string, exitCode: number) => void) => void;

      // Monitoring
      monitoringStart: (url: string, intervalMs?: number) => Promise<any>;
      monitoringStop: (id: string) => Promise<any>;
      monitoringGetMetrics: (id: string) => Promise<any>;
      monitoringAddAlert: (id: string, rule: { type: string; threshold: number }) => Promise<any>;
      monitoringGetAlerts: (id: string) => Promise<any>;
      monitoringListActive: () => Promise<any>;
      onMonitoringUpdate: (callback: (data: any) => void) => void;
      onMonitoringAlert: (callback: (data: any) => void) => void;

      // Error Tracking
      errorsReport: (error: { message: string; stack?: string; source?: string; severity: string; projectId: string }) => Promise<any>;
      errorsList: (projectId: string) => Promise<any>;
      errorsResolve: (errorId: string) => Promise<any>;
      errorsGetDetail: (errorId: string) => Promise<any>;

      // Cost Tracking
      costsRecord: (entry: { provider: string; model: string; inputTokens: number; outputTokens: number; cost: number }) => Promise<any>;
      costsGetSummary: (period?: string) => Promise<any>;
      costsGetBudget: () => Promise<any>;
      costsSetBudget: (limit: number) => Promise<any>;
      onCostsBudgetWarning: (callback: (data: any) => void) => void;
      onCostsBudgetExceeded: (callback: (data: any) => void) => void;

      // Auth
      authRegister: (email: string, password: string, name: string) => Promise<any>;
      authLogin: (email: string, password: string) => Promise<any>;
      authLogout: () => Promise<any>;
      authGetCurrentUser: () => Promise<any>;
      authUpdatePlan: (plan: string) => Promise<any>;
      authUpdateProfile: (data: { name?: string; email?: string }) => Promise<any>;
      onAuthUserChanged: (callback: (user: any) => void) => void;

      // Team
      teamCreate: (name: string) => Promise<any>;
      teamInvite: (teamId: string, email: string) => Promise<any>;
      teamGetMyTeams: () => Promise<any>;
      teamGetMembers: (teamId: string) => Promise<any>;
      teamRemoveMember: (teamId: string, userId: string) => Promise<any>;

      // Updater
      updaterCheckForUpdates: () => Promise<any>;
      updaterDownloadUpdate: () => Promise<any>;
      updaterInstallUpdate: () => Promise<any>;
      updaterGetVersion: () => Promise<any>;
      updaterGetChangelog: () => Promise<any>;
      updaterGetAutoUpdateEnabled: () => Promise<any>;
      updaterSetAutoUpdateEnabled: (enabled: boolean) => Promise<any>;
      onUpdaterChecking: (callback: () => void) => void;
      onUpdaterAvailable: (callback: (info: any) => void) => void;
      onUpdaterNotAvailable: (callback: (info: any) => void) => void;
      onUpdaterDownloadProgress: (callback: (progress: any) => void) => void;
      onUpdaterDownloaded: (callback: (info: any) => void) => void;
      onUpdaterError: (callback: (error: any) => void) => void;

      // Payment
      paymentGetPlans: () => Promise<any>;
      paymentGetCurrentSubscription: () => Promise<any>;
      paymentSubscribe: (planId: string) => Promise<any>;
      paymentCancel: () => Promise<any>;
      paymentGetInvoices: () => Promise<any>;
      paymentSetStripeKey: (key: string) => Promise<any>;
      paymentCreateCustomerPortalSession: () => Promise<any>;
      paymentSyncSubscriptionStatus: () => Promise<any>;
      onPaymentSubscriptionUpdated: (callback: (subscription: any) => void) => void;

      // Connections
      connectionsSave: (serviceId: string, credentials: Record<string, string>) => Promise<any>;
      connectionsVerify: (serviceId: string, credentials: Record<string, string>) => Promise<any>;
      connectionsGet: (serviceId: string) => Promise<any>;
      connectionsGetAll: () => Promise<any>;
      connectionsDelete: (serviceId: string) => Promise<any>;

      // Rust ai_agent IPC bridge
      agentPing: () => Promise<any>;
      agentInit: () => Promise<any>;
      agentChat: (prompt: string, callerId?: string) => Promise<{ success: boolean; content: string; error?: string }>;
      agentSprintRun: (project: string, request: string) => Promise<any>;
      agentBoardStatus: (project: string) => Promise<{ success: boolean; board: string; error?: string }>;
      agentCapabilities: () => Promise<any>;
      agentKill: () => Promise<any>;
      agentCheckBinary: () => Promise<{ exists: boolean; path: string }>;
      onAgentStream: (callback: (text: string) => void) => void;
      onAgentSprintProgress: (callback: (data: any) => void) => void;
      onAgentLog: (callback: (text: string) => void) => void;
      onAgentExit: (callback: (code: number | null) => void) => void;
    };
    __appStore?: any;
    __projectStore?: any;
  }
}
