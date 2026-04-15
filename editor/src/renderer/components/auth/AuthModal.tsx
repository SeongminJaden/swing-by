import { useState, useEffect, useRef, useCallback } from 'react';
import { X, Lock, CheckCircle, AlertCircle, Loader2, ExternalLink, Copy, Eye, EyeOff } from 'lucide-react';
import { useAppStore } from '../../stores/appStore';

// ─── Service Config Types ────────────────────────────────────────────────────

interface ServiceField {
  id: string;
  label: string;
  placeholder: string;
  type: 'password' | 'text';
  required: boolean;
}

interface GuideStep {
  step: number;
  text: string;
}

interface ServiceConfig {
  id: string;
  name: string;
  color: string;
  loginUrl: string;
  fields: ServiceField[];
  guide: GuideStep[];
}

// ─── Service Configs ─────────────────────────────────────────────────────────

const serviceConfigs: Record<string, ServiceConfig> = {
  claude: {
    id: 'claude',
    name: 'Claude (Anthropic)',
    color: '#d97706',
    loginUrl: 'https://console.anthropic.com/settings/keys',
    fields: [
      { id: 'apiKey', label: 'API Key', placeholder: 'sk-ant-api03-...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Anthropic Console에 로그인하세요' },
      { step: 2, text: 'Settings → API Keys로 이동하세요' },
      { step: 3, text: '"Create Key" 버튼을 클릭하세요' },
      { step: 4, text: '생성된 키를 복사하여 아래에 붙여넣으세요' },
    ],
  },
  openai: {
    id: 'openai',
    name: 'OpenAI',
    color: '#10a37f',
    loginUrl: 'https://platform.openai.com/api-keys',
    fields: [
      { id: 'apiKey', label: 'API Key', placeholder: 'sk-...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'OpenAI Platform에 로그인하세요' },
      { step: 2, text: 'API Keys 페이지로 이동하세요' },
      { step: 3, text: '"Create new secret key"를 클릭하세요' },
      { step: 4, text: '생성된 키를 복사하여 아래에 붙여넣으세요' },
    ],
  },
  gemini: {
    id: 'gemini',
    name: 'Google Gemini',
    color: '#4285f4',
    loginUrl: 'https://aistudio.google.com/app/apikey',
    fields: [
      { id: 'apiKey', label: 'API Key', placeholder: 'AI...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Google AI Studio에 로그인하세요' },
      { step: 2, text: 'API Keys 메뉴로 이동하세요' },
      { step: 3, text: '"Create API Key"를 클릭하세요' },
      { step: 4, text: '생성된 키를 복사하여 아래에 붙여넣으세요' },
    ],
  },
  ollama: {
    id: 'ollama',
    name: 'Ollama (로컬)',
    color: '#ffffff',
    loginUrl: 'https://ollama.com',
    fields: [
      { id: 'endpoint', label: 'Endpoint URL', placeholder: 'http://localhost:11434', type: 'text', required: true },
    ],
    guide: [
      { step: 1, text: 'Ollama를 로컬에 설치하세요' },
      { step: 2, text: '터미널에서 "ollama serve"를 실행하세요' },
      { step: 3, text: '기본 엔드포인트는 http://localhost:11434 입니다' },
      { step: 4, text: '엔드포인트 주소를 아래에 입력하세요' },
    ],
  },
  github: {
    id: 'github',
    name: 'GitHub',
    color: '#8b949e',
    loginUrl: 'https://github.com/settings/tokens/new',
    fields: [
      { id: 'token', label: 'Personal Access Token', placeholder: 'ghp_...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'GitHub에 로그인하세요' },
      { step: 2, text: 'Settings → Developer settings → Personal access tokens로 이동하세요' },
      { step: 3, text: '"Generate new token"을 클릭하세요' },
      { step: 4, text: '필요한 권한을 선택하고 토큰을 생성하세요' },
      { step: 5, text: '생성된 토큰을 복사하여 아래에 붙여넣으세요' },
    ],
  },
  gitlab: {
    id: 'gitlab',
    name: 'GitLab',
    color: '#fc6d26',
    loginUrl: 'https://gitlab.com/-/user_settings/personal_access_tokens',
    fields: [
      { id: 'token', label: 'Access Token', placeholder: 'glpat-...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'GitLab에 로그인하세요' },
      { step: 2, text: 'Preferences → Access Tokens로 이동하세요' },
      { step: 3, text: '토큰 이름과 만료일을 설정하세요' },
      { step: 4, text: '필요한 스코프를 선택하고 토큰을 생성하세요' },
      { step: 5, text: '생성된 토큰을 복사하여 아래에 붙여넣으세요' },
    ],
  },
  bitbucket: {
    id: 'bitbucket',
    name: 'Bitbucket',
    color: '#0052cc',
    loginUrl: 'https://bitbucket.org/account/settings/app-passwords/',
    fields: [
      { id: 'token', label: 'App Password', placeholder: '앱 비밀번호를 입력하세요', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Bitbucket에 로그인하세요' },
      { step: 2, text: 'Personal settings → App passwords로 이동하세요' },
      { step: 3, text: '"Create app password"를 클릭하세요' },
      { step: 4, text: '필요한 권한을 선택하고 비밀번호를 생성하세요' },
      { step: 5, text: '생성된 비밀번호를 복사하여 아래에 붙여넣으세요' },
    ],
  },
  vercel: {
    id: 'vercel',
    name: 'Vercel',
    color: '#ffffff',
    loginUrl: 'https://vercel.com/account/tokens',
    fields: [
      { id: 'token', label: 'API Token', placeholder: '토큰을 입력하세요', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Vercel에 로그인하세요' },
      { step: 2, text: 'Account Settings → Tokens로 이동하세요' },
      { step: 3, text: '"Create" 버튼을 클릭하세요' },
      { step: 4, text: '토큰 이름을 입력하고 생성하세요' },
      { step: 5, text: '생성된 토큰을 복사하여 아래에 붙여넣으세요' },
    ],
  },
  railway: {
    id: 'railway',
    name: 'Railway',
    color: '#a855f7',
    loginUrl: 'https://railway.app/account/tokens',
    fields: [
      { id: 'token', label: 'API Token', placeholder: '토큰을 입력하세요', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Railway에 로그인하세요' },
      { step: 2, text: 'Account Settings → Tokens로 이동하세요' },
      { step: 3, text: '"Create Token"을 클릭하세요' },
      { step: 4, text: '생성된 토큰을 복사하여 아래에 붙여넣으세요' },
    ],
  },
  netlify: {
    id: 'netlify',
    name: 'Netlify',
    color: '#00c7b7',
    loginUrl: 'https://app.netlify.com/user/applications#personal-access-tokens',
    fields: [
      { id: 'token', label: 'Personal Access Token', placeholder: '토큰을 입력하세요', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Netlify에 로그인하세요' },
      { step: 2, text: 'User Settings → Applications로 이동하세요' },
      { step: 3, text: 'Personal access tokens 섹션에서 "New access token"을 클릭하세요' },
      { step: 4, text: '생성된 토큰을 복사하여 아래에 붙여넣으세요' },
    ],
  },
  cloudflare: {
    id: 'cloudflare',
    name: 'Cloudflare',
    color: '#f38020',
    loginUrl: 'https://dash.cloudflare.com/profile/api-tokens',
    fields: [
      { id: 'apiToken', label: 'API Token', placeholder: '토큰을 입력하세요', type: 'password', required: true },
      { id: 'accountId', label: 'Account ID', placeholder: '계정 ID를 입력하세요', type: 'text', required: true },
    ],
    guide: [
      { step: 1, text: 'Cloudflare Dashboard에 로그인하세요' },
      { step: 2, text: 'Profile → API Tokens로 이동하세요' },
      { step: 3, text: '"Create Token"을 클릭하세요' },
      { step: 4, text: 'Account ID는 대시보드 우측 사이드바에서 확인하세요' },
      { step: 5, text: '토큰과 Account ID를 아래에 입력하세요' },
    ],
  },
  stripe: {
    id: 'stripe',
    name: 'Stripe',
    color: '#635bff',
    loginUrl: 'https://dashboard.stripe.com/apikeys',
    fields: [
      { id: 'secretKey', label: 'Secret Key', placeholder: 'sk_live_... 또는 sk_test_...', type: 'password', required: true },
      { id: 'publishableKey', label: 'Publishable Key', placeholder: 'pk_live_... 또는 pk_test_...', type: 'text', required: false },
    ],
    guide: [
      { step: 1, text: 'Stripe Dashboard에 로그인하세요' },
      { step: 2, text: 'Developers → API Keys로 이동하세요' },
      { step: 3, text: 'Secret key 옆의 "Reveal test key"를 클릭하세요' },
      { step: 4, text: 'Secret Key와 Publishable Key를 아래에 입력하세요' },
    ],
  },
  supabase: {
    id: 'supabase',
    name: 'Supabase',
    color: '#3ecf8e',
    loginUrl: 'https://supabase.com/dashboard/project/_/settings/api',
    fields: [
      { id: 'projectUrl', label: 'Project URL', placeholder: 'https://xxxxx.supabase.co', type: 'text', required: true },
      { id: 'anonKey', label: 'Anon (Public) Key', placeholder: 'eyJ...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Supabase Dashboard에 로그인하세요' },
      { step: 2, text: '프로젝트를 선택하세요' },
      { step: 3, text: 'Project Settings → API로 이동하세요' },
      { step: 4, text: 'Project URL과 anon key를 아래에 입력하세요' },
    ],
  },
  firebase: {
    id: 'firebase',
    name: 'Firebase',
    color: '#ffca28',
    loginUrl: 'https://console.firebase.google.com/project/_/settings/general',
    fields: [
      { id: 'projectId', label: 'Project ID', placeholder: 'my-project-id', type: 'text', required: true },
      { id: 'apiKey', label: 'Web API Key', placeholder: 'AIza...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Firebase Console에 로그인하세요' },
      { step: 2, text: '프로젝트를 선택하세요' },
      { step: 3, text: 'Project Settings → General로 이동하세요' },
      { step: 4, text: 'Project ID와 Web API Key를 아래에 입력하세요' },
    ],
  },
  slack: {
    id: 'slack',
    name: 'Slack',
    color: '#e01e5a',
    loginUrl: 'https://api.slack.com/apps',
    fields: [
      { id: 'botToken', label: 'Bot Token', placeholder: 'xoxb-...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'api.slack.com/apps에 접속하세요' },
      { step: 2, text: '앱을 선택하거나 새로 생성하세요' },
      { step: 3, text: 'OAuth & Permissions에서 Bot Token을 확인하세요' },
      { step: 4, text: 'Bot User OAuth Token을 복사하여 아래에 붙여넣으세요' },
    ],
  },
  discord: {
    id: 'discord',
    name: 'Discord',
    color: '#5865f2',
    loginUrl: 'https://discord.com/developers/applications',
    fields: [
      { id: 'botToken', label: 'Bot Token', placeholder: '봇 토큰을 입력하세요', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Discord Developer Portal에 접속하세요' },
      { step: 2, text: '애플리케이션을 선택하거나 새로 생성하세요' },
      { step: 3, text: 'Bot 메뉴에서 "Reset Token"을 클릭하세요' },
      { step: 4, text: '생성된 토큰을 복사하여 아래에 붙여넣으세요' },
    ],
  },
  'apple-developer': {
    id: 'apple-developer',
    name: 'Apple Developer',
    color: '#000000',
    loginUrl: 'https://appstoreconnect.apple.com/access/integrations/api',
    fields: [
      { id: 'keyId', label: 'Key ID', placeholder: 'XXXXXXXXXX', type: 'text', required: true },
      { id: 'issuerId', label: 'Issuer ID', placeholder: 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx', type: 'text', required: true },
      { id: 'privateKey', label: 'Private Key (.p8)', placeholder: '-----BEGIN PRIVATE KEY-----\n...', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'App Store Connect에 로그인하세요' },
      { step: 2, text: 'Users and Access → Integrations → App Store Connect API로 이동하세요' },
      { step: 3, text: '"Generate API Key"를 클릭하세요' },
      { step: 4, text: 'Key ID, Issuer ID를 확인하고 .p8 키를 다운로드하세요' },
      { step: 5, text: '정보를 아래에 입력하세요' },
    ],
  },
  'google-developer': {
    id: 'google-developer',
    name: 'Google Cloud',
    color: '#34a853',
    loginUrl: 'https://console.cloud.google.com/iam-admin/serviceaccounts',
    fields: [
      { id: 'serviceAccountJson', label: 'Service Account JSON', placeholder: '{"type": "service_account", ...}', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'Google Cloud Console에 로그인하세요' },
      { step: 2, text: 'IAM & Admin → Service Accounts로 이동하세요' },
      { step: 3, text: '서비스 계정을 선택하거나 새로 생성하세요' },
      { step: 4, text: 'Keys 탭에서 "Add Key" → "Create new key" (JSON)를 선택하세요' },
      { step: 5, text: '다운로드된 JSON 내용을 아래에 붙여넣으세요' },
    ],
  },
  aws: {
    id: 'aws',
    name: 'AWS',
    color: '#ff9900',
    loginUrl: 'https://console.aws.amazon.com/iam/home#/security_credentials',
    fields: [
      { id: 'accessKeyId', label: 'Access Key ID', placeholder: 'AKIA...', type: 'text', required: true },
      { id: 'secretAccessKey', label: 'Secret Access Key', placeholder: '시크릿 키를 입력하세요', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: 'AWS Management Console에 로그인하세요' },
      { step: 2, text: 'IAM → Users → Security credentials로 이동하세요' },
      { step: 3, text: '"Create access key"를 클릭하세요' },
      { step: 4, text: 'Access Key ID와 Secret Access Key를 아래에 입력하세요' },
    ],
  },
  tosspay: {
    id: 'tosspay',
    name: '토스페이먼츠',
    color: '#0064ff',
    loginUrl: 'https://developers.tosspayments.com/my/api-keys',
    fields: [
      { id: 'secretKey', label: '시크릿 키', placeholder: 'test_sk_... 또는 live_sk_...', type: 'password', required: true },
      { id: 'clientKey', label: '클라이언트 키', placeholder: 'test_ck_... 또는 live_ck_...', type: 'text', required: true },
    ],
    guide: [
      { step: 1, text: '토스페이먼츠 개발자센터에 로그인하세요' },
      { step: 2, text: '내 개발정보 → API 키로 이동하세요' },
      { step: 3, text: '테스트 또는 라이브 키를 확인하세요' },
      { step: 4, text: '시크릿 키와 클라이언트 키를 아래에 입력하세요' },
    ],
  },
};

function genericConfig(serviceId: string): ServiceConfig {
  const name = serviceId
    .split('-')
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join(' ');
  return {
    id: serviceId,
    name,
    color: '#58a6ff',
    loginUrl: `https://${serviceId.replace(/\s+/g, '')}.com`,
    fields: [
      { id: 'token', label: 'API Token / Key', placeholder: '인증 토큰을 입력하세요', type: 'password', required: true },
    ],
    guide: [
      { step: 1, text: `${name} 웹사이트에 로그인하세요` },
      { step: 2, text: '설정 또는 개발자 메뉴로 이동하세요' },
      { step: 3, text: 'API Key 또는 Token을 생성하세요' },
      { step: 4, text: '생성된 키를 복사하여 아래에 붙여넣으세요' },
    ],
  };
}

// ─── Styles ──────────────────────────────────────────────────────────────────

const colors = {
  bgPrimary: '#0d1117',
  bgSecondary: '#161b22',
  bgTertiary: '#21262d',
  textPrimary: '#e6edf3',
  textSecondary: '#8b949e',
  textTertiary: '#484f58',
  accentPrimary: '#58a6ff',
  accentSuccess: '#3fb950',
  accentError: '#f85149',
  border: '#30363d',
} as const;

// ─── PasswordField Component ─────────────────────────────────────────────────

function PasswordField({
  field,
  value,
  onChange,
}: {
  field: ServiceField;
  value: string;
  onChange: (val: string) => void;
}) {
  const [visible, setVisible] = useState(false);
  const isPassword = field.type === 'password';
  const isMultiline = field.id === 'privateKey' || field.id === 'serviceAccountJson';

  if (isMultiline) {
    return (
      <div style={{ marginBottom: 12 }}>
        <label
          style={{
            display: 'block',
            fontSize: 12,
            fontWeight: 500,
            color: colors.textSecondary,
            marginBottom: 6,
          }}
        >
          {field.label}
          {field.required && <span style={{ color: colors.accentError, marginLeft: 2 }}>*</span>}
        </label>
        <textarea
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.placeholder}
          rows={4}
          style={{
            width: '100%',
            padding: '10px 12px',
            backgroundColor: colors.bgPrimary,
            border: `1px solid ${colors.border}`,
            borderRadius: 10,
            color: colors.textPrimary,
            fontSize: 12,
            fontFamily: 'monospace',
            resize: 'vertical',
            outline: 'none',
            transition: 'border-color 0.15s ease',
          }}
          onFocus={(e) => { e.currentTarget.style.borderColor = colors.accentPrimary; }}
          onBlur={(e) => { e.currentTarget.style.borderColor = colors.border; }}
        />
      </div>
    );
  }

  return (
    <div style={{ marginBottom: 12 }}>
      <label
        style={{
          display: 'block',
          fontSize: 12,
          fontWeight: 500,
          color: colors.textSecondary,
          marginBottom: 6,
        }}
      >
        {field.label}
        {field.required && <span style={{ color: colors.accentError, marginLeft: 2 }}>*</span>}
      </label>
      <div style={{ position: 'relative' }}>
        <input
          type={isPassword && !visible ? 'password' : 'text'}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.placeholder}
          style={{
            width: '100%',
            padding: '10px 12px',
            paddingRight: isPassword ? 38 : 12,
            backgroundColor: colors.bgPrimary,
            border: `1px solid ${colors.border}`,
            borderRadius: 10,
            color: colors.textPrimary,
            fontSize: 13,
            fontFamily: isPassword ? 'monospace' : 'inherit',
            outline: 'none',
            transition: 'border-color 0.15s ease',
          }}
          onFocus={(e) => { e.currentTarget.style.borderColor = colors.accentPrimary; }}
          onBlur={(e) => { e.currentTarget.style.borderColor = colors.border; }}
        />
        {isPassword && (
          <button
            type="button"
            onClick={() => setVisible(!visible)}
            style={{
              position: 'absolute',
              right: 8,
              top: '50%',
              transform: 'translateY(-50%)',
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              padding: 4,
              color: colors.textTertiary,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            {visible ? <EyeOff size={14} /> : <Eye size={14} />}
          </button>
        )}
      </div>
    </div>
  );
}

// ─── Main Component ──────────────────────────────────────────────────────────

export function AuthModal() {
  const { authModalOpen, authModalService, closeAuthModal } = useAppStore();

  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [status, setStatus] = useState<'idle' | 'verifying' | 'success' | 'error'>('idle');
  const [error, setError] = useState('');
  const [webviewUrl, setWebviewUrl] = useState('');
  const [activeStep, setActiveStep] = useState(0);

  const webviewRef = useRef<HTMLWebViewElement | null>(null);
  const webviewContainerRef = useRef<HTMLDivElement | null>(null);

  const config = authModalService
    ? serviceConfigs[authModalService] || genericConfig(authModalService)
    : null;

  // Reset state when modal opens with a new service
  useEffect(() => {
    if (authModalOpen && authModalService) {
      setCredentials({});
      setStatus('idle');
      setError('');
      setWebviewUrl('');
      setActiveStep(0);
    }
  }, [authModalOpen, authModalService]);

  // Track webview navigation
  useEffect(() => {
    if (!authModalOpen || !config) return;

    const checkWebview = () => {
      const container = webviewContainerRef.current;
      if (!container) return;

      const wv = container.querySelector('webview') as HTMLWebViewElement | null;
      if (!wv || wv === webviewRef.current) return;

      webviewRef.current = wv;

      const handleNavigate = (e: any) => {
        setWebviewUrl(e.url || '');
      };

      const handleNewWindow = (e: any) => {
        // Redirect new-window events into the same webview
        if (e.url && wv) {
          (wv as any).loadURL(e.url);
        }
      };

      wv.addEventListener('did-navigate', handleNavigate);
      wv.addEventListener('did-navigate-in-page', handleNavigate);
      wv.addEventListener('new-window', handleNewWindow);

      return () => {
        wv.removeEventListener('did-navigate', handleNavigate);
        wv.removeEventListener('did-navigate-in-page', handleNavigate);
        wv.removeEventListener('new-window', handleNewWindow);
      };
    };

    // Small delay to allow webview to mount in DOM
    const timer = setTimeout(checkWebview, 300);
    return () => clearTimeout(timer);
  }, [authModalOpen, config]);

  const handleFieldChange = useCallback((fieldId: string, value: string) => {
    setCredentials((prev) => ({ ...prev, [fieldId]: value }));
    if (status === 'error') {
      setStatus('idle');
      setError('');
    }
  }, [status]);

  const handleConnect = useCallback(async () => {
    if (!config) return;

    // Validate required fields
    const missingFields = config.fields
      .filter((f) => f.required && !credentials[f.id]?.trim())
      .map((f) => f.label);

    if (missingFields.length > 0) {
      setStatus('error');
      setError(`필수 항목을 입력하세요: ${missingFields.join(', ')}`);
      return;
    }

    setStatus('verifying');
    setError('');

    try {
      const api = (window as any).electronAPI;

      if (api?.connectionsVerify) {
        const result = await api.connectionsVerify(config.id, credentials);
        if (result?.success) {
          await api.connectionsSave(config.id, credentials);
          setStatus('success');
          setTimeout(() => {
            closeAuthModal();
          }, 1500);
        } else {
          setStatus('error');
          setError(result?.error || '인증에 실패했습니다. 입력 정보를 확인해주세요.');
        }
      } else {
        // Fallback: save directly if verify API is not available
        if (api?.connectionsSave) {
          await api.connectionsSave(config.id, credentials);
        }
        setStatus('success');
        setTimeout(() => {
          closeAuthModal();
        }, 1500);
      }
    } catch (err: any) {
      setStatus('error');
      setError(err?.message || '연동 중 오류가 발생했습니다.');
    }
  }, [config, credentials, closeAuthModal]);

  const handleCopyUrl = useCallback(() => {
    const url = webviewUrl || config?.loginUrl || '';
    navigator.clipboard.writeText(url).catch(() => {});
  }, [webviewUrl, config]);

  const handleOpenExternal = useCallback(() => {
    const url = webviewUrl || config?.loginUrl || '';
    const api = (window as any).electronAPI;
    if (api?.openExternal) {
      api.openExternal(url);
    } else {
      window.open(url, '_blank');
    }
  }, [webviewUrl, config]);

  // Keyboard shortcuts
  useEffect(() => {
    if (!authModalOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeAuthModal();
      } else if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        if (status !== 'verifying' && status !== 'success') {
          handleConnect();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [authModalOpen, closeAuthModal, handleConnect, status]);

  if (!authModalOpen || !authModalService || !config) return null;

  const allRequiredFilled = config.fields
    .filter((f) => f.required)
    .every((f) => credentials[f.id]?.trim());

  const displayUrl = webviewUrl || config.loginUrl;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 50,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      {/* Backdrop */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          backgroundColor: 'rgba(0, 0, 0, 0.6)',
          backdropFilter: 'blur(4px)',
        }}
        onClick={closeAuthModal}
      />

      {/* Modal */}
      <div
        style={{
          position: 'relative',
          width: 1100,
          height: 720,
          backgroundColor: colors.bgSecondary,
          borderRadius: 16,
          border: `1px solid ${colors.border}`,
          boxShadow: '0 25px 60px rgba(0, 0, 0, 0.5)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          animation: 'authModalIn 0.2s ease-out',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '0 20px',
            height: 52,
            borderBottom: `1px solid ${colors.border}`,
            flexShrink: 0,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div
              style={{
                width: 10,
                height: 10,
                borderRadius: '50%',
                backgroundColor: config.color === '#000000' ? '#666' : config.color,
                boxShadow: `0 0 8px ${config.color === '#000000' ? '#66666644' : config.color + '44'}`,
              }}
            />
            <span
              style={{
                fontSize: 14,
                fontWeight: 600,
                color: colors.textPrimary,
              }}
            >
              {config.name} 연동
            </span>
          </div>
          <button
            onClick={closeAuthModal}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: 32,
              height: 32,
              borderRadius: 8,
              border: 'none',
              backgroundColor: 'transparent',
              color: colors.textTertiary,
              cursor: 'pointer',
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = colors.bgTertiary;
              e.currentTarget.style.color = colors.textPrimary;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = 'transparent';
              e.currentTarget.style.color = colors.textTertiary;
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Body: Two-panel layout */}
        <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
          {/* Left Panel: Guide + Inputs */}
          <div
            style={{
              width: 400,
              display: 'flex',
              flexDirection: 'column',
              borderRight: `1px solid ${colors.border}`,
              backgroundColor: colors.bgSecondary,
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                flex: 1,
                overflowY: 'auto',
                padding: '20px 24px',
              }}
            >
              {/* Guide Section */}
              <div style={{ marginBottom: 24 }}>
                <div
                  style={{
                    fontSize: 12,
                    fontWeight: 600,
                    color: colors.textSecondary,
                    textTransform: 'uppercase',
                    letterSpacing: '0.5px',
                    marginBottom: 14,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                  }}
                >
                  <span style={{ fontSize: 14 }}>&#128203;</span>
                  연동 가이드
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                  {config.guide.map((step) => (
                    <div
                      key={step.step}
                      onClick={() => setActiveStep(step.step)}
                      style={{
                        display: 'flex',
                        alignItems: 'flex-start',
                        gap: 10,
                        padding: '10px 12px',
                        borderRadius: 10,
                        backgroundColor:
                          activeStep === step.step
                            ? `${colors.accentPrimary}15`
                            : 'transparent',
                        cursor: 'pointer',
                        transition: 'background-color 0.15s ease',
                      }}
                      onMouseEnter={(e) => {
                        if (activeStep !== step.step) {
                          e.currentTarget.style.backgroundColor = `${colors.bgTertiary}`;
                        }
                      }}
                      onMouseLeave={(e) => {
                        if (activeStep !== step.step) {
                          e.currentTarget.style.backgroundColor = 'transparent';
                        }
                      }}
                    >
                      <div
                        style={{
                          width: 22,
                          height: 22,
                          borderRadius: '50%',
                          backgroundColor:
                            activeStep === step.step
                              ? colors.accentPrimary
                              : colors.bgTertiary,
                          color:
                            activeStep === step.step
                              ? '#ffffff'
                              : colors.textSecondary,
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          fontSize: 11,
                          fontWeight: 600,
                          flexShrink: 0,
                          transition: 'all 0.15s ease',
                        }}
                      >
                        {step.step}
                      </div>
                      <span
                        style={{
                          fontSize: 13,
                          color:
                            activeStep === step.step
                              ? colors.textPrimary
                              : colors.textSecondary,
                          lineHeight: '22px',
                          transition: 'color 0.15s ease',
                        }}
                      >
                        {step.text}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Divider */}
              <div
                style={{
                  height: 1,
                  backgroundColor: colors.border,
                  margin: '0 0 20px 0',
                }}
              />

              {/* Credential Fields */}
              <div style={{ marginBottom: 20 }}>
                <div
                  style={{
                    fontSize: 12,
                    fontWeight: 600,
                    color: colors.textSecondary,
                    textTransform: 'uppercase',
                    letterSpacing: '0.5px',
                    marginBottom: 14,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                  }}
                >
                  <Lock size={13} />
                  인증 정보
                </div>

                {config.fields.map((field) => (
                  <PasswordField
                    key={field.id}
                    field={field}
                    value={credentials[field.id] || ''}
                    onChange={(val) => handleFieldChange(field.id, val)}
                  />
                ))}
              </div>

              {/* Error message */}
              {status === 'error' && error && (
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: 8,
                    padding: '10px 14px',
                    borderRadius: 10,
                    backgroundColor: 'rgba(248, 81, 73, 0.1)',
                    marginBottom: 16,
                  }}
                >
                  <AlertCircle
                    size={14}
                    style={{ color: colors.accentError, flexShrink: 0, marginTop: 1 }}
                  />
                  <span style={{ fontSize: 12, color: colors.accentError, lineHeight: 1.5 }}>
                    {error}
                  </span>
                </div>
              )}

              {/* Success message */}
              {status === 'success' && (
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    padding: '10px 14px',
                    borderRadius: 10,
                    backgroundColor: 'rgba(63, 185, 80, 0.1)',
                    marginBottom: 16,
                  }}
                >
                  <CheckCircle
                    size={14}
                    style={{ color: colors.accentSuccess, flexShrink: 0 }}
                  />
                  <span style={{ fontSize: 12, color: colors.accentSuccess }}>
                    연동이 완료되었습니다!
                  </span>
                </div>
              )}

              {/* Action Buttons */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {/* Verify & Connect button */}
                {status !== 'success' && (
                  <button
                    onClick={handleConnect}
                    disabled={!allRequiredFilled || status === 'verifying'}
                    style={{
                      width: '100%',
                      height: 42,
                      borderRadius: 10,
                      border: 'none',
                      backgroundColor:
                        !allRequiredFilled || status === 'verifying'
                          ? colors.bgTertiary
                          : colors.accentPrimary,
                      color:
                        !allRequiredFilled || status === 'verifying'
                          ? colors.textTertiary
                          : '#ffffff',
                      fontSize: 13,
                      fontWeight: 600,
                      cursor:
                        !allRequiredFilled || status === 'verifying'
                          ? 'not-allowed'
                          : 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      gap: 8,
                      transition: 'all 0.2s ease',
                    }}
                    onMouseEnter={(e) => {
                      if (allRequiredFilled && status !== 'verifying') {
                        e.currentTarget.style.filter = 'brightness(1.1)';
                      }
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.filter = 'brightness(1)';
                    }}
                  >
                    {status === 'verifying' ? (
                      <>
                        <Loader2 size={14} style={{ animation: 'spin 1s linear infinite' }} />
                        연동 확인 중...
                      </>
                    ) : (
                      <>&#128260; 연동 확인</>
                    )}
                  </button>
                )}

                {/* Success state button */}
                {status === 'success' && (
                  <button
                    disabled
                    style={{
                      width: '100%',
                      height: 42,
                      borderRadius: 10,
                      border: 'none',
                      backgroundColor: colors.accentSuccess,
                      color: '#ffffff',
                      fontSize: 13,
                      fontWeight: 600,
                      cursor: 'default',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      gap: 8,
                    }}
                  >
                    <CheckCircle size={14} />
                    연동 완료
                  </button>
                )}
              </div>

              {/* Shortcut hint */}
              <div
                style={{
                  marginTop: 12,
                  textAlign: 'center',
                  fontSize: 11,
                  color: colors.textTertiary,
                }}
              >
                <kbd
                  style={{
                    padding: '2px 6px',
                    borderRadius: 4,
                    backgroundColor: colors.bgTertiary,
                    border: `1px solid ${colors.border}`,
                    fontSize: 10,
                    fontFamily: 'monospace',
                  }}
                >
                  {navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+Enter
                </kbd>{' '}
                로 빠르게 연동
              </div>
            </div>
          </div>

          {/* Right Panel: Webview */}
          <div
            style={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              backgroundColor: colors.bgPrimary,
              overflow: 'hidden',
            }}
          >
            {/* URL Bar */}
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '8px 12px',
                backgroundColor: colors.bgTertiary,
                borderBottom: `1px solid ${colors.border}`,
                flexShrink: 0,
              }}
            >
              <Lock size={12} style={{ color: colors.accentSuccess, flexShrink: 0 }} />
              <div
                style={{
                  flex: 1,
                  fontSize: 12,
                  fontFamily: 'monospace',
                  color: colors.textSecondary,
                  whiteSpace: 'nowrap',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  padding: '4px 10px',
                  backgroundColor: colors.bgPrimary,
                  borderRadius: 6,
                  border: `1px solid ${colors.border}`,
                }}
                title={displayUrl}
              >
                {displayUrl}
              </div>
              <button
                onClick={handleCopyUrl}
                title="URL 복사"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: 28,
                  height: 28,
                  borderRadius: 6,
                  border: 'none',
                  backgroundColor: 'transparent',
                  color: colors.textTertiary,
                  cursor: 'pointer',
                  flexShrink: 0,
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.backgroundColor = colors.bgSecondary;
                  e.currentTarget.style.color = colors.textSecondary;
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.backgroundColor = 'transparent';
                  e.currentTarget.style.color = colors.textTertiary;
                }}
              >
                <Copy size={13} />
              </button>
              <button
                onClick={handleOpenExternal}
                title="외부 브라우저에서 열기"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: 28,
                  height: 28,
                  borderRadius: 6,
                  border: 'none',
                  backgroundColor: 'transparent',
                  color: colors.textTertiary,
                  cursor: 'pointer',
                  flexShrink: 0,
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.backgroundColor = colors.bgSecondary;
                  e.currentTarget.style.color = colors.textSecondary;
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.backgroundColor = 'transparent';
                  e.currentTarget.style.color = colors.textTertiary;
                }}
              >
                <ExternalLink size={13} />
              </button>
            </div>

            {/* Webview Container */}
            <div
              ref={webviewContainerRef}
              style={{ flex: 1, overflow: 'hidden' }}
              dangerouslySetInnerHTML={{
                __html: `<webview
                  src="${config.loginUrl}"
                  style="width: 100%; height: 100%;"
                  partition="persist:service-auth"
                  allowpopups="true"
                ></webview>`,
              }}
            />
          </div>
        </div>

        {/* Footer */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '8px 20px',
            borderTop: `1px solid ${colors.border}`,
            backgroundColor: colors.bgPrimary,
            flexShrink: 0,
          }}
        >
          <Lock size={11} style={{ color: colors.textTertiary }} />
          <span style={{ fontSize: 11, color: colors.textTertiary }}>
            모든 인증 정보는 암호화되어 안전하게 저장됩니다
          </span>
        </div>
      </div>

      {/* Keyframe animation */}
      <style>{`
        @keyframes authModalIn {
          from {
            opacity: 0;
            transform: scale(0.96) translateY(8px);
          }
          to {
            opacity: 1;
            transform: scale(1) translateY(0);
          }
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
}
