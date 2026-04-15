import React, { useState, useRef, useCallback, useEffect } from 'react';
import {
  AlertTriangle,
  Globe,
  Loader2,
  Monitor,
  RefreshCw,
  Smartphone,
  Tablet,
} from 'lucide-react';
import IconButton from '../common/IconButton';

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

type Viewport = 'desktop' | 'tablet' | 'mobile';

interface PreviewPanelProps {
  className?: string;
}

/* ------------------------------------------------------------------ */
/*  Viewport width constraints                                         */
/* ------------------------------------------------------------------ */

const viewportWidths: Record<Viewport, number | null> = {
  desktop: null,   // full width
  tablet: 768,
  mobile: 375,
};

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */

export default function PreviewPanel({ className = '' }: PreviewPanelProps) {
  const [viewport, setViewport] = useState<Viewport>('desktop');
  const [url, setUrl] = useState('');
  const [inputUrl, setInputUrl] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [hasError, setHasError] = useState(false);
  const [iframeKey, setIframeKey] = useState(0);
  const [lastUpdated, setLastUpdated] = useState<string | null>(null);
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Navigate to URL
  const navigateTo = useCallback((targetUrl: string) => {
    let normalized = targetUrl.trim();
    if (!normalized) return;

    // Add protocol if missing
    if (!/^https?:\/\//i.test(normalized)) {
      normalized = 'http://' + normalized;
    }

    setUrl(normalized);
    setInputUrl(normalized);
    setIsLoading(true);
    setHasError(false);
    setIframeKey((k) => k + 1);
  }, []);

  // Refresh current URL
  const handleRefresh = useCallback(() => {
    setIsLoading(true);
    setHasError(false);
    setIframeKey((k) => k + 1);
  }, []);

  // Handle URL input submission
  const handleUrlSubmit = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        navigateTo(inputUrl);
        inputRef.current?.blur();
      }
    },
    [inputUrl, navigateTo],
  );

  // Handle URL input blur -- navigate on blur as well
  const handleUrlBlur = useCallback(() => {
    if (inputUrl.trim() && inputUrl.trim() !== url) {
      navigateTo(inputUrl);
    }
  }, [inputUrl, url, navigateTo]);

  // iframe load handler
  const handleIframeLoad = useCallback(() => {
    setIsLoading(false);
    setHasError(false);
    const now = new Date();
    setLastUpdated(
      `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}`,
    );
  }, []);

  // iframe error handler -- use a timeout to detect errors since iframe onerror doesn't always fire
  useEffect(() => {
    if (!isLoading) return;

    const timeout = setTimeout(() => {
      // If still loading after 10s, assume error
      if (isLoading) {
        setIsLoading(false);
        setHasError(true);
      }
    }, 10000);

    return () => clearTimeout(timeout);
  }, [isLoading, iframeKey]);

  const vpWidth = viewportWidths[viewport];

  return (
    <div className={`preview-panel ${className}`}>
      {/* ---- Header ---- */}
      <div className="preview-header">
        <Globe size={16} className="text-accent-primary shrink-0" />
        <span className="preview-header-title">미리보기</span>

        {/* URL bar */}
        <div className="preview-toolbar" style={{ cursor: 'text' }}>
          <input
            ref={inputRef}
            type="text"
            value={inputUrl}
            onChange={(e) => setInputUrl(e.target.value)}
            onKeyDown={handleUrlSubmit}
            onBlur={handleUrlBlur}
            className="w-full bg-transparent outline-none font-mono text-xs text-text-secondary placeholder-text-tertiary"
            placeholder="URL 입력 (예: http://localhost:3000)"
            spellCheck={false}
          />
        </div>

        {/* Actions */}
        <IconButton
          icon={RefreshCw}
          size="sm"
          tooltip="새로고침"
          onClick={handleRefresh}
          className={isLoading ? 'animate-spin' : ''}
        />

        <div className="preview-toolbar-divider" />

        <IconButton
          icon={Monitor}
          size="sm"
          tooltip="데스크톱"
          active={viewport === 'desktop'}
          onClick={() => setViewport('desktop')}
        />
        <IconButton
          icon={Tablet}
          size="sm"
          tooltip="태블릿"
          active={viewport === 'tablet'}
          onClick={() => setViewport('tablet')}
        />
        <IconButton
          icon={Smartphone}
          size="sm"
          tooltip="모바일"
          active={viewport === 'mobile'}
          onClick={() => setViewport('mobile')}
        />
      </div>

      {/* ---- Loading indicator ---- */}
      {isLoading && (
        <div className="h-0.5 w-full bg-bg-tertiary overflow-hidden shrink-0">
          <div className="h-full bg-accent-primary animate-pulse" style={{ width: '60%' }} />
        </div>
      )}

      {/* ---- Preview area ---- */}
      <div className="preview-content">
        {hasError ? (
          /* Error state */
          <div className="flex flex-col items-center justify-center h-full gap-4 text-center px-8">
            <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-accent-error/10">
              <AlertTriangle size={28} className="text-accent-error" />
            </div>
            <div>
              <p className="text-sm font-semibold text-text-primary mb-1">페이지를 불러올 수 없습니다</p>
              <p className="text-xs text-text-tertiary leading-relaxed max-w-xs">
                {url} 에 연결할 수 없습니다. 개발 서버가 실행 중인지 확인하세요.
              </p>
            </div>
            <button
              onClick={handleRefresh}
              className="flex items-center gap-2 px-4 py-2 text-xs font-medium rounded-lg bg-accent-primary text-white hover:opacity-90 transition-opacity"
            >
              <RefreshCw size={14} />
              다시 시도
            </button>
          </div>
        ) : (
          /* iframe viewport */
          <div
            className={`h-full bg-white transition-all duration-300 ease-in-out ${
              viewport !== 'desktop' ? 'preview-viewport-framed' : ''
            }`}
            style={{
              width: vpWidth ? `${vpWidth}px` : '100%',
              maxWidth: '100%',
            }}
          >
            {isLoading && (
              <div className="absolute inset-0 flex items-center justify-center bg-bg-tertiary/80 z-10">
                <div className="flex flex-col items-center gap-3">
                  <Loader2 size={32} className="text-accent-primary animate-spin" />
                  <span className="text-xs text-text-tertiary">로딩 중...</span>
                </div>
              </div>
            )}
            <iframe
              key={iframeKey}
              ref={iframeRef}
              src={url}
              title="Preview"
              className="w-full h-full border-0"
              onLoad={handleIframeLoad}
              onError={() => {
                setIsLoading(false);
                setHasError(true);
              }}
              sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
            />
          </div>
        )}
      </div>

      {/* ---- Status bar ---- */}
      <div className="preview-status-bar">
        <span className="flex items-center gap-2">
          {hasError ? (
            <>
              연결 상태: <span className="text-accent-error">연결 실패</span>
            </>
          ) : isLoading ? (
            <>
              <Loader2 size={12} className="animate-spin text-accent-primary" />
              <span className="text-accent-primary">로딩 중...</span>
            </>
          ) : (
            <>
              연결 상태: <span className="text-accent-success">활성</span>
            </>
          )}
        </span>
        <span className="flex items-center gap-3">
          {vpWidth && (
            <span className="text-text-tertiary">{vpWidth}px</span>
          )}
          {lastUpdated && <span>마지막 업데이트: {lastUpdated}</span>}
        </span>
      </div>
    </div>
  );
}
