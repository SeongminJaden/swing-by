import { useState, useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { useAppStore } from '../stores/appStore';
import { useTranslation } from '../i18n';

type Mode = 'login' | 'register';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function getAPI(): any {
  return (window as any).electronAPI;
}

export default function LoginPage() {
  const { setUserLogin, setUserPlan, setCurrentView, userPlan } = useAppStore();
  const { t } = useTranslation();

  const [mode, setMode] = useState<Mode>('login');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [waitingForOAuth, setWaitingForOAuth] = useState(false);

  const validate = (): string | null => {
    if (!email) return t('login.enterEmail');
    if (!EMAIL_RE.test(email)) return t('login.invalidEmail');
    if (!password) return t('login.enterPassword');
    if (password.length < 6) return t('login.passwordMinLength');
    if (mode === 'register') {
      if (!name) return t('login.enterName');
      if (password !== confirmPassword) return t('login.passwordMismatch');
    }
    return null;
  };

  const navigateAfterLogin = (plan?: string | null) => {
    if (plan) {
      setCurrentView('dashboard');
    } else {
      setCurrentView('pricing');
    }
  };

  /** Fallback for browser dev mode (no electronAPI) */
  const fallbackLogin = (loginEmail: string) => {
    setUserLogin(loginEmail);
    navigateAfterLogin(userPlan);
  };

  // Listen for OAuth callback from the main process
  useEffect(() => {
    const api = getAPI();
    if (!api?.onAuthUserChanged) return;

    api.onAuthUserChanged((user: any) => {
      if (user) {
        setUserLogin(user.email);
        if (user.plan) setUserPlan(user.plan);
        setWaitingForOAuth(false);
        navigateAfterLogin(user.plan);
      }
    });
  }, []);

  const handleLogin = async () => {
    setError('');
    const validationError = validate();
    if (validationError) {
      setError(validationError);
      return;
    }

    const api = getAPI();
    if (!api?.authLogin) {
      fallbackLogin(email);
      return;
    }

    setLoading(true);
    try {
      const result = await api.authLogin(email, password);
      if (result?.success) {
        setUserLogin(result.user?.email || email);
        if (result.user?.plan) {
          setUserPlan(result.user.plan);
        }
        navigateAfterLogin(result.user?.plan);
      } else {
        setError(result?.message || t('login.invalidCredentials'));
      }
    } catch {
      setError(t('login.invalidCredentials'));
    } finally {
      setLoading(false);
    }
  };

  const handleRegister = async () => {
    setError('');
    const validationError = validate();
    if (validationError) {
      setError(validationError);
      return;
    }

    const api = getAPI();
    if (!api?.authRegister) {
      fallbackLogin(email);
      return;
    }

    setLoading(true);
    try {
      const regResult = await api.authRegister(email, password, name);
      if (!regResult?.success) {
        setError(regResult?.message || t('login.registerFailed'));
        return;
      }
      // Auto login after registration
      const loginResult = await api.authLogin(email, password);
      if (loginResult?.success) {
        setUserLogin(loginResult.user?.email || email);
        if (loginResult.user?.plan) {
          setUserPlan(loginResult.user.plan);
        }
        navigateAfterLogin(loginResult.user?.plan);
      } else {
        // Registration succeeded but auto-login failed: switch to login mode
        setMode('login');
        setError(t('login.registerSuccess'));
      }
    } catch {
      setError(t('login.registerError'));
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = () => {
    if (mode === 'login') handleLogin();
    else handleRegister();
  };

  const handleSocialLogin = async (provider: string) => {
    const api = getAPI();
    if (!api?.authSocialLogin) {
      if (!api?.authLogin) {
        // Fallback: mock social login in dev mode
        fallbackLogin(`${provider}@videplace.com`);
        return;
      }
      setError('소셜 로그인을 사용하려면 백엔드 설정이 필요합니다');
      return;
    }

    setLoading(true);
    setError('');
    try {
      const result = await api.authSocialLogin(provider);
      if (result?.success) {
        setUserLogin(result.user?.email || `${provider}@user`);
        if (result.user?.plan) {
          setUserPlan(result.user.plan);
        }
        navigateAfterLogin(result.user?.plan);
      } else if (result?.url) {
        // OAuth flow started - browser opened, waiting for callback
        // The auth:userChanged event will handle the rest
        setWaitingForOAuth(true);
        setError('');
      } else {
        setError(result?.message || '소셜 로그인에 실패했습니다');
      }
    } catch {
      setError('소셜 로그인 중 오류가 발생했습니다');
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSubmit();
  };

  const toggleMode = () => {
    setMode(mode === 'login' ? 'register' : 'login');
    setError('');
  };

  return (
    <div className="login-wrapper">
      <div className="login-container">
        {/* Logo area */}
        <div className="login-logo-area">
          <div className="login-logo">V</div>
          <div className="login-logo-title">VidEplace</div>
          <p className="login-tagline">{t('login.tagline')}</p>
        </div>

        {/* Login card */}
        <div className="login-card">
          <h2 className="login-heading">
            {mode === 'login' ? t('auth.login') : t('auth.register')}
          </h2>

          {error && (
            <div className="login-error" style={{ color: 'var(--color-accent-error, #f85149)', fontSize: '0.85rem', marginBottom: '0.75rem', textAlign: 'center' }}>
              {error}
            </div>
          )}

          {waitingForOAuth ? (
            <div style={{ textAlign: 'center', padding: '20px' }}>
              <Loader2 className="animate-spin" style={{ margin: '0 auto 12px', color: 'var(--color-accent-primary)' }} size={32} />
              <p style={{ fontSize: '14px', color: 'var(--color-text-secondary)' }}>
                브라우저에서 로그인을 완료해주세요
              </p>
              <p style={{ fontSize: '12px', color: 'var(--color-text-tertiary)', marginTop: '4px' }}>
                로그인 완료 후 자동으로 진행됩니다
              </p>
              <button
                className="login-footer-link"
                onClick={() => setWaitingForOAuth(false)}
                style={{ marginTop: '16px' }}
              >
                취소
              </button>
            </div>
          ) : (
            <>
              <div className="login-fields">
                {mode === 'register' && (
                  <input
                    type="text"
                    className="login-input"
                    placeholder={t('auth.name')}
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    onKeyDown={handleKeyDown}
                    disabled={loading}
                  />
                )}
                <input
                  type="email"
                  className="login-input"
                  placeholder={t('login.emailPlaceholder')}
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  onKeyDown={handleKeyDown}
                  disabled={loading}
                />
                <input
                  type="password"
                  className="login-input"
                  placeholder={t('login.passwordPlaceholder')}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  onKeyDown={handleKeyDown}
                  disabled={loading}
                />
                {mode === 'register' && (
                  <input
                    type="password"
                    className="login-input"
                    placeholder={t('login.confirmPassword')}
                    value={confirmPassword}
                    onChange={(e) => setConfirmPassword(e.target.value)}
                    onKeyDown={handleKeyDown}
                    disabled={loading}
                  />
                )}
              </div>

              <button
                className="login-btn-primary"
                onClick={handleSubmit}
                disabled={loading}
                style={loading ? { opacity: 0.6, cursor: 'not-allowed' } : undefined}
              >
                {loading
                  ? t('login.processing')
                  : mode === 'login'
                    ? t('auth.login')
                    : t('auth.register')}
              </button>

              {/* Divider */}
              <div className="login-divider">
                <span className="login-divider-line" />
                <span className="login-divider-text">{t('login.or')}</span>
                <span className="login-divider-line" />
              </div>

              {/* Social buttons */}
              <button className="login-btn-social" onClick={() => handleSocialLogin('google')} disabled={loading}>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 0 1-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z" fill="#4285F4"/>
                  <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
                  <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/>
                  <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/>
                </svg>
                {t('login.continueWithGoogle')}
              </button>

              <button className="login-btn-social" onClick={() => handleSocialLogin('github')} disabled={loading}>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 0C5.374 0 0 5.373 0 12c0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0 1 12 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z"/>
                </svg>
                {t('login.continueWithGithub')}
              </button>

              <button className="login-btn-social" onClick={() => handleSocialLogin('apple')} disabled={loading}>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M17.05 20.28c-.98.95-2.05.88-3.08.4-1.09-.5-2.08-.48-3.24 0-1.44.62-2.2.44-3.06-.4C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.53 4.09zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z"/>
                </svg>
                Apple로 계속하기
              </button>

              {/* Sign up / login toggle link */}
              <div className="login-footer">
                <span className="login-footer-text">
                  {mode === 'login' ? t('auth.noAccount') : t('auth.hasAccount')}
                </span>
                <button className="login-footer-link" onClick={toggleMode}>
                  {mode === 'login' ? t('auth.register') : t('auth.login')}
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
