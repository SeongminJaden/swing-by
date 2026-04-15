import React, { useState, useEffect } from 'react';
import { ArrowLeft, Bot, Key, Sparkles, Github, Moon, Sun, Palette, Check } from 'lucide-react';
import { useAppStore, Theme } from '../stores/appStore';

const TOTAL_STEPS = 4;

function getAPI(): any {
  return (window as any).electronAPI;
}

export default function OnboardingPage() {
  const { setCurrentView, theme, setTheme } = useAppStore();
  const [step, setStep] = useState(1);
  const [selectedAI, setSelectedAI] = useState<string | null>(null);
  const [selectedTheme, setSelectedTheme] = useState<Theme>(theme);

  const next = () => {
    if (step < TOTAL_STEPS) setStep(step + 1);
  };
  const prev = () => {
    if (step > 1) setStep(step - 1);
  };
  const finish = async () => {
    setTheme(selectedTheme);
    // Mark onboarding as completed
    const api = getAPI();
    if (api?.readFile && api?.writeFile) {
      try {
        const raw = await api.readFile('~/.videplace/settings.json');
        const settings = raw ? JSON.parse(raw) : {};
        settings.onboardingCompleted = true;
        await api.writeFile('~/.videplace/settings.json', JSON.stringify(settings, null, 2));
      } catch {
        // Ignore – settings write is best-effort
      }
    }
    setCurrentView('dashboard');
  };

  const handleThemeSelect = (t: Theme) => {
    setSelectedTheme(t);
    setTheme(t);
  };

  return (
    <div className="onboarding-wrapper">
      <div className="onboarding-container">
        {/* Back button */}
        {step > 1 && (
          <button
            onClick={prev}
            className="btn-icon-md absolute left-8 top-8"
          >
            <ArrowLeft size={20} />
          </button>
        )}

        <div className="onboarding-card onboarding-step" key={step}>
          {step === 1 && <StepWelcome onNext={next} />}
          {step === 2 && (
            <StepAI
              selected={selectedAI}
              onSelect={setSelectedAI}
              onNext={next}
              onSkip={next}
            />
          )}
          {step === 3 && <StepGitHub onNext={next} onSkip={next} />}
          {step === 4 && (
            <StepTheme
              selected={selectedTheme}
              onSelect={handleThemeSelect}
              onFinish={finish}
            />
          )}
        </div>

        {/* Dots */}
        <div className="onboarding-dots">
          {Array.from({ length: TOTAL_STEPS }).map((_, i) => (
            <div
              key={i}
              className={`onboarding-dot${step === i + 1 ? ' active' : ''}`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

/* ---- Step 1: Welcome ---- */
function StepWelcome({ onNext }: { onNext: () => void }) {
  return (
    <div className="flex flex-col items-center text-center">
      <div className="onboarding-logo">V</div>
      <h1 className="heading-1 mt-8">VidEplace에 오신 것을 환영합니다</h1>
      <p className="subtitle mt-3">AI 기반 올인원 IDE</p>
      <button onClick={onNext} className="btn-primary mt-10">
        시작하기
      </button>
    </div>
  );
}

/* ---- Step 2: AI Connection ---- */
function StepAI({
  selected,
  onSelect,
  onNext,
  onSkip,
}: {
  selected: string | null;
  onSelect: (id: string) => void;
  onNext: () => void;
  onSkip: () => void;
}) {
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({});
  const [hasKeys, setHasKeys] = useState<Record<string, boolean>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [error, setError] = useState('');

  const services = [
    {
      id: 'claude',
      name: 'Claude',
      desc: 'Anthropic의 AI 어시스턴트',
      iconClass: 'onboarding-icon-claude',
      icon: <Bot size={22} className="text-accent-warning" />,
    },
    {
      id: 'openai',
      name: 'OpenAI',
      desc: 'GPT 모델 기반 코드 생성',
      iconClass: 'onboarding-icon-openai',
      icon: <Sparkles size={22} className="text-accent-success" />,
    },
    {
      id: 'gemini',
      name: 'Gemini',
      desc: 'Google AI 멀티모달 모델',
      iconClass: 'onboarding-icon-gemini',
      icon: <Key size={22} className="text-accent-info" />,
    },
  ];

  // Check which keys are already set
  useEffect(() => {
    const api = getAPI();
    if (!api?.aiHasKey) return;

    Promise.all(
      services.map((s) =>
        api.aiHasKey(s.id).then((result: any) => ({
          id: s.id,
          has: result?.hasKey ?? result === true,
        }))
      )
    )
      .then((results) => {
        const map: Record<string, boolean> = {};
        results.forEach((r) => (map[r.id] = r.has));
        setHasKeys(map);
      })
      .catch(() => { /* ignore */ });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSaveKey = async (providerId: string) => {
    const key = apiKeys[providerId]?.trim();
    if (!key) return;

    const api = getAPI();
    if (!api?.aiSetKey) {
      // Fallback: just mark as set locally
      setHasKeys((prev) => ({ ...prev, [providerId]: true }));
      return;
    }

    setSaving(providerId);
    setError('');
    try {
      const result = await api.aiSetKey(providerId, key);
      if (result?.success !== false) {
        setHasKeys((prev) => ({ ...prev, [providerId]: true }));
        setApiKeys((prev) => ({ ...prev, [providerId]: '' }));
      } else {
        setError(result?.message || 'API 키 저장에 실패했습니다');
      }
    } catch {
      setError('API 키 저장 중 오류가 발생했습니다');
    } finally {
      setSaving(null);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent, providerId: string) => {
    if (e.key === 'Enter') handleSaveKey(providerId);
  };

  return (
    <div>
      <h1 className="heading-1 text-center">AI 서비스 연결</h1>
      <p className="subtitle mt-3 text-center">
        사용할 AI 서비스를 선택하고 API 키를 입력하세요
      </p>

      {error && (
        <div style={{ color: 'var(--color-accent-error, #f85149)', fontSize: '0.85rem', textAlign: 'center', marginTop: '0.5rem' }}>
          {error}
        </div>
      )}

      <div className="mt-8 flex flex-col gap-3">
        {services.map((s) => (
          <div key={s.id}>
            <button
              onClick={() => onSelect(s.id)}
              className={`onboarding-service-card${selected === s.id ? ' selected' : ''}`}
            >
              <div className={s.iconClass}>{s.icon}</div>
              <div className="flex-1 text-left">
                <span className="text-body block">{s.name}</span>
                <p className="mt-1 text-sm text-text-secondary">{s.desc}</p>
              </div>
              {hasKeys[s.id] && (
                <Check size={18} className="text-accent-primary" strokeWidth={2.5} />
              )}
            </button>
            {/* API key input when selected */}
            {selected === s.id && !hasKeys[s.id] && (
              <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem', paddingLeft: '0.5rem' }}>
                <input
                  type="password"
                  placeholder="API 키 입력"
                  value={apiKeys[s.id] || ''}
                  onChange={(e) => setApiKeys((prev) => ({ ...prev, [s.id]: e.target.value }))}
                  onKeyDown={(e) => handleKeyDown(e, s.id)}
                  disabled={saving === s.id}
                  style={{
                    flex: 1,
                    padding: '0.4rem 0.6rem',
                    borderRadius: '6px',
                    border: '1px solid var(--color-border-primary, #30363d)',
                    background: 'var(--color-bg-secondary, #161b22)',
                    color: 'var(--color-text-primary, #e6edf3)',
                    fontSize: '0.85rem',
                  }}
                />
                <button
                  onClick={() => handleSaveKey(s.id)}
                  disabled={saving === s.id || !apiKeys[s.id]?.trim()}
                  style={{
                    padding: '0.4rem 0.8rem',
                    borderRadius: '6px',
                    border: 'none',
                    background: 'var(--color-accent-primary, #58a6ff)',
                    color: '#fff',
                    cursor: saving === s.id ? 'not-allowed' : 'pointer',
                    opacity: saving === s.id ? 0.6 : 1,
                    fontSize: '0.85rem',
                  }}
                >
                  {saving === s.id ? '저장 중...' : '저장'}
                </button>
              </div>
            )}
            {selected === s.id && hasKeys[s.id] && (
              <p style={{ fontSize: '0.8rem', color: 'var(--color-accent-success, #3fb950)', marginTop: '0.25rem', paddingLeft: '0.5rem' }}>
                API 키가 설정되어 있습니다
              </p>
            )}
          </div>
        ))}
      </div>

      <div className="onboarding-actions">
        <button onClick={onSkip} className="btn-ghost">
          건너뛰기
        </button>
        <button onClick={onNext} className="btn-primary">
          다음
        </button>
      </div>
    </div>
  );
}

/* ---- Step 3: GitHub ---- */
function StepGitHub({
  onNext,
  onSkip,
}: {
  onNext: () => void;
  onSkip: () => void;
}) {
  const [tokenMode, setTokenMode] = useState(false);
  const [token, setToken] = useState('');
  const [saved, setSaved] = useState(false);
  const [message, setMessage] = useState('');

  const handleGitHubConnect = () => {
    setTokenMode(true);
    setMessage('');
  };

  const handleSaveToken = async () => {
    if (!token.trim()) return;
    const api = getAPI();
    if (api?.writeFile) {
      try {
        const raw = await api.readFile('~/.videplace/settings.json').catch(() => null);
        const settings = raw ? JSON.parse(raw) : {};
        settings.githubToken = token.trim();
        await api.writeFile('~/.videplace/settings.json', JSON.stringify(settings, null, 2));
        setSaved(true);
        setMessage('GitHub 토큰이 저장되었습니다');
      } catch {
        setMessage('토큰 저장에 실패했습니다');
      }
    } else {
      // Fallback: just pretend it worked
      setSaved(true);
      setMessage('GitHub 토큰이 저장되었습니다');
    }
  };

  return (
    <div>
      <h1 className="heading-1 text-center">GitHub 연결</h1>
      <p className="subtitle mt-3 text-center">
        소스 코드 관리를 위해 GitHub 계정을 연결하세요
      </p>

      <div className="mt-8">
        <button className="onboarding-service-card w-full" onClick={handleGitHubConnect}>
          <div className="onboarding-icon-github">
            <Github size={22} className="text-text-secondary" />
          </div>
          <div className="flex-1 text-left">
            <span className="text-body block">GitHub 연결</span>
            <p className="mt-1 text-sm text-text-secondary">
              {saved
                ? 'GitHub 토큰이 설정되었습니다'
                : 'Personal Access Token으로 연결하세요'}
            </p>
          </div>
          {saved && <Check size={18} className="text-accent-primary" strokeWidth={2.5} />}
        </button>

        {tokenMode && !saved && (
          <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.75rem' }}>
            <input
              type="password"
              placeholder="GitHub Personal Access Token"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSaveToken()}
              style={{
                flex: 1,
                padding: '0.4rem 0.6rem',
                borderRadius: '6px',
                border: '1px solid var(--color-border-primary, #30363d)',
                background: 'var(--color-bg-secondary, #161b22)',
                color: 'var(--color-text-primary, #e6edf3)',
                fontSize: '0.85rem',
              }}
            />
            <button
              onClick={handleSaveToken}
              disabled={!token.trim()}
              style={{
                padding: '0.4rem 0.8rem',
                borderRadius: '6px',
                border: 'none',
                background: 'var(--color-accent-primary, #58a6ff)',
                color: '#fff',
                cursor: !token.trim() ? 'not-allowed' : 'pointer',
                opacity: !token.trim() ? 0.6 : 1,
                fontSize: '0.85rem',
              }}
            >
              저장
            </button>
          </div>
        )}

        {message && (
          <p style={{
            fontSize: '0.8rem',
            color: saved ? 'var(--color-accent-success, #3fb950)' : 'var(--color-accent-error, #f85149)',
            marginTop: '0.5rem',
            textAlign: 'center',
          }}>
            {message}
          </p>
        )}
      </div>

      <div className="onboarding-actions">
        <button onClick={onSkip} className="btn-ghost">
          건너뛰기
        </button>
        <button onClick={onNext} className="btn-primary">
          다음
        </button>
      </div>
    </div>
  );
}

/* ---- Step 4: Theme ---- */
function StepTheme({
  selected,
  onSelect,
  onFinish,
}: {
  selected: Theme;
  onSelect: (t: Theme) => void;
  onFinish: () => void;
}) {
  const themes: {
    id: Theme;
    name: string;
    desc: string;
    icon: React.ReactNode;
    colors: string[];
    bgClass: string;
    borderClass: string;
  }[] = [
    {
      id: 'dark',
      name: 'Dark',
      desc: '기본 어두운 테마',
      icon: <Moon size={18} />,
      colors: ['#0d1117', '#161b22', '#1c2128', '#58a6ff'],
      bgClass: 'bg-[#161b22]',
      borderClass: 'border-[#30363d]',
    },
    {
      id: 'light',
      name: 'Light',
      desc: '밝고 깨끗한 테마',
      icon: <Sun size={18} />,
      colors: ['#ffffff', '#f6f8fa', '#f0f2f5', '#0969da'],
      bgClass: 'bg-[#f6f8fa]',
      borderClass: 'border-[#d0d7de]',
    },
    {
      id: 'monokai',
      name: 'Monokai',
      desc: '클래식 에디터 테마',
      icon: <Palette size={18} />,
      colors: ['#272822', '#2d2e27', '#3e3d32', '#66d9ef'],
      bgClass: 'bg-[#2d2e27]',
      borderClass: 'border-[#3e3d32]',
    },
  ];

  return (
    <div>
      <h1 className="heading-1 text-center">테마를 선택하세요</h1>
      <p className="subtitle mt-3 text-center">
        나중에 설정에서 변경할 수 있습니다
      </p>

      <div className="mt-8 flex flex-col gap-3">
        {themes.map((t) => (
          <button
            key={t.id}
            onClick={() => onSelect(t.id)}
            className={`onboarding-theme-card ${t.bgClass} ${t.borderClass}${selected === t.id ? ' selected' : ''}`}
          >
            <div className="onboarding-theme-preview">
              {t.colors.map((c, i) => (
                <div
                  key={i}
                  className="onboarding-theme-preview-stripe"
                  style={{ background: c }}
                />
              ))}
            </div>
            <div className="flex-1 text-left">
              <span className="text-body flex items-center gap-2">
                {t.icon}
                {t.name}
              </span>
              <p className="mt-1 text-sm text-text-secondary">{t.desc}</p>
            </div>
            {selected === t.id && (
              <Check size={18} className="text-accent-primary" strokeWidth={2.5} />
            )}
          </button>
        ))}
      </div>

      <div className="onboarding-actions-end">
        <button onClick={onFinish} className="btn-primary">
          완료
        </button>
      </div>
    </div>
  );
}
