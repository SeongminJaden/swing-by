import { Panel, Group as PanelGroup, Separator } from 'react-resizable-panels';
import { useAppStore } from '../stores/appStore';
import { ActivityBar } from '../components/sidebar/ActivityBar';
import { Sidebar } from '../components/sidebar/Sidebar';
import { EditorPanel } from '../components/editor/EditorPanel';
import ChatPanel from '../components/chat/ChatPanel';
import PreviewPanel from '../components/preview/PreviewPanel';
import DebugPanel from '../components/debug/DebugPanel';
import { AgentGraph } from '../components/agents/AgentGraph';
import { ChatWidget } from '../components/chat/ChatWidget';

function ResizeHandle({ direction = 'vertical' }: { direction?: 'vertical' | 'horizontal' }) {
  return (
    <Separator
      className={direction === 'vertical' ? 'ide-resize-handle-v' : 'ide-resize-handle-h'}
    />
  );
}

export function IDEPage() {
  const {
    sidebarCollapsed,
    bottomPanelVisible,
    chatPanelVisible,
    previewVisible,
    workflowView,
  } = useAppStore();

  // Agent graph mode: full canvas + floating chat widget
  if (workflowView) {
    return (
      <div className="ide-layout">
        <div className="ide-main" style={{ position: 'relative' }}>
          <AgentGraph />
          <ChatWidget />
        </div>
      </div>
    );
  }

  // Code editor mode: VS Code-style layout
  return (
    <div className="ide-layout">
      <div className="ide-main">
        <ActivityBar />

        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          {!sidebarCollapsed && (
            <>
              <div style={{ width: '240px', minWidth: '180px', maxWidth: '380px', flexShrink: 0 }}>
                <Sidebar />
              </div>
              <div className="ide-resize-handle-v" style={{ width: '3px', flexShrink: 0 }} />
            </>
          )}

          {/* Vertical split: editor (top) + bottom panel */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <div style={{ flex: 1, overflow: 'hidden', display: 'flex', minHeight: 0 }}>
              {previewVisible ? (
                <PanelGroup orientation="horizontal">
                  <Panel defaultSize={50} minSize={30}>
                    <EditorPanel />
                  </Panel>
                  <ResizeHandle />
                  <Panel defaultSize={50} minSize={30}>
                    <PreviewPanel />
                  </Panel>
                </PanelGroup>
              ) : (
                <div style={{ width: '100%', height: '100%' }}>
                  <EditorPanel />
                </div>
              )}
            </div>

            {bottomPanelVisible && (
              <>
                <div className="ide-resize-handle-h" style={{ height: '3px', flexShrink: 0 }} />
                <div style={{ height: '250px', minHeight: '120px', maxHeight: '500px', flexShrink: 0, overflow: 'hidden' }}>
                  <DebugPanel />
                </div>
              </>
            )}
          </div>

          {/* Chat panel (right) */}
          {chatPanelVisible && (
            <>
              <div className="ide-resize-handle-v" style={{ width: '3px', flexShrink: 0 }} />
              <div style={{ width: '320px', minWidth: '250px', maxWidth: '500px', flexShrink: 0 }}>
                <ChatPanel />
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
