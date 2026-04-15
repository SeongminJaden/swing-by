import React from 'react';
import {
  FolderTree,
  GitBranch,
  Rocket,
  Activity,
  Box,
  Settings,
  BotMessageSquare,
  Network,
} from 'lucide-react';
import { useAppStore, SidebarTab } from '../../stores/appStore';

interface NavItem {
  id: SidebarTab;
  icon: React.ElementType;
  label: string;
}

const topItems: NavItem[] = [
  { id: 'agents',   icon: BotMessageSquare, label: 'AI Agents' },
  { id: 'files',    icon: FolderTree,        label: 'File Explorer' },
  { id: 'git',      icon: GitBranch,         label: 'Source Control' },
  { id: 'deploy',   icon: Rocket,            label: 'Deploy' },
  { id: 'watchdog', icon: Activity,          label: 'Watchdog' },
  { id: 'devenv',   icon: Box,               label: 'Dev Environment' },
];

const bottomItems: NavItem[] = [
  { id: 'settings', icon: Settings, label: 'Settings' },
];

export const ActivityBar: React.FC = () => {
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);
  const sidebarCollapsed = useAppStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);

  const handleClick = (id: SidebarTab) => {
    if (sidebarTab === id && !sidebarCollapsed) {
      toggleSidebar();
    } else {
      setSidebarTab(id);
      if (sidebarCollapsed) {
        toggleSidebar();
      }
    }
  };

  const renderButton = (item: NavItem) => {
    const isActive = sidebarTab === item.id && !sidebarCollapsed;
    const Icon = item.icon;

    return (
      <button
        key={item.id}
        onClick={() => handleClick(item.id)}
        title={item.label}
        className={isActive ? 'activity-bar-icon-active' : 'activity-bar-icon'}
      >
        {isActive && <div className="activity-bar-icon-indicator" />}
        <Icon size={20} />
      </button>
    );
  };

  return (
    <div className="activity-bar">
      <div className="activity-bar-top">
        {topItems.map(renderButton)}
      </div>
      <div className="activity-bar-bottom">
        {bottomItems.map(renderButton)}
      </div>
    </div>
  );
};

export default ActivityBar;
