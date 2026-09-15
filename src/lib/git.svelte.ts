import { invoke } from '@tauri-apps/api/core';
import { session } from './session.svelte';

export interface GitFileChange {
  path: string;
  status: 'modified' | 'added' | 'deleted' | 'renamed' | 'untracked';
  oldPath?: string | null;
}

export interface GitStatusResult {
  gitAvailable: boolean;
  isRepo: boolean;
  branch: string;
  ahead: number;
  behind: number;
  staged: GitFileChange[];
  unstaged: GitFileChange[];
  untracked: GitFileChange[];
}

export interface GitBranchInfo {
  name: string;
  current: boolean;
  isRemote: boolean;
}

export interface DiffModalState {
  open: boolean;
  path: string;
  staged: boolean;
  diff: string;
}

export class GitStore {
  gitAvailable = $state(true);
  isRepo = $state(false);
  branch = $state('');
  ahead = $state(0);
  behind = $state(0);
  staged = $state<GitFileChange[]>([]);
  unstaged = $state<GitFileChange[]>([]);
  untracked = $state<GitFileChange[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  diffModal = $state<DiffModalState>({
    open: false,
    path: '',
    staged: false,
    diff: '',
  });
  branchModal = $state(false);
  branches = $state<GitBranchInfo[]>([]);

  totalChanges = $derived(
    this.staged.length + this.unstaged.length + this.untracked.length
  );

  statusMap = $derived.by(() => {
    const map = new Map<string, string>();
    for (const item of this.untracked) {
      map.set(item.path, 'U');
    }
    for (const item of this.unstaged) {
      if (item.status === 'deleted') map.set(item.path, 'D');
      else if (item.status === 'added') map.set(item.path, 'A');
      else map.set(item.path, 'M');
    }
    for (const item of this.staged) {
      if (item.status === 'deleted') map.set(item.path, 'D');
      else if (item.status === 'added') map.set(item.path, 'A');
      else if (item.status === 'renamed') map.set(item.path, 'R');
      else if (!map.has(item.path)) map.set(item.path, 'M');
    }
    return map;
  });

  private refreshDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  async refresh() {
    if (!session.root) {
      this.isRepo = false;
      this.branch = '';
      this.ahead = 0;
      this.behind = 0;
      this.staged = [];
      this.unstaged = [];
      this.untracked = [];
      this.loading = false;
      this.error = null;
      return;
    }

    this.loading = true;
    try {
      const res = await invoke<GitStatusResult>('git_status', {
        workspaceRoot: session.root,
      });
      this.gitAvailable = res.gitAvailable ?? true;
      this.isRepo = res.isRepo;
      this.branch = res.branch;
      this.ahead = res.ahead;
      this.behind = res.behind;
      this.staged = res.staged;
      this.unstaged = res.unstaged;
      this.untracked = res.untracked;
      this.error = null;
    } catch (err) {
      const errStr = String(err);
      this.error = errStr;
      this.isRepo = false;
      if (errStr.includes('git executable not found') || errStr.includes('No such file or directory')) {
        this.gitAvailable = false;
      }
    } finally {
      this.loading = false;
    }
  }

  debouncedRefresh(delayMs = 250) {
    if (this.refreshDebounceTimer) {
      clearTimeout(this.refreshDebounceTimer);
    }
    this.refreshDebounceTimer = setTimeout(() => {
      this.refreshDebounceTimer = null;
      void this.refresh();
    }, delayMs);
  }

  async stage(paths: string[] = []) {
    try {
      this.loading = true;
      await invoke('git_stage', { workspaceRoot: session.root, paths });
      await this.refresh();
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async unstage(paths: string[] = []) {
    try {
      this.loading = true;
      await invoke('git_unstage', { workspaceRoot: session.root, paths });
      await this.refresh();
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async discard(paths: string[] = []) {
    try {
      this.loading = true;
      await invoke('git_discard', { workspaceRoot: session.root, paths });
      session.revision++;
      await this.refresh();
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async commit(message: string, amend = false) {
    try {
      this.loading = true;
      await invoke('git_commit', {
        workspaceRoot: session.root,
        message,
        amend,
      });
      await this.refresh();
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async push() {
    try {
      this.loading = true;
      const res = await invoke<string>('git_push', {
        workspaceRoot: session.root,
      });
      await this.refresh();
      return res;
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async pull() {
    try {
      this.loading = true;
      const res = await invoke<string>('git_pull', {
        workspaceRoot: session.root,
      });
      session.revision++;
      await this.refresh();
      return res;
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async openDiff(path: string, staged: boolean) {
    try {
      this.loading = true;
      const diff = await invoke<string>('git_diff', {
        workspaceRoot: session.root,
        path,
        staged,
      });
      this.diffModal = {
        open: true,
        path,
        staged,
        diff,
      };
    } catch (err) {
      this.error = String(err);
    } finally {
      this.loading = false;
    }
  }

  closeDiff() {
    this.diffModal = {
      open: false,
      path: '',
      staged: false,
      diff: '',
    };
  }

  async loadBranches() {
    try {
      const branches = await invoke<GitBranchInfo[]>('git_branches', {
        workspaceRoot: session.root,
      });
      this.branches = branches;
    } catch (err) {
      this.error = String(err);
    }
  }

  async openBranchModal() {
    await this.loadBranches();
    this.branchModal = true;
  }

  closeBranchModal() {
    this.branchModal = false;
  }

  async checkoutBranch(branch: string) {
    try {
      this.loading = true;
      await invoke('git_checkout', {
        workspaceRoot: session.root,
        branch,
      });
      this.branchModal = false;
      session.revision++;
      await this.refresh();
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async createBranch(name: string) {
    try {
      this.loading = true;
      await invoke('git_create_branch', {
        workspaceRoot: session.root,
        name,
      });
      this.branchModal = false;
      session.revision++;
      await this.refresh();
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }

  async initRepo() {
    try {
      this.loading = true;
      await invoke('git_init', {
        workspaceRoot: session.root,
      });
      session.revision++;
      await this.refresh();
    } catch (err) {
      this.error = String(err);
      throw err;
    } finally {
      this.loading = false;
    }
  }
}

export const gitStore = new GitStore();
