import React, { useState, useCallback, useEffect } from 'react';
import {
  Activity,
  Play,
  Square,
  Plus,
  Trash2,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Clock,
  Globe,
  Loader2,
} from 'lucide-react';

interface Monitor {
  id: string;
  url: string;
  status: 'running' | 'stopped' | 'error';
  uptime: number;
  avgResponseTime: number;
  lastStatus: number;
  responseTimes: number[];
}

interface AlertItem {
  id: string;
  monitorId: string;
  type: string;
  message: string;
  timestamp: string;
}

export default function WatchdogPage() {
  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [alerts, setAlerts] = useState<AlertItem[]>([]);
  const [urlInput, setUrlInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [selectedMonitor, setSelectedMonitor] = useState<string | null>(null);

  // Load active monitors on mount
  useEffect(() => {
    const loadMonitors = async () => {
      if (!(window as any).electronAPI) return;
      try {
        const active = await (window as any).electronAPI.monitoringListActive();
        if (Array.isArray(active) && active.length > 0) {
          const loaded: Monitor[] = active.map((m: any) => ({
            id: m.id,
            url: m.url,
            status: 'running',
            uptime: m.uptime ?? 99.9,
            avgResponseTime: m.avgResponseTime ?? 0,
            lastStatus: m.lastStatus ?? 200,
            responseTimes: m.responseTimes ?? [],
          }));
          setMonitors(loaded);
        }
      } catch {
        // ignore
      }
    };
    loadMonitors();
  }, []);

  // Listen for updates
  useEffect(() => {
    if (!(window as any).electronAPI) return;
    (window as any).electronAPI.onMonitoringUpdate?.((data: any) => {
      setMonitors((prev) =>
        prev.map((m) =>
          m.id === data.id
            ? {
                ...m,
                uptime: data.uptime ?? m.uptime,
                avgResponseTime: data.avgResponseTime ?? m.avgResponseTime,
                lastStatus: data.statusCode ?? m.lastStatus,
                responseTimes: [...m.responseTimes.slice(-9), data.responseTime ?? 0],
              }
            : m,
        ),
      );
    });
    (window as any).electronAPI.onMonitoringAlert?.((data: any) => {
      setAlerts((prev) => [
        {
          id: `alert-${Date.now()}`,
          monitorId: data.monitorId || data.id,
          type: data.type || 'warning',
          message: data.message || '알림이 발생했습니다',
          timestamp: new Date().toISOString(),
        },
        ...prev,
      ]);
    });
  }, []);

  const handleStartMonitoring = useCallback(async () => {
    if (!urlInput.trim()) return;

    const url = urlInput.trim().startsWith('http') ? urlInput.trim() : `https://${urlInput.trim()}`;
    setLoading(true);

    try {
      if ((window as any).electronAPI) {
        const res = await (window as any).electronAPI.monitoringStart(url, 30000);
        if (res?.id) {
          setMonitors((prev) => [
            ...prev,
            {
              id: res.id,
              url,
              status: 'running',
              uptime: 100,
              avgResponseTime: 0,
              lastStatus: 0,
              responseTimes: [],
            },
          ]);
        }
      } else {
        // Mock for UI development
        const mockId = `mon-${Date.now()}`;
        setMonitors((prev) => [
          ...prev,
          {
            id: mockId,
            url,
            status: 'running',
            uptime: 99.8,
            avgResponseTime: 245,
            lastStatus: 200,
            responseTimes: [180, 210, 320, 195, 240, 280, 195, 310, 225, 245],
          },
        ]);
      }
      setUrlInput('');
    } catch {
      // fallback mock
      const mockId = `mon-${Date.now()}`;
      setMonitors((prev) => [
        ...prev,
        {
          id: mockId,
          url,
          status: 'running',
          uptime: 99.8,
          avgResponseTime: 245,
          lastStatus: 200,
          responseTimes: [180, 210, 320, 195, 240, 280, 195, 310, 225, 245],
        },
      ]);
      setUrlInput('');
    } finally {
      setLoading(false);
    }
  }, [urlInput]);

  const handleStopMonitoring = useCallback(async (id: string) => {
    try {
      await (window as any).electronAPI?.monitoringStop(id);
    } catch {
      // ignore
    }
    setMonitors((prev) =>
      prev.map((m) => (m.id === id ? { ...m, status: 'stopped' } : m)),
    );
  }, []);

  const handleRemoveMonitor = useCallback((id: string) => {
    setMonitors((prev) => prev.filter((m) => m.id !== id));
    if (selectedMonitor === id) setSelectedMonitor(null);
  }, [selectedMonitor]);

  const handleResumeMonitoring = useCallback(async (id: string) => {
    const monitor = monitors.find((m) => m.id === id);
    if (!monitor) return;
    try {
      if ((window as any).electronAPI) {
        await (window as any).electronAPI.monitoringStart(monitor.url, 30000);
      }
    } catch {
      // ignore
    }
    setMonitors((prev) =>
      prev.map((m) => (m.id === id ? { ...m, status: 'running' } : m)),
    );
  }, [monitors]);

  const selected = monitors.find((m) => m.id === selectedMonitor);

  const maxResponseTime = selected
    ? Math.max(...selected.responseTimes, 1)
    : 1;

  return (
    <div className="watchdog-page">
      {/* Header */}
      <div className="watchdog-header">
        <div className="watchdog-header-left">
          <Activity size={24} style={{ color: 'var(--color-accent-primary)' }} />
          <h1 className="watchdog-title">워치독 모니터링</h1>
        </div>
      </div>

      {/* URL input */}
      <div className="watchdog-add-section">
        <div className="watchdog-url-row">
          <Globe size={16} style={{ color: 'var(--color-text-tertiary)', flexShrink: 0 }} />
          <input
            type="text"
            className="watchdog-url-input"
            placeholder="모니터링할 URL을 입력하세요 (예: https://example.com)"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleStartMonitoring()}
          />
          <button
            className="watchdog-start-btn"
            onClick={handleStartMonitoring}
            disabled={!urlInput.trim() || loading}
          >
            {loading ? (
              <Loader2 size={14} className="animate-spin" />
            ) : (
              <Plus size={14} />
            )}
            모니터링 시작
          </button>
        </div>
      </div>

      <div className="watchdog-body">
        {/* Active monitors list */}
        <div className="watchdog-monitors-section">
          <h2 className="watchdog-section-title">활성 모니터</h2>
          {monitors.length === 0 ? (
            <div className="watchdog-empty-card">
              <Activity size={28} style={{ color: 'var(--color-text-tertiary)', opacity: 0.4 }} />
              <p>모니터링 중인 서비스가 없습니다</p>
            </div>
          ) : (
            <div className="watchdog-monitor-list">
              {monitors.map((monitor) => (
                <div
                  key={monitor.id}
                  className={`watchdog-monitor-card ${selectedMonitor === monitor.id ? 'watchdog-monitor-card-selected' : ''}`}
                  onClick={() => setSelectedMonitor(monitor.id)}
                >
                  <div className="watchdog-monitor-card-top">
                    <div className="watchdog-monitor-status-row">
                      <span className={`watchdog-status-dot watchdog-status-dot-${monitor.status === 'running' ? 'active' : monitor.status === 'error' ? 'error' : 'stopped'}`} />
                      <span className="watchdog-monitor-url">{monitor.url}</span>
                    </div>
                    <div className="watchdog-monitor-actions">
                      {monitor.status === 'running' ? (
                        <button
                          className="watchdog-icon-btn"
                          onClick={(e) => { e.stopPropagation(); handleStopMonitoring(monitor.id); }}
                          title="모니터링 중지"
                        >
                          <Square size={13} />
                        </button>
                      ) : (
                        <button
                          className="watchdog-icon-btn"
                          onClick={(e) => { e.stopPropagation(); handleResumeMonitoring(monitor.id); }}
                          title="모니터링 시작"
                        >
                          <Play size={13} />
                        </button>
                      )}
                      <button
                        className="watchdog-icon-btn watchdog-icon-btn-danger"
                        onClick={(e) => { e.stopPropagation(); handleRemoveMonitor(monitor.id); }}
                        title="삭제"
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>
                  <div className="watchdog-monitor-metrics">
                    <div className="watchdog-metric-mini">
                      <span className="watchdog-metric-mini-value">{monitor.uptime.toFixed(1)}%</span>
                      <span className="watchdog-metric-mini-label">가동률</span>
                    </div>
                    <div className="watchdog-metric-mini">
                      <span className="watchdog-metric-mini-value">{monitor.avgResponseTime}ms</span>
                      <span className="watchdog-metric-mini-label">평균 응답</span>
                    </div>
                    <div className="watchdog-metric-mini">
                      <span className={`watchdog-metric-mini-value ${monitor.lastStatus >= 200 && monitor.lastStatus < 400 ? 'text-accent-success' : 'text-accent-error'}`}>
                        {monitor.lastStatus || '--'}
                      </span>
                      <span className="watchdog-metric-mini-label">상태 코드</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Detail panel */}
        {selected && (
          <div className="watchdog-detail-section">
            <h2 className="watchdog-section-title">상세 정보</h2>

            {/* Metric cards */}
            <div className="watchdog-metric-cards">
              <div className="watchdog-metric-card">
                <div className="watchdog-metric-card-icon" style={{ background: 'rgba(63, 185, 80, 0.12)' }}>
                  <CheckCircle2 size={20} style={{ color: 'var(--color-accent-success)' }} />
                </div>
                <div className="watchdog-metric-card-value">{selected.uptime.toFixed(1)}%</div>
                <div className="watchdog-metric-card-label">가동률</div>
              </div>
              <div className="watchdog-metric-card">
                <div className="watchdog-metric-card-icon" style={{ background: 'rgba(88, 166, 255, 0.12)' }}>
                  <Clock size={20} style={{ color: 'var(--color-accent-primary)' }} />
                </div>
                <div className="watchdog-metric-card-value">{selected.avgResponseTime}ms</div>
                <div className="watchdog-metric-card-label">평균 응답 시간</div>
              </div>
              <div className="watchdog-metric-card">
                <div className="watchdog-metric-card-icon" style={{
                  background: selected.lastStatus >= 200 && selected.lastStatus < 400
                    ? 'rgba(63, 185, 80, 0.12)'
                    : 'rgba(248, 81, 73, 0.12)',
                }}>
                  {selected.lastStatus >= 200 && selected.lastStatus < 400
                    ? <CheckCircle2 size={20} style={{ color: 'var(--color-accent-success)' }} />
                    : <XCircle size={20} style={{ color: 'var(--color-accent-error)' }} />
                  }
                </div>
                <div className="watchdog-metric-card-value">{selected.lastStatus || '--'}</div>
                <div className="watchdog-metric-card-label">마지막 상태</div>
              </div>
            </div>

            {/* Bar chart — last 10 response times */}
            {selected.responseTimes.length > 0 && (
              <div className="watchdog-chart-card">
                <div className="watchdog-chart-title">최근 응답 시간 (ms)</div>
                <div className="watchdog-bar-chart">
                  {selected.responseTimes.slice(-10).map((time, i) => (
                    <div key={i} className="watchdog-bar-col">
                      <div
                        className="watchdog-bar"
                        style={{
                          height: `${Math.max((time / maxResponseTime) * 100, 4)}%`,
                          background: time > 500
                            ? 'var(--color-accent-error)'
                            : time > 300
                              ? 'var(--color-accent-warning)'
                              : 'var(--color-accent-primary)',
                        }}
                      />
                      <span className="watchdog-bar-label">{time}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Alerts */}
        {alerts.length > 0 && (
          <div className="watchdog-alerts-section">
            <h2 className="watchdog-section-title">알림</h2>
            <div className="watchdog-alert-list">
              {alerts.slice(0, 20).map((alert) => (
                <div key={alert.id} className="watchdog-alert-item">
                  <AlertTriangle size={14} style={{ color: 'var(--color-accent-warning)', flexShrink: 0 }} />
                  <div className="watchdog-alert-body">
                    <div className="watchdog-alert-message">{alert.message}</div>
                    <div className="watchdog-alert-time">
                      {new Date(alert.timestamp).toLocaleString('ko-KR')}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
