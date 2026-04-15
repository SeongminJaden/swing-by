import React, { useEffect, useState } from 'react';
import { useAppStore } from '../../stores/appStore';

interface ServiceItem {
  id: string;
  name: string;
  icon: string;
}

interface ServiceSection {
  title: string;
  services: ServiceItem[];
}

const sections: ServiceSection[] = [
  {
    title: 'AI 프로바이더',
    services: [
      { id: 'claude', name: 'Claude', icon: '\uD83E\uDD16' },
      { id: 'openai', name: 'OpenAI', icon: '\uD83E\uDDE0' },
      { id: 'gemini', name: 'Gemini', icon: '\uD83D\uDCA0' },
    ],
  },
  {
    title: '개발 플랫폼',
    services: [
      { id: 'github', name: 'GitHub', icon: '\uD83D\uDC19' },
      { id: 'gitlab', name: 'GitLab', icon: '\uD83E\uDD8A' },
    ],
  },
  {
    title: '출시 플랫폼',
    services: [
      { id: 'vercel', name: 'Vercel', icon: '\u25B2' },
      { id: 'netlify', name: 'Netlify', icon: '\uD83C\uDF10' },
      { id: 'railway', name: 'Railway', icon: '\uD83D\uDE82' },
    ],
  },
  {
    title: '데이터베이스',
    services: [
      { id: 'supabase', name: 'Supabase', icon: '\u26A1' },
      { id: 'firebase', name: 'Firebase', icon: '\uD83D\uDD25' },
    ],
  },
  {
    title: '결제',
    services: [
      { id: 'stripe', name: 'Stripe', icon: '\uD83D\uDCB3' },
    ],
  },
  {
    title: '앱 스토어',
    services: [
      { id: 'apple-developer', name: 'Apple Developer', icon: '\uD83C\uDF4E' },
      { id: 'google-developer', name: 'Google Developer', icon: '\uD83E\uDD16' },
    ],
  },
];

const ConnectedBadge: React.FC<{ maskedKey?: string }> = ({ maskedKey }) => (
  <span className="accounts-connected-badge">
    <span className="accounts-connected-dot" />
    {maskedKey || '연결됨'}
  </span>
);

const ConnectButton: React.FC<{ serviceId: string }> = ({ serviceId }) => {
  const openAuthModal = useAppStore((s) => s.openAuthModal);

  return (
    <button
      onClick={() => openAuthModal(serviceId)}
      className="accounts-connect-btn"
    >
      연결하기
    </button>
  );
};

const DisconnectButton: React.FC<{ serviceId: string; onDisconnect: () => void }> = ({ serviceId, onDisconnect }) => {
  const [confirming, setConfirming] = useState(false);

  const handleDisconnect = async () => {
    const api = (window as any).electronAPI;
    if (api?.connectionsDelete) {
      await api.connectionsDelete(serviceId);
      onDisconnect();
    }
  };

  if (confirming) {
    return (
      <div style={{ display: 'flex', gap: '4px' }}>
        <button
          onClick={handleDisconnect}
          style={{ fontSize: '10px', color: 'var(--color-accent-error)', cursor: 'pointer' }}
        >
          확인
        </button>
        <button
          onClick={() => setConfirming(false)}
          style={{ fontSize: '10px', color: 'var(--color-text-tertiary)', cursor: 'pointer' }}
        >
          취소
        </button>
      </div>
    );
  }

  return (
    <button
      onClick={() => setConfirming(true)}
      style={{ fontSize: '10px', color: 'var(--color-text-tertiary)', cursor: 'pointer' }}
    >
      해제
    </button>
  );
};

export const AccountsPanel: React.FC = () => {
  const serviceConnections = useAppStore((s) => s.serviceConnections);
  const setServiceConnections = useAppStore((s) => s.setServiceConnections);
  const updateServiceConnection = useAppStore((s) => s.updateServiceConnection);

  useEffect(() => {
    const api = (window as any).electronAPI;
    if (api?.connectionsGetAll) {
      api.connectionsGetAll().then((result: any) => {
        if (result && typeof result === 'object') {
          setServiceConnections(result);
        }
      });
    }
  }, [setServiceConnections]);

  const handleDisconnect = (serviceId: string) => {
    updateServiceConnection(serviceId, { connected: false });
  };

  return (
    <div className="accounts-panel">
      {sections.map((section) => (
        <div key={section.title} className="accounts-section">
          <div className="accounts-section-title">
            {section.title}
          </div>

          {section.services.map((service) => {
            const conn = serviceConnections[service.id];
            const isConnected = conn?.connected || false;

            return (
              <div key={service.id} className="accounts-service-item">
                <div className="accounts-service-info">
                  <span className="accounts-service-icon">{service.icon}</span>
                  <span className="accounts-service-name">{service.name}</span>
                </div>
                {isConnected ? (
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <ConnectedBadge maskedKey={conn?.maskedKey} />
                    <DisconnectButton serviceId={service.id} onDisconnect={() => handleDisconnect(service.id)} />
                  </div>
                ) : (
                  <ConnectButton serviceId={service.id} />
                )}
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
};

export default AccountsPanel;
