/**
 * Connects Electron IPC agent events to the agentStore.
 * Mount once at app root.
 */
import { useEffect } from 'react';
import { useAgentStore } from '../stores/agentStore';

export function useAgentEvents() {
  const { setAgentStatus, appendAgentLog, setAgentProgress, setSprintRunning } = useAgentStore();

  useEffect(() => {
    const api = (window as any).electronAPI;
    if (!api) return;

    // Generic log lines from agent stderr
    api.onAgentLog?.((text: string) => {
      // Parse "[IPC/Sprint] <agentId>: <message>" pattern if present
      const sprintMatch = text.match(/\[IPC\/Sprint\]\s+(\w+):\s+(.*)/i);
      if (sprintMatch) {
        const agentId = sprintMatch[1].toLowerCase().replace(/\s+/g, '_');
        const msg = sprintMatch[2];
        appendAgentLog(agentId, msg);
      }
    });

    // Structured sprint progress events (future extension)
    api.onAgentSprintProgress?.((data: {
      agentId?: string;
      status?: string;
      message?: string;
      progress?: number;
      sprintDone?: boolean;
    }) => {
      if (data.agentId) {
        if (data.status === 'running') setAgentStatus(data.agentId, 'running');
        if (data.status === 'done')    setAgentStatus(data.agentId, 'done');
        if (data.status === 'error')   setAgentStatus(data.agentId, 'error');
        if (data.message)              appendAgentLog(data.agentId, data.message);
        if (data.progress !== undefined) setAgentProgress(data.agentId, data.progress);
      }
      if (data.sprintDone) setSprintRunning(false);
    });

    // Agent process exit
    api.onAgentExit?.((code: number | null) => {
      setSprintRunning(false);
    });
  }, [setAgentStatus, appendAgentLog, setAgentProgress, setSprintRunning]);
}
