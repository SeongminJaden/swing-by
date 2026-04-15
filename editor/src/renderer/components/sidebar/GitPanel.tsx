import React, { useState, useEffect, useCallback } from 'react';
import { GitBranch, Sparkles, ArrowUp, ArrowDown, RefreshCw, Loader2, Plus, Minus, Check } from 'lucide-react';
import { useFileStore } from '../../stores/fileStore';

interface GitStatus {
  branch: string;
  staged: string[];
  modified: string[];
  not_added: string[];
  deleted: string[];
  ahead: number;
  behind: number;
  isClean: boolean;
}

export function GitPanel() {
  const { workspacePath } = useFileStore();
  const [isRepo, setIsRepo] = useState(false);
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState<GitStatus | null>(null);
  const [commitMsg, setCommitMsg] = useState('');
  const [committing, setCommitting] = useState(false);
  const [pushing, setPushing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const api = window.electronAPI as any;

  const refresh = useCallback(async () => {
    if (!api || !workspacePath) return;
    setLoading(true);
    setError(null);

    try {
      const repo = await api.gitIsRepo(workspacePath);
      setIsRepo(repo);

      if (repo) {
        const result = await api.gitStatus(workspacePath);
        if (result.success) {
          setStatus(result.data);
        } else {
          setError(result.error);
        }
      }
    } catch (e: any) {
      setError(e.message);
    }

    setLoading(false);
  }, [workspacePath]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleInit = async () => {
    if (!api || !workspacePath) return;
    const result = await api.gitInit(workspacePath);
    if (result.success) refresh();
    else setError(result.error);
  };

  const handleStageAll = async () => {
    if (!api || !workspacePath) return;
    await api.gitAdd(workspacePath, ['.']);
    refresh();
  };

  const handleStageFile = async (file: string) => {
    if (!api || !workspacePath) return;
    await api.gitAdd(workspacePath, [file]);
    refresh();
  };

  const handleUnstageFile = async (file: string) => {
    if (!api || !workspacePath) return;
    await api.gitReset(workspacePath, [file]);
    refresh();
  };

  const handleCommit = async () => {
    if (!api || !workspacePath || !commitMsg.trim()) return;
    setCommitting(true);
    const result = await api.gitCommit(workspacePath, commitMsg.trim());
    setCommitting(false);
    if (result.success) {
      setCommitMsg('');
      refresh();
    } else {
      setError(result.error);
    }
  };

  const handlePush = async () => {
    if (!api || !workspacePath) return;
    setPushing(true);
    const result = await api.gitPush(workspacePath);
    setPushing(false);
    if (result.success) refresh();
    else setError(result.error);
  };

  const handlePull = async () => {
    if (!api || !workspacePath) return;
    setLoading(true);
    const result = await api.gitPull(workspacePath);
    setLoading(false);
    if (result.success) refresh();
    else setError(result.error);
  };

  // No folder open
  if (!workspacePath) {
    return (
      <div className="git-panel" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', padding: '24px' }}>
        <p style={{ color: 'var(--color-text-tertiary)', fontSize: '13px', textAlign: 'center' }}>
          폴더를 먼저 열어주세요
        </p>
      </div>
    );
  }

  // Not a git repo
  if (!loading && !isRepo) {
    return (
      <div className="git-panel" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', padding: '24px', gap: '12px' }}>
        <GitBranch size={32} style={{ color: 'var(--color-text-tertiary)' }} />
        <p style={{ color: 'var(--color-text-tertiary)', fontSize: '13px', textAlign: 'center' }}>
          Git 저장소가 아닙니다
        </p>
        <button onClick={handleInit} className="btn-primary" style={{ padding: '8px 20px', fontSize: '12px' }}>
          Git 초기화
        </button>
      </div>
    );
  }

  const allChanges = [
    ...(status?.modified || []).map(f => ({ path: f, status: 'M' as const })),
    ...(status?.not_added || []).map(f => ({ path: f, status: 'A' as const })),
    ...(status?.deleted || []).map(f => ({ path: f, status: 'D' as const })),
  ];
  const stagedFiles = status?.staged || [];
  const changeCount = allChanges.length + stagedFiles.length;

  return (
    <div className="git-panel">
      {/* Branch */}
      <div className="git-branch-bar">
        <GitBranch size={14} style={{ color: 'var(--color-accent-primary)' }} />
        <span className="git-branch-name">{status?.branch || '...'}</span>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: '4px' }}>
          <button onClick={refresh} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '24px', height: '24px', borderRadius: '6px', color: 'var(--color-text-tertiary)', background: 'none', border: 'none', cursor: 'pointer' }}>
            {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
          </button>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div style={{ padding: '8px 12px', fontSize: '11px', color: 'var(--color-accent-error)', background: 'rgba(248,81,73,0.1)' }}>
          {error}
        </div>
      )}

      {/* Staged */}
      {stagedFiles.length > 0 && (
        <>
          <div className="git-changes-header">
            <span className="git-changes-title">스테이지됨 ({stagedFiles.length})</span>
          </div>
          {stagedFiles.map((file) => (
            <div key={file} className="git-change-item git-change-item-staged">
              <div className="devenv-item-left">
                <span style={{ color: 'var(--color-accent-success)', fontSize: '11px', fontWeight: 700, width: '16px' }}>S</span>
                <span className="git-change-path">{file}</span>
              </div>
              <button onClick={() => handleUnstageFile(file)} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '20px', height: '20px', borderRadius: '4px', color: 'var(--color-text-tertiary)', background: 'none', border: 'none', cursor: 'pointer' }} title="Unstage">
                <Minus size={12} />
              </button>
            </div>
          ))}
        </>
      )}

      {/* Changes */}
      <div className="git-changes-header">
        <span className="git-changes-title">변경 사항 ({allChanges.length})</span>
        {allChanges.length > 0 && (
          <button onClick={handleStageAll} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '20px', height: '20px', borderRadius: '4px', color: 'var(--color-text-tertiary)', background: 'none', border: 'none', cursor: 'pointer' }} title="전체 Stage">
            <Plus size={13} />
          </button>
        )}
      </div>

      <div style={{ flex: 1, overflow: 'auto' }}>
        {allChanges.length === 0 && stagedFiles.length === 0 && (
          <p style={{ padding: '12px', color: 'var(--color-text-tertiary)', fontSize: '12px', textAlign: 'center' }}>
            {status?.isClean ? '변경 사항 없음' : '로딩 중...'}
          </p>
        )}
        {allChanges.map(({ path, status: s }) => (
          <div key={path} className="git-change-item">
            <div className="devenv-item-left">
              <span style={{
                color: s === 'M' ? 'var(--color-accent-warning)' : s === 'A' ? 'var(--color-accent-success)' : 'var(--color-accent-error)',
                fontSize: '11px', fontWeight: 700, width: '16px',
              }}>{s}</span>
              <span className="git-change-path">{path}</span>
            </div>
            <button onClick={() => handleStageFile(path)} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '20px', height: '20px', borderRadius: '4px', color: 'var(--color-text-tertiary)', background: 'none', border: 'none', cursor: 'pointer' }} title="Stage">
              <Plus size={12} />
            </button>
          </div>
        ))}
      </div>

      {/* Commit area */}
      <div style={{ borderTop: '1px solid var(--color-border-primary)', padding: '8px' }}>
        <textarea
          value={commitMsg}
          onChange={(e) => setCommitMsg(e.target.value)}
          placeholder="커밋 메시지를 입력하세요"
          className="git-commit-input"
          rows={2}
        />
        <div style={{ display: 'flex', gap: '6px', marginTop: '6px' }}>
          <button
            onClick={handleCommit}
            disabled={committing || !commitMsg.trim() || (stagedFiles.length === 0 && allChanges.length === 0)}
            className="git-commit-btn"
            style={{ flex: 1, opacity: committing || !commitMsg.trim() ? 0.5 : 1 }}
          >
            {committing ? <Loader2 size={13} className="animate-spin" /> : <Check size={13} />}
            커밋
          </button>
          <button
            onClick={handlePush}
            disabled={pushing}
            className="git-commit-btn"
            style={{ flex: 1, background: 'var(--color-bg-elevated)', color: 'var(--color-text-secondary)', opacity: pushing ? 0.5 : 1 }}
          >
            {pushing ? <Loader2 size={13} className="animate-spin" /> : <ArrowUp size={13} />}
            푸시
          </button>
        </div>
      </div>

      {/* Sync status */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '12px', padding: '6px', borderTop: '1px solid var(--color-border-primary)', fontSize: '11px', color: 'var(--color-text-tertiary)' }}>
        <span style={{ display: 'flex', alignItems: 'center', gap: '3px' }}><ArrowUp size={11} /> {status?.ahead || 0}</span>
        <span style={{ display: 'flex', alignItems: 'center', gap: '3px' }}><ArrowDown size={11} /> {status?.behind || 0}</span>
      </div>
    </div>
  );
}

export default GitPanel;
