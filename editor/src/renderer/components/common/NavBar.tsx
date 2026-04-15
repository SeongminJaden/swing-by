import {
  Minus,
  Square,
  X,
  Settings,
  Workflow,
  Code2,
  MessageSquare,
  PanelBottom,
  BotMessageSquare,
} from 'lucide-react';
import { useAppStore, AppView } from '../../stores/appStore';
import { useProjectStore } from '../../stores/projectStore';

const viewLabels: Record<AppView, string> = {
  ide:        'Workspace',
  settings:   'Settings',
  deploy:     'Deploy',
  watchdog:   'Monitoring',
  newService: 'New Service',
  agents:     'AI Agents',
};

export function NavBar() {
  const {
    currentView, setCurrentView,
    workflowView, toggleWorkflowView,
    chatPanelVisible, toggleChatPanel,
    bottomPanelVisible, toggleBottomPanel,
  } = useAppStore();
  const { currentProject } = useProjectStore();

  const label = viewLabels[currentView] ?? currentView;
  const projectName = currentView === 'ide' && currentProject ? currentProject.name : null;

  return (
    <nav className="navbar">
      {/* Left: Breadcrumb */}
      <div className="navbar-left">
        <div className="navbar-breadcrumb">
          <span className="navbar-breadcrumb-item">AI Agent IDE</span>
          <span className="navbar-breadcrumb-divider">/</span>
          <span className="navbar-breadcrumb-current">{projectName ?? label}</span>
        </div>
      </div>

      {/* Center: Drag area */}
      <div className="navbar-center" />

      {/* Right: Actions + Window controls */}
      <div className="navbar-right">
        {currentView === 'ide' && (
          <>
            <button
              onClick={toggleWorkflowView}
              className="navbar-btn"
              title={workflowView ? 'Code Editor' : 'Agent Graph'}
            >
              {workflowView ? <Code2 size={16} /> : <Workflow size={16} />}
            </button>

            {!workflowView && (
              <>
                <button
                  onClick={toggleChatPanel}
                  className={`navbar-btn ${chatPanelVisible ? 'navbar-btn-active' : ''}`}
                  title={chatPanelVisible ? 'Close AI Chat' : 'Open AI Chat'}
                >
                  <MessageSquare size={16} />
                </button>
                <button
                  onClick={toggleBottomPanel}
                  className={`navbar-btn ${bottomPanelVisible ? 'navbar-btn-active' : ''}`}
                  title={bottomPanelVisible ? 'Close Bottom Panel' : 'Open Bottom Panel'}
                >
                  <PanelBottom size={16} />
                </button>
              </>
            )}

            <button
              onClick={() => setCurrentView('agents')}
              className="navbar-btn"
              title="Agents View"
            >
              <BotMessageSquare size={16} />
            </button>
          </>
        )}

        {currentView !== 'ide' && (
          <button
            onClick={() => setCurrentView('ide')}
            className="navbar-btn"
            title="Back to Editor"
          >
            <Code2 size={16} />
          </button>
        )}

        {currentView !== 'settings' && (
          <button
            onClick={() => setCurrentView('settings')}
            className="navbar-btn"
            title="Settings"
          >
            <Settings size={16} />
          </button>
        )}

        <div className="navbar-separator" />

        <button onClick={() => (window as any).electronAPI?.windowMinimize()} className="navbar-window-btn">
          <Minus size={14} />
        </button>
        <button onClick={() => (window as any).electronAPI?.windowMaximize()} className="navbar-window-btn">
          <Square size={11} />
        </button>
        <button onClick={() => (window as any).electronAPI?.windowClose()} className="navbar-window-btn navbar-window-btn-close">
          <X size={14} />
        </button>
      </div>
    </nav>
  );
}
