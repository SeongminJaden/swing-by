import React from 'react';
import { PanelLeftClose } from 'lucide-react';
import { useAppStore, SidebarTab } from '../../stores/appStore';
import { FileExplorer } from './FileExplorer';
import { GitPanel } from './GitPanel';
import { DevEnvPanel } from './DevEnvPanel';
import { SecurityPanel } from './SecurityPanel';
import { DeployPanel } from './DeployPanel';
import { AgentPanel } from '../agents/AgentPanel';

const tabTitles: Record<SidebarTab, string> = {
  agents:   'AI Agents',
  files:    'Explorer',
  git:      'Source Control',
  deploy:   'Deploy',
  watchdog: 'Security / Watchdog',
  devenv:   'Dev Environment',
  settings: 'Settings',
};

const PlaceholderPanel: React.FC<{ title: string; description: string }> = ({
  title, description,
}) => (
  <div className="sidebar-placeholder">
    <p className="sidebar-placeholder-title">{title}</p>
    <p className="sidebar-placeholder-desc">{description}</p>
  </div>
);

const panelContent: Record<SidebarTab, React.ReactNode> = {
  agents:   <AgentPanel />,
  files:    <FileExplorer />,
  git:      <GitPanel />,
  deploy:   <DeployPanel />,
  watchdog: <SecurityPanel />,
  devenv:   <DevEnvPanel />,
  settings: <PlaceholderPanel title="Settings" description="Manage editor, theme, and language preferences." />,
};

export const Sidebar: React.FC = () => {
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const sidebarCollapsed = useAppStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);

  if (sidebarCollapsed) {
    return null;
  }

  return (
    <div className="sidebar">
      {/* Header */}
      <div className="sidebar-header">
        <span className="sidebar-header-title">
          {tabTitles[sidebarTab]}
        </span>
        <button
          onClick={toggleSidebar}
          className="sidebar-header-btn"
          title="Close sidebar"
        >
          <PanelLeftClose size={14} />
        </button>
      </div>

      {/* Content */}
      <div className="sidebar-panel">
        {panelContent[sidebarTab]}
      </div>
    </div>
  );
};

export default Sidebar;
