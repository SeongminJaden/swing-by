import { useEffect, useRef, useState } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from '@xterm/addon-fit';
import 'xterm/css/xterm.css';
import { useFileStore } from '../../stores/fileStore';

export function TerminalPanel() {
  const termRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const termIdRef = useRef<string | null>(null);
  const [connected, setConnected] = useState(false);
  const { workspacePath } = useFileStore();

  useEffect(() => {
    const api = window.electronAPI;
    if (!termRef.current || !api) return;

    // Create xterm instance
    const term = new Terminal({
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      fontSize: 13,
      lineHeight: 1.5,
      cursorBlink: true,
      cursorStyle: 'bar',
      theme: {
        background: '#0d1117',
        foreground: '#e6edf3',
        cursor: '#58a6ff',
        selectionBackground: '#264f78',
        black: '#0d1117',
        red: '#f85149',
        green: '#3fb950',
        yellow: '#d29922',
        blue: '#58a6ff',
        magenta: '#bc8cff',
        cyan: '#79c0ff',
        white: '#e6edf3',
        brightBlack: '#484f58',
        brightRed: '#f85149',
        brightGreen: '#3fb950',
        brightYellow: '#d29922',
        brightBlue: '#79c0ff',
        brightMagenta: '#bc8cff',
        brightCyan: '#79c0ff',
        brightWhite: '#ffffff',
      },
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(termRef.current);

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    // Fit to container
    setTimeout(() => {
      fitAddon.fit();
    }, 100);

    // Create PTY process
    const cwd = workspacePath || undefined;
    api.terminalCreate(cwd).then((id) => {
      termIdRef.current = id;
      setConnected(true);

      // Send terminal size
      api.terminalResize(id, term.cols, term.rows);
    });

    // Receive data from PTY → xterm
    api.onTerminalData((id, data) => {
      if (id === termIdRef.current) {
        term.write(data);
      }
    });

    // Send data from xterm → PTY
    term.onData((data) => {
      if (termIdRef.current) {
        api.terminalWrite(termIdRef.current, data);
      }
    });

    // Handle resize
    const handleResize = () => {
      fitAddon.fit();
      if (termIdRef.current) {
        api.terminalResize(termIdRef.current, term.cols, term.rows);
      }
    };

    const resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(termRef.current);

    // Handle exit
    api.onTerminalExit((id, _exitCode) => {
      if (id === termIdRef.current) {
        term.write('\r\n\x1b[90m[터미널 종료]\x1b[0m\r\n');
        setConnected(false);
      }
    });

    return () => {
      resizeObserver.disconnect();
      if (termIdRef.current) {
        api.terminalKill(termIdRef.current);
      }
      term.dispose();
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div style={{ width: '100%', height: '100%', background: '#0d1117', overflow: 'hidden' }}>
      <div ref={termRef} style={{ width: '100%', height: '100%' }} />
    </div>
  );
}

export default TerminalPanel;
