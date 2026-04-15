import { lazy, Suspense, useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { useAppStore } from './stores/appStore';
import { useProjectStore } from './stores/projectStore';
import { NavBar } from './components/common/NavBar';
import { IDEPage } from './pages/IDEPage';
import ToastContainer from './components/common/Toast';
import { AgentDetailOverlay } from './components/agents/AgentDetailOverlay';
import { useAgentEvents } from './hooks/useAgentEvents';

const SettingsPage = lazy(() => import('./pages/SettingsPage'));
const NewServicePage = lazy(() => import('./pages/NewServicePage'));
const WatchdogPage = lazy(() => import('./pages/WatchdogPage'));

export function App() {
  const { currentView, theme } = useAppStore();
  const { loadProjects } = useProjectStore();
  useAgentEvents();

  useEffect(() => {
    loadProjects();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Apply theme class to <html> element
  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove('theme-light', 'theme-monokai');
    if (theme === 'light') {
      root.classList.add('theme-light');
    } else if (theme === 'monokai') {
      root.classList.add('theme-monokai');
    }
  }, [theme]);

  const renderView = () => {
    switch (currentView) {
      case 'ide':
      case 'agents':
        return <IDEPage />;
      case 'settings':
        return <SettingsPage />;
      case 'newService':
        return <NewServicePage />;
      case 'watchdog':
        return <WatchdogPage />;
      default:
        return <IDEPage />;
    }
  };

  return (
    <div className="flex flex-col h-screen bg-bg-primary text-text-primary overflow-hidden">
      <NavBar />
      <div className="flex-1 min-h-0">
        <Suspense fallback={<div style={{display:'flex',alignItems:'center',justifyContent:'center',height:'100%',color:'var(--color-text-tertiary)'}}><Loader2 className="animate-spin" size={24} /></div>}>
          {renderView()}
        </Suspense>
      </div>
      <AgentDetailOverlay />
      <ToastContainer />
    </div>
  );
}
