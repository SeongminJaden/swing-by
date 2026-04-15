import React from 'react';
import {
  Plus,
  ExternalLink,
  FolderOpen,
  Github,
  Bot,
  Triangle,
  Check,
  Clock,
  ArrowRight,
  Cloud,
  Server,
  Database,
  Globe,
  CreditCard,
  MessageSquare,
  Cpu,
  Flame,
  Container,
  Boxes,
  Shield,
  Webhook,
  Key,
  Box,
  CheckCircle2,
} from 'lucide-react';
import { useAppStore } from '../stores/appStore';
import { useProjectStore, Project } from '../stores/projectStore';
import StatusBadge from '../components/common/StatusBadge';
import { useTranslation } from '../i18n';

export default function DashboardPage() {
  const { setCurrentView } = useAppStore();
  const { projects, setCurrentProject } = useProjectStore();
  const { t } = useTranslation();

  const handleOpenProject = (project: Project) => {
    setCurrentProject(project);
    setCurrentView('ide');
  };

  const recentActivities = projects.map((p) => ({
    id: p.id,
    projectName: p.name,
    activity: p.lastActivity,
    status: p.status,
  }));

  return (
    <div className="h-full overflow-y-auto">
      <div className="page-content-inner">

          {/* Header */}
          <div className="dashboard-header">
            <div>
              <h1 className="heading-1">{t('dashboard.myServices')}</h1>
              <p className="subtitle mt-3">{t('dashboard.myServicesSubtitle')}</p>
            </div>
            <button className="btn-primary" onClick={() => setCurrentView('newService')}>
              <Plus size={18} strokeWidth={2.5} />
              {t('dashboard.newService')}
            </button>
          </div>

          {/* Project grid */}
          <div className="section">
            {projects.length === 0 ? (
              <div className="card" style={{ padding: '48px 32px', textAlign: 'center' }}>
                <FolderOpen size={48} style={{ margin: '0 auto 16px', color: 'var(--color-text-tertiary)', opacity: 0.4 }} />
                <p className="text-body" style={{ marginBottom: '8px' }}>{t('dashboard.noServices')}</p>
                <p className="text-caption" style={{ marginBottom: '20px' }}>{t('dashboard.noServicesHint')}</p>
                <button className="btn-primary" onClick={() => setCurrentView('newService')} style={{ margin: '0 auto' }}>
                  <Plus size={18} strokeWidth={2.5} />
                  {t('dashboard.createNewService')}
                </button>
              </div>
            ) : (
              <div className="project-grid">
                {projects.map((project) => (
                  <button
                    key={project.id}
                    onClick={() => handleOpenProject(project)}
                    className="project-card"
                  >
                    <div className="flex items-start justify-between">
                      <div className="project-card-icon">
                        <FolderOpen size={30} className="text-accent-primary" />
                      </div>
                      <StatusBadge status={project.status} />
                    </div>

                    <h3 className="project-card-title">{project.name}</h3>

                    <span className="badge self-start">{project.framework}</span>

                    {project.deployUrl && (
                      <div className="mt-4 flex items-center gap-2">
                        <ExternalLink size={14} className="shrink-0 text-accent-success" />
                        <span className="text-sm font-medium text-accent-success">
                          {project.deployUrl}
                        </span>
                      </div>
                    )}

                    <div className="flex-1" />

                    <div className="project-card-footer">
                      <div className="flex items-center gap-2.5">
                        <Clock size={14} className="shrink-0 text-text-tertiary" />
                        <p className="text-caption">{project.lastActivity}</p>
                      </div>
                      <ArrowRight size={18} className="project-card-arrow" />
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Recent activity */}
          {recentActivities.length > 0 && (
            <div className="section">
              <h2 className="section-header heading-2">{t('dashboard.recentActivity')}</h2>
              <div className="list-card">
                {recentActivities.map((item, i) => (
                  <div
                    key={item.id}
                    className={i < recentActivities.length - 1 ? 'list-card-row-bordered' : 'list-card-row'}
                  >
                    <div className="flex items-center gap-4">
                      <StatusBadge status={item.status} />
                      <span className="text-body">{item.projectName}</span>
                    </div>
                    <span className="text-caption shrink-0 ml-4">{item.activity}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Dev Environment Summary */}
          <DevEnvSummary />

          {/* Connected services */}
          <ServiceSection />

        </div>
    </div>
  );
}

// ========================================
// 개발 환경 요약 섹션
// ========================================

interface DevEnvTool {
  name: string;
  version: string;
  color: string;
}

const toolDetectCommands = [
  { name: 'Node.js', cmd: 'node --version', color: 'bg-accent-success/15 text-accent-success' },
  { name: 'Python', cmd: 'python3 --version 2>&1 || python --version 2>&1', color: 'bg-accent-primary/15 text-accent-primary' },
  { name: 'Java', cmd: 'java --version 2>&1 | head -1', color: 'bg-accent-warning/15 text-accent-warning' },
  { name: 'Git', cmd: 'git --version', color: 'bg-accent-error/15 text-accent-error' },
  { name: 'Docker', cmd: 'docker --version', color: 'bg-accent-info/15 text-accent-info' },
  { name: 'Go', cmd: 'go version', color: 'bg-accent-primary/15 text-accent-primary' },
  { name: 'Rust', cmd: 'rustc --version', color: 'bg-accent-warning/15 text-accent-warning' },
  { name: 'npm', cmd: 'npm --version', color: 'bg-accent-success/15 text-accent-success' },
  { name: 'pnpm', cmd: 'pnpm --version', color: 'bg-accent-warning/15 text-accent-warning' },
  { name: 'Bun', cmd: 'bun --version', color: 'bg-accent-primary/15 text-accent-primary' },
];

function parseVersion(output: string): string | null {
  if (!output || output.trim() === '') return null;
  const match = output.match(/v?(\d+\.\d+[\.\d]*)/);
  return match ? `v${match[1]}` : output.trim().substring(0, 30);
}

function DevEnvSummary() {
  const { setCurrentView } = useAppStore();
  const { t } = useTranslation();
  const [tools, setTools] = React.useState<DevEnvTool[]>([]);
  const [loading, setLoading] = React.useState(true);

  React.useEffect(() => {
    const api = (window as any).electronAPI;
    if (!api?.execCommand) {
      setLoading(false);
      return;
    }

    (async () => {
      const detected: DevEnvTool[] = [];
      for (const t of toolDetectCommands) {
        try {
          const res = await api.execCommand(t.cmd);
          if (res?.success && res.output) {
            const ver = parseVersion(res.output);
            if (ver) {
              detected.push({ name: t.name, version: ver, color: t.color });
            }
          }
        } catch { /* skip */ }
      }
      setTools(detected);
      setLoading(false);
    })();
  }, []);

  return (
    <div className="section">
      <div className="flex items-center justify-between" style={{ marginBottom: '24px' }}>
        <h2 className="heading-2">{t('dashboard.devEnvironment')}</h2>
        <button
          onClick={() => {
            setCurrentView('ide');
          }}
          className="btn-ghost"
        >
          {t('dashboard.viewAll')}
        </button>
      </div>
      <div className="card" style={{ padding: '32px 36px' }}>
        <div className="flex items-center justify-between" style={{ marginBottom: '20px' }}>
          <div className="flex items-center" style={{ gap: '12px' }}>
            <div
              className="flex items-center justify-center"
              style={{
                width: '44px',
                height: '44px',
                borderRadius: '14px',
                background: 'rgba(88, 166, 255, 0.12)',
              }}
            >
              <Box size={22} className="text-accent-primary" />
            </div>
            <div>
              <p className="text-body">{t('dashboard.installedTools')}</p>
              <p className="text-caption">
                {loading ? t('dashboard.detecting') : `${tools.length}${t('dashboard.toolsDetected')}`}
              </p>
            </div>
          </div>
          {!loading && tools.length > 0 && (
            <div className="flex items-center" style={{ gap: '6px' }}>
              <CheckCircle2 size={16} className="text-accent-success" />
              <span className="text-sm font-semibold text-accent-success">{t('dashboard.healthy')}</span>
            </div>
          )}
        </div>
        {loading ? (
          <div className="flex items-center justify-center" style={{ padding: '16px', color: 'var(--color-text-tertiary)' }}>
            {t('dashboard.detectingEnv')}
          </div>
        ) : tools.length === 0 ? (
          <div className="flex items-center justify-center" style={{ padding: '16px', color: 'var(--color-text-tertiary)' }}>
            {t('dashboard.noToolsDetected')}
          </div>
        ) : (
          <div className="flex flex-wrap" style={{ gap: '8px' }}>
            {tools.map((tool) => (
              <span
                key={tool.name}
                className={`inline-flex items-center font-semibold ${tool.color}`}
                style={{
                  padding: '8px 16px',
                  borderRadius: '14px',
                  fontSize: '13px',
                  gap: '6px',
                  whiteSpace: 'nowrap',
                }}
              >
                {tool.name}
                <span style={{ opacity: 0.7, fontSize: '11px', fontFamily: 'var(--font-mono, monospace)' }}>
                  {tool.version}
                </span>
              </span>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ========================================
// 서비스 연결 섹션
// ========================================

interface Service {
  id: string;
  name: string;
  description: string;
  icon: React.ReactNode;
  category: 'ai' | 'git' | 'deploy' | 'database' | 'payment' | 'notification' | 'auth' | 'storage' | 'appstore';
  connected: boolean;
}

const allServices: Service[] = [
  // AI 프로바이더
  { id: 'claude', name: 'Claude', description: 'Anthropic AI 모델', icon: <Bot size={22} />, category: 'ai', connected: false },
  { id: 'openai', name: 'OpenAI', description: 'GPT-4o, o3', icon: <Cpu size={22} />, category: 'ai', connected: false },
  { id: 'gemini', name: 'Gemini', description: 'Google AI 모델', icon: <Globe size={22} />, category: 'ai', connected: false },
  { id: 'ollama', name: 'Ollama', description: '로컬 AI 모델', icon: <Server size={22} />, category: 'ai', connected: false },

  // Git 플랫폼
  { id: 'github', name: 'GitHub', description: '소스 코드 관리', icon: <Github size={22} />, category: 'git', connected: false },
  { id: 'gitlab', name: 'GitLab', description: 'DevOps 플랫폼', icon: <Boxes size={22} />, category: 'git', connected: false },
  { id: 'bitbucket', name: 'Bitbucket', description: 'Atlassian Git', icon: <Container size={22} />, category: 'git', connected: false },

  // 배포 플랫폼
  { id: 'vercel', name: 'Vercel', description: 'Next.js 출시 최적화', icon: <Triangle size={22} />, category: 'deploy', connected: false },
  { id: 'railway', name: 'Railway', description: '풀스택 앱 출시', icon: <Server size={22} />, category: 'deploy', connected: false },
  { id: 'netlify', name: 'Netlify', description: '정적 사이트 + Functions', icon: <Globe size={22} />, category: 'deploy', connected: false },
  { id: 'aws', name: 'AWS', description: 'EC2, ECS, Lambda, S3', icon: <Cloud size={22} />, category: 'deploy', connected: false },
  { id: 'gcp', name: 'Google Cloud', description: 'Cloud Run, Firebase', icon: <Cloud size={22} />, category: 'deploy', connected: false },
  { id: 'cloudflare', name: 'Cloudflare', description: 'Pages, Workers, R2', icon: <Shield size={22} />, category: 'deploy', connected: false },
  { id: 'flyio', name: 'Fly.io', description: '글로벌 엣지 출시', icon: <Globe size={22} />, category: 'deploy', connected: false },
  { id: 'digitalocean', name: 'DigitalOcean', description: 'App Platform, Droplets', icon: <Cloud size={22} />, category: 'deploy', connected: false },
  { id: 'heroku', name: 'Heroku', description: 'PaaS 호스팅', icon: <Cloud size={22} />, category: 'deploy', connected: false },
  { id: 'render', name: 'Render', description: '자동 출시 + DB', icon: <Server size={22} />, category: 'deploy', connected: false },

  // 데이터베이스 / BaaS
  { id: 'supabase', name: 'Supabase', description: 'PostgreSQL + Auth', icon: <Database size={22} />, category: 'database', connected: false },
  { id: 'firebase', name: 'Firebase', description: 'Firestore + Auth', icon: <Flame size={22} />, category: 'database', connected: false },
  { id: 'planetscale', name: 'PlanetScale', description: 'MySQL 서버리스', icon: <Database size={22} />, category: 'database', connected: false },
  { id: 'neon', name: 'Neon', description: 'PostgreSQL 서버리스', icon: <Database size={22} />, category: 'database', connected: false },
  { id: 'mongodb', name: 'MongoDB Atlas', description: 'NoSQL 클라우드 DB', icon: <Database size={22} />, category: 'database', connected: false },
  { id: 'redis', name: 'Redis Cloud', description: '캐시 + 메시지큐', icon: <Database size={22} />, category: 'database', connected: false },
  { id: 'turso', name: 'Turso', description: 'SQLite 엣지 DB', icon: <Database size={22} />, category: 'database', connected: false },

  // 결제
  { id: 'stripe', name: 'Stripe', description: '글로벌 결제', icon: <CreditCard size={22} />, category: 'payment', connected: false },
  { id: 'tosspay', name: '토스페이먼츠', description: '국내 결제', icon: <CreditCard size={22} />, category: 'payment', connected: false },

  // 알림
  { id: 'slack', name: 'Slack', description: '팀 메시지 + 알림', icon: <MessageSquare size={22} />, category: 'notification', connected: false },
  { id: 'discord', name: 'Discord', description: '커뮤니티 알림', icon: <MessageSquare size={22} />, category: 'notification', connected: false },
  { id: 'telegram', name: 'Telegram', description: '봇 알림', icon: <MessageSquare size={22} />, category: 'notification', connected: false },

  // 인증
  { id: 'auth0', name: 'Auth0', description: '인증 서비스', icon: <Key size={22} />, category: 'auth', connected: false },
  { id: 'clerk', name: 'Clerk', description: '사용자 인증 + 관리', icon: <Key size={22} />, category: 'auth', connected: false },

  // 스토리지
  { id: 's3', name: 'AWS S3', description: '파일 스토리지', icon: <Cloud size={22} />, category: 'storage', connected: false },
  { id: 'r2', name: 'Cloudflare R2', description: '오브젝트 스토리지', icon: <Cloud size={22} />, category: 'storage', connected: false },
  { id: 'uploadthing', name: 'UploadThing', description: '파일 업로드', icon: <Cloud size={22} />, category: 'storage', connected: false },

  // 앱 스토어
  { id: 'apple-developer', name: 'Apple Developer', description: 'App Store Connect API', icon: <Globe size={22} />, category: 'appstore', connected: false },
  { id: 'google-developer', name: 'Google Developer', description: 'Google Play Console API', icon: <Globe size={22} />, category: 'appstore', connected: false },
];

const categoryLabels: Record<string, string> = {
  ai: 'AI 프로바이더',
  git: 'Git 플랫폼',
  deploy: '출시 플랫폼',
  database: '데이터베이스',
  payment: '결제',
  notification: '알림',
  auth: '인증',
  storage: '스토리지',
  appstore: '앱 스토어',
};

const categoryOrder = ['ai', 'git', 'deploy', 'database', 'payment', 'notification', 'auth', 'storage', 'appstore'];

function ServiceSection() {
  const { t } = useTranslation();
  const serviceConnections = useAppStore((s) => s.serviceConnections);
  const setServiceConnections = useAppStore((s) => s.setServiceConnections);

  // Load real connection statuses on mount
  React.useEffect(() => {
    const api = (window as any).electronAPI;
    if (api?.connectionsGetAll) {
      api.connectionsGetAll().then((result: any) => {
        if (result && typeof result === 'object') {
          setServiceConnections(result);
        }
      });
    }
  }, [setServiceConnections]);

  // Apply real connection status
  const servicesWithStatus = allServices.map((s) => ({
    ...s,
    connected: serviceConnections[s.id]?.connected || false,
  }));

  // 연결된 서비스 먼저, 그 다음 미연결
  const sorted = [...servicesWithStatus].sort((a, b) => {
    if (a.connected && !b.connected) return -1;
    if (!a.connected && b.connected) return 1;
    return 0;
  });

  // 카테고리별 그룹핑
  const grouped = categoryOrder
    .map((cat) => ({
      category: cat,
      label: categoryLabels[cat],
      services: sorted.filter((s) => s.category === cat),
    }))
    .filter((g) => g.services.length > 0);

  return (
    <div>
      <h2 className="section-header heading-2">{t('dashboard.connectedServices')}</h2>
      <div className="flex flex-col" style={{ gap: '32px' }}>
        {grouped.map((group) => (
          <div key={group.category}>
            <h3 className="text-sm font-semibold text-text-tertiary uppercase tracking-wider" style={{ marginBottom: '12px' }}>
              {group.label}
            </h3>
            <div className="service-grid">
              {group.services.map((service) => (
                <ServiceCard
                  key={service.id}
                  id={service.id}
                  icon={service.icon}
                  name={service.name}
                  description={service.description}
                  connected={service.connected}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function ServiceCard({
  id,
  icon,
  name,
  description,
  connected,
}: {
  id: string;
  icon: React.ReactNode;
  name: string;
  description: string;
  connected: boolean;
}) {
  const openAuthModal = useAppStore((s) => s.openAuthModal);
  const { t } = useTranslation();

  return (
    <div className="service-card">
      <div className={connected ? 'service-card-icon-connected' : 'service-card-icon-disconnected'}>
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <span className="text-body block">{name}</span>
        <p className="text-sm text-text-secondary mt-1">{description}</p>
      </div>
      {connected ? (
        <div className="badge-success">
          <Check size={14} className="text-accent-success" strokeWidth={2.5} />
          <span className="text-sm font-semibold text-accent-success">{t('dashboard.connected')}</span>
        </div>
      ) : (
        <button className="btn-ghost shrink-0" onClick={() => openAuthModal(id)}>{t('dashboard.connect')}</button>
      )}
    </div>
  );
}
