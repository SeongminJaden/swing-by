import { ipcMain } from 'electron';
import simpleGit, { SimpleGit, StatusResult } from 'simple-git';

function getGit(cwd: string): SimpleGit {
  return simpleGit(cwd);
}

export function registerGitHandlers() {
  // Check if directory is a git repo
  ipcMain.handle('git:isRepo', async (_event, cwd: string) => {
    try {
      const git = getGit(cwd);
      return await git.checkIsRepo();
    } catch {
      return false;
    }
  });

  // Init repo
  ipcMain.handle('git:init', async (_event, cwd: string) => {
    try {
      const git = getGit(cwd);
      await git.init();
      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Get status
  ipcMain.handle('git:status', async (_event, cwd: string) => {
    try {
      const git = getGit(cwd);
      const status: StatusResult = await git.status();
      return {
        success: true,
        data: {
          branch: status.current,
          staged: status.staged,
          modified: status.modified,
          not_added: status.not_added,
          deleted: status.deleted,
          ahead: status.ahead,
          behind: status.behind,
          isClean: status.isClean(),
        },
      };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Stage files
  ipcMain.handle('git:add', async (_event, cwd: string, files: string[]) => {
    try {
      const git = getGit(cwd);
      await git.add(files);
      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Unstage files
  ipcMain.handle('git:reset', async (_event, cwd: string, files: string[]) => {
    try {
      const git = getGit(cwd);
      await git.reset(['HEAD', '--', ...files]);
      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Commit
  ipcMain.handle('git:commit', async (_event, cwd: string, message: string) => {
    try {
      const git = getGit(cwd);
      const result = await git.commit(message);
      return { success: true, data: { hash: result.commit, summary: result.summary } };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Push
  ipcMain.handle('git:push', async (_event, cwd: string, remote?: string, branch?: string) => {
    try {
      const git = getGit(cwd);
      await git.push(remote || 'origin', branch);
      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Pull
  ipcMain.handle('git:pull', async (_event, cwd: string, remote?: string, branch?: string) => {
    try {
      const git = getGit(cwd);
      const result = await git.pull(remote || 'origin', branch);
      return { success: true, data: { changes: result.summary.changes, insertions: result.summary.insertions, deletions: result.summary.deletions } };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Log
  ipcMain.handle('git:log', async (_event, cwd: string, maxCount?: number) => {
    try {
      const git = getGit(cwd);
      const log = await git.log({ maxCount: maxCount || 20 });
      return {
        success: true,
        data: log.all.map(c => ({
          hash: c.hash.substring(0, 7),
          message: c.message,
          author: c.author_name,
          date: c.date,
        })),
      };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Diff
  ipcMain.handle('git:diff', async (_event, cwd: string, file?: string) => {
    try {
      const git = getGit(cwd);
      const diff = file ? await git.diff([file]) : await git.diff();
      return { success: true, data: diff };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Branch list
  ipcMain.handle('git:branches', async (_event, cwd: string) => {
    try {
      const git = getGit(cwd);
      const branches = await git.branchLocal();
      return {
        success: true,
        data: {
          current: branches.current,
          all: branches.all,
        },
      };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Create branch
  ipcMain.handle('git:createBranch', async (_event, cwd: string, name: string) => {
    try {
      const git = getGit(cwd);
      await git.checkoutLocalBranch(name);
      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Checkout branch
  ipcMain.handle('git:checkout', async (_event, cwd: string, branch: string) => {
    try {
      const git = getGit(cwd);
      await git.checkout(branch);
      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });

  // Get remotes
  ipcMain.handle('git:remotes', async (_event, cwd: string) => {
    try {
      const git = getGit(cwd);
      const remotes = await git.getRemotes(true);
      return {
        success: true,
        data: remotes.map(r => ({ name: r.name, fetch: r.refs.fetch, push: r.refs.push })),
      };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  });
}
