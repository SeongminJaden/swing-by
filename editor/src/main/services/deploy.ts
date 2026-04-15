import { ipcMain } from 'electron';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';

interface DeployResult {
  success: boolean;
  url?: string;
  error?: string;
  buildLog?: string;
}

interface FrameworkInfo {
  framework: string;
  buildCmd: string;
  outputDir: string;
}

interface PlatformInfo {
  id: string;
  name: string;
  installed: boolean;
  description: string;
}

function isCliInstalled(command: string): boolean {
  try {
    execSync(`which ${command} 2>/dev/null || where ${command} 2>NUL`, {
      encoding: 'utf-8',
      timeout: 5000,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    return true;
  } catch {
    return false;
  }
}

function detectFrameworkFromPackageJson(cwd: string): FrameworkInfo {
  const pkgPath = path.join(cwd, 'package.json');

  if (!fs.existsSync(pkgPath)) {
    return { framework: 'static', buildCmd: '', outputDir: '.' };
  }

  try {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf-8'));
    const deps = { ...pkg.dependencies, ...pkg.devDependencies };
    const scripts = pkg.scripts || {};

    // Next.js
    if (deps['next']) {
      return { framework: 'nextjs', buildCmd: 'npm run build', outputDir: '.next' };
    }

    // Nuxt
    if (deps['nuxt'] || deps['nuxt3']) {
      return { framework: 'nuxt', buildCmd: 'npm run build', outputDir: '.output/public' };
    }

    // Astro
    if (deps['astro']) {
      return { framework: 'astro', buildCmd: 'npm run build', outputDir: 'dist' };
    }

    // SvelteKit
    if (deps['@sveltejs/kit']) {
      return { framework: 'sveltekit', buildCmd: 'npm run build', outputDir: 'build' };
    }

    // Remix
    if (deps['@remix-run/react'] || deps['remix']) {
      return { framework: 'remix', buildCmd: 'npm run build', outputDir: 'build' };
    }

    // Vite (generic)
    if (deps['vite']) {
      return { framework: 'vite', buildCmd: 'npm run build', outputDir: 'dist' };
    }

    // Create React App
    if (deps['react-scripts']) {
      return { framework: 'create-react-app', buildCmd: 'npm run build', outputDir: 'build' };
    }

    // Vue CLI
    if (deps['@vue/cli-service']) {
      return { framework: 'vue-cli', buildCmd: 'npm run build', outputDir: 'dist' };
    }

    // Angular
    if (deps['@angular/core']) {
      const projectName = pkg.name || 'app';
      return { framework: 'angular', buildCmd: 'npm run build', outputDir: `dist/${projectName}` };
    }

    // Gatsby
    if (deps['gatsby']) {
      return { framework: 'gatsby', buildCmd: 'npm run build', outputDir: 'public' };
    }

    // Has a build script but unknown framework
    if (scripts['build']) {
      return { framework: 'unknown', buildCmd: 'npm run build', outputDir: 'dist' };
    }

    return { framework: 'static', buildCmd: '', outputDir: '.' };
  } catch {
    return { framework: 'static', buildCmd: '', outputDir: '.' };
  }
}

export function registerDeployHandlers() {
  // Check if Vercel CLI is installed
  ipcMain.handle('deploy:checkVercel', async () => {
    try {
      const output = execSync('npx vercel --version 2>/dev/null', { encoding: 'utf-8', timeout: 10000 });
      return { installed: true, version: output.trim() };
    } catch {
      return { installed: false };
    }
  });

  // Deploy to Vercel
  ipcMain.handle('deploy:vercel', async (_event, cwd: string, options?: { prod?: boolean; token?: string }) => {
    try {
      let cmd = 'npx vercel';
      if (options?.prod) cmd += ' --prod';
      if (options?.token) cmd += ` --token ${options.token}`;
      cmd += ' --yes 2>&1';

      const output = execSync(cmd, {
        cwd,
        encoding: 'utf-8',
        timeout: 300000,
        env: { ...process.env },
      });

      const urlMatch = output.match(/(https:\/\/[^\s]+\.vercel\.app)/);
      const url = urlMatch ? urlMatch[1] : undefined;

      return { success: true, url, buildLog: output } as DeployResult;
    } catch (e: any) {
      return { success: false, error: e.message, buildLog: e.stdout || e.stderr || '' } as DeployResult;
    }
  });

  // Deploy to Railway
  ipcMain.handle('deploy:railway', async (_event, cwd: string) => {
    try {
      const output = execSync('railway up --detach 2>&1', {
        cwd,
        encoding: 'utf-8',
        timeout: 300000,
        env: { ...process.env },
      });

      return { success: true, buildLog: output } as DeployResult;
    } catch (e: any) {
      return { success: false, error: e.message, buildLog: e.stdout || '' } as DeployResult;
    }
  });

  // Deploy to Netlify
  ipcMain.handle('deploy:netlify', async (_event, cwd: string, token?: string) => {
    try {
      const framework = detectFrameworkFromPackageJson(cwd);

      // Build first if needed
      if (framework.buildCmd) {
        execSync(framework.buildCmd, {
          cwd,
          encoding: 'utf-8',
          timeout: 300000,
          env: { ...process.env },
          stdio: ['pipe', 'pipe', 'pipe'],
        });
      }

      let cmd = `npx netlify deploy --prod --dir ${framework.outputDir}`;
      if (token) {
        cmd = `NETLIFY_AUTH_TOKEN=${token} ${cmd}`;
      }
      cmd += ' 2>&1';

      const output = execSync(cmd, {
        cwd,
        encoding: 'utf-8',
        timeout: 300000,
        env: { ...process.env },
      });

      // Extract URL from output
      const urlMatch = output.match(/(https:\/\/[^\s]+\.netlify\.app)/);
      const url = urlMatch ? urlMatch[1] : undefined;

      return { success: true, url, buildLog: output } as DeployResult;
    } catch (e: any) {
      return { success: false, error: e.message, buildLog: e.stdout || e.stderr || '' } as DeployResult;
    }
  });

  // Deploy to Cloudflare Pages
  ipcMain.handle('deploy:cloudflare', async (_event, cwd: string, token?: string) => {
    try {
      const framework = detectFrameworkFromPackageJson(cwd);

      // Build first if needed
      if (framework.buildCmd) {
        execSync(framework.buildCmd, {
          cwd,
          encoding: 'utf-8',
          timeout: 300000,
          env: { ...process.env },
          stdio: ['pipe', 'pipe', 'pipe'],
        });
      }

      let cmd = `npx wrangler pages deploy ${framework.outputDir}`;
      if (token) {
        cmd = `CLOUDFLARE_API_TOKEN=${token} ${cmd}`;
      }
      cmd += ' 2>&1';

      const output = execSync(cmd, {
        cwd,
        encoding: 'utf-8',
        timeout: 300000,
        env: { ...process.env },
      });

      // Extract URL from output
      const urlMatch = output.match(/(https:\/\/[^\s]+\.pages\.dev)/);
      const url = urlMatch ? urlMatch[1] : undefined;

      return { success: true, url, buildLog: output } as DeployResult;
    } catch (e: any) {
      return { success: false, error: e.message, buildLog: e.stdout || e.stderr || '' } as DeployResult;
    }
  });

  // Detect framework in a project
  ipcMain.handle('deploy:detectFramework', async (_event, cwd: string) => {
    return detectFrameworkFromPackageJson(cwd);
  });

  // List available deploy platforms
  ipcMain.handle('deploy:listPlatforms', async () => {
    const platforms: PlatformInfo[] = [
      {
        id: 'vercel',
        name: 'Vercel',
        installed: isCliInstalled('vercel'),
        description: 'Deploy frontend and serverless functions with zero configuration',
      },
      {
        id: 'netlify',
        name: 'Netlify',
        installed: isCliInstalled('netlify'),
        description: 'Build, deploy, and manage modern web projects',
      },
      {
        id: 'cloudflare',
        name: 'Cloudflare Pages',
        installed: isCliInstalled('wrangler'),
        description: 'Deploy full-stack applications on Cloudflare global network',
      },
      {
        id: 'railway',
        name: 'Railway',
        installed: isCliInstalled('railway'),
        description: 'Deploy apps, databases, and cron jobs with instant infrastructure',
      },
    ];

    return platforms;
  });

  // Check deploy status (generic)
  ipcMain.handle('deploy:status', async (_event, url: string) => {
    try {
      const https = require('https');
      const http = require('http');
      const client = url.startsWith('https') ? https : http;

      return new Promise<{ online: boolean; statusCode?: number }>((resolve) => {
        const req = client.get(url, { timeout: 10000 }, (res: any) => {
          resolve({ online: true, statusCode: res.statusCode });
        });
        req.on('error', () => resolve({ online: false }));
        req.on('timeout', () => {
          req.destroy();
          resolve({ online: false });
        });
      });
    } catch {
      return { online: false };
    }
  });

  // Set Vercel token
  ipcMain.handle('deploy:setVercelToken', async (_event, token: string) => {
    try {
      const output = execSync(`npx vercel whoami --token ${token} 2>&1`, {
        encoding: 'utf-8',
        timeout: 10000,
      });
      return { success: true, user: output.trim() };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // List Vercel projects
  ipcMain.handle('deploy:vercelProjects', async (_event, token: string) => {
    try {
      const output = execSync(`npx vercel ls --token ${token} 2>&1`, {
        encoding: 'utf-8',
        timeout: 15000,
      });
      return { success: true, data: output };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });
}
