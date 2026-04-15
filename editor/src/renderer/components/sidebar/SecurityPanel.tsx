import React, { useState, useCallback } from 'react';
import { Shield, AlertTriangle, AlertCircle, Info, Loader2, RefreshCw } from 'lucide-react';
import { useFileStore } from '../../stores/fileStore';

interface SecurityIssue {
  severity: 'high' | 'medium' | 'low';
  type: string;
  message: string;
  file?: string;
  line?: number;
}

interface ScanResult {
  score: number;
  issues: SecurityIssue[];
  scannedFiles: number;
  timestamp: string;
}

export const SecurityPanel: React.FC = () => {
  const workspacePath = useFileStore((s) => s.workspacePath);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleScan = useCallback(async () => {
    if (!workspacePath || !(window as any).electronAPI) return;
    setLoading(true);
    setError(null);
    try {
      const res = await (window as any).electronAPI.securityScan(workspacePath);
      if (res && typeof res === 'object') {
        setResult(res as ScanResult);
      } else {
        setError('보안 스캔 결과를 가져올 수 없습니다');
      }
    } catch (err: any) {
      setError(err.message || '보안 스캔에 실패했습니다');
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  const scoreColor = (score: number) => {
    if (score > 70) return 'security-score-good';
    if (score >= 40) return 'security-score-warn';
    return 'security-score-bad';
  };

  const severityIcon = (severity: string) => {
    switch (severity) {
      case 'high': return <AlertCircle size={14} />;
      case 'medium': return <AlertTriangle size={14} />;
      default: return <Info size={14} />;
    }
  };

  const groupedIssues = result?.issues.reduce(
    (acc, issue) => {
      acc[issue.severity] = acc[issue.severity] || [];
      acc[issue.severity].push(issue);
      return acc;
    },
    {} as Record<string, SecurityIssue[]>,
  );

  if (!workspacePath) {
    return (
      <div className="sidebar-placeholder">
        <p className="sidebar-placeholder-title">보안 스캔</p>
        <p className="sidebar-placeholder-desc">
          폴더를 열어 보안 스캔을 시작하세요.
        </p>
      </div>
    );
  }

  return (
    <div className="security-panel">
      {/* Scan button */}
      <div className="security-panel-actions">
        <button
          className="security-scan-btn"
          onClick={handleScan}
          disabled={loading}
        >
          {loading ? (
            <Loader2 size={14} className="animate-spin" />
          ) : (
            <Shield size={14} />
          )}
          {loading ? '스캔 중...' : '보안 스캔'}
        </button>
        {result && (
          <button
            className="security-rescan-btn"
            onClick={handleScan}
            disabled={loading}
            title="다시 스캔"
          >
            <RefreshCw size={13} />
          </button>
        )}
      </div>

      {/* Error */}
      {error && (
        <div className="security-error">
          <AlertCircle size={14} />
          <span>{error}</span>
        </div>
      )}

      {/* Loading */}
      {loading && !result && (
        <div className="security-loading">
          <Loader2 size={28} className="animate-spin" style={{ color: 'var(--color-accent-primary)' }} />
          <p>프로젝트를 분석하고 있습니다...</p>
        </div>
      )}

      {/* Result */}
      {result && !loading && (
        <>
          {/* Score */}
          <div className="security-score-card">
            <div className={`security-score ${scoreColor(result.score)}`}>
              {result.score}
            </div>
            <div className="security-score-label">보안 점수</div>
            <div className="security-score-meta">
              {result.scannedFiles}개 파일 스캔됨
            </div>
          </div>

          {/* Issue groups */}
          <div className="security-issues">
            {(['high', 'medium', 'low'] as const).map((severity) => {
              const issues = groupedIssues?.[severity];
              if (!issues || issues.length === 0) return null;
              return (
                <div key={severity} className="security-issue-group">
                  <div className="security-issue-group-header">
                    <span className={`security-issue-severity security-severity-${severity}`}>
                      {severityIcon(severity)}
                      {severity === 'high' ? '높음' : severity === 'medium' ? '보통' : '낮음'}
                    </span>
                    <span className="security-issue-count">{issues.length}</span>
                  </div>
                  {issues.map((issue, i) => (
                    <div key={i} className="security-issue-item">
                      <div className="security-issue-type">{issue.type}</div>
                      <div className="security-issue-message">{issue.message}</div>
                      {issue.file && (
                        <div className="security-issue-location">
                          {issue.file}{issue.line ? `:${issue.line}` : ''}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        </>
      )}

      {/* Initial state */}
      {!result && !loading && !error && (
        <div className="security-empty">
          <Shield size={32} style={{ color: 'var(--color-text-tertiary)', opacity: 0.5 }} />
          <p>보안 스캔 버튼을 눌러<br />프로젝트를 검사하세요.</p>
        </div>
      )}
    </div>
  );
};

export default SecurityPanel;
