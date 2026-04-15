import React, { useState, useRef, useEffect, useCallback } from 'react';
import {
  Bot, ChevronDown, Copy, Paperclip, SendHorizontal,
  X, AtSign, Loader2, Check, User, AlertCircle, Zap,
} from 'lucide-react';
import IconButton from '../common/IconButton';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

interface ChatPanelProps {
  onClose?: () => void;
  className?: string;
}

type BackendMode = 'agent' | 'claude' | 'openai';

function InlineCodeBlock({ language, code }: { language: string; code: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = () => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };
  return (
    <div className="chat-code-block">
      <div className="chat-code-block-header">
        <span className="chat-code-block-lang">{language || 'code'}</span>
        <button onClick={handleCopy} className="chat-code-block-copy">
          {copied ? <Check size={12} /> : <Copy size={12} />}
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre className="chat-code-block-content"><code>{code}</code></pre>
    </div>
  );
}

function renderMessageContent(content: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  const codeBlockRegex = /```(\w*)\n?([\s\S]*?)```/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = codeBlockRegex.exec(content)) !== null) {
    if (match.index > lastIndex) {
      parts.push(<span key={`t-${lastIndex}`} style={{ whiteSpace: 'pre-wrap' }}>{content.slice(lastIndex, match.index)}</span>);
    }
    parts.push(<InlineCodeBlock key={`c-${match.index}`} language={match[1] || ''} code={match[2].replace(/\n$/, '')} />);
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < content.length) {
    parts.push(<span key={`t-${lastIndex}`} style={{ whiteSpace: 'pre-wrap' }}>{content.slice(lastIndex)}</span>);
  }
  return parts;
}

const MODE_OPTIONS: { value: BackendMode; label: string; icon: React.ReactNode }[] = [
  { value: 'agent',  label: 'AI Agent (Rust)',  icon: <Zap size={12} /> },
  { value: 'claude', label: 'Claude API',        icon: <Bot size={12} /> },
  { value: 'openai', label: 'OpenAI GPT-4o',     icon: <Bot size={12} /> },
];

export default function ChatPanel({ onClose, className = '' }: ChatPanelProps) {
  const [message, setMessage] = useState('');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [mode, setMode] = useState<BackendMode>('agent');
  const [modeOpen, setModeOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [streamingText, setStreamingText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [agentReady, setAgentReady] = useState<boolean | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const msgIdCounter = useRef(0);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingText]);

  // Check agent binary on mount
  useEffect(() => {
    const api = (window as any).electronAPI;
    if (!api?.agentCheckBinary) return;
    api.agentCheckBinary().then((r: any) => setAgentReady(r.exists));
  }, []);

  // Listen for streaming events from ai:stream (Claude/OpenAI)
  useEffect(() => {
    const api = (window as any).electronAPI;
    if (!api) return;

    api.onAIStream?.((text: string) => {
      if (mode !== 'agent') setStreamingText((p) => p + text);
    });
    api.onAIStreamEnd?.(() => {
      if (mode !== 'agent') {
        setStreamingText((prev) => {
          if (prev) {
            const id = `msg_${++msgIdCounter.current}`;
            const ts = new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
            setMessages((m) => [...m, { id, role: 'assistant', content: prev, timestamp: ts }]);
          }
          return '';
        });
        setIsLoading(false);
      }
    });
  }, [mode]);

  const handleSend = useCallback(async () => {
    const api = (window as any).electronAPI;
    if (!api || !message.trim() || isLoading) return;

    setError(null);
    const ts = new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
    const userMsg: ChatMessage = { id: `msg_${++msgIdCounter.current}`, role: 'user', content: message.trim(), timestamp: ts };
    const newMessages = [...messages, userMsg];
    setMessages(newMessages);
    setMessage('');
    setIsLoading(true);
    setStreamingText('');

    try {
      if (mode === 'agent') {
        if (!api.agentChat) throw new Error('Agent IPC not available. Make sure the app is running in Electron.');
        const result = await api.agentChat(message.trim(), 'editor-ui');
        const content = result?.content || result?.error || 'No response';
        const replyTs = new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
        setMessages((m) => [...m, { id: `msg_${++msgIdCounter.current}`, role: 'assistant', content, timestamp: replyTs }]);
        setIsLoading(false);
      } else if (mode === 'claude') {
        const chatHistory = newMessages.map((m) => ({ role: m.role, content: m.content }));
        const result = await api.aiChatClaude?.(chatHistory, 'claude-sonnet-4-20250514');
        if (result?.error) {
          setMessages((m) => [...m, { id: `msg_${++msgIdCounter.current}`, role: 'assistant', content: `Error: ${result.error}`, timestamp: ts }]);
          setIsLoading(false);
        }
        // streaming handled by listener
      } else {
        const chatHistory = newMessages.map((m) => ({ role: m.role, content: m.content }));
        const result = await api.aiChatOpenAI?.(chatHistory, 'gpt-4o');
        if (result?.error) {
          setMessages((m) => [...m, { id: `msg_${++msgIdCounter.current}`, role: 'assistant', content: `Error: ${result.error}`, timestamp: ts }]);
          setIsLoading(false);
        }
      }
    } catch (err: any) {
      setError(err?.message || 'Request failed.');
      setIsLoading(false);
    }
  }, [message, messages, isLoading, mode]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); }
  };

  const currentMode = MODE_OPTIONS.find((m) => m.value === mode)!;
  const notReady = mode === 'agent' && agentReady === false;

  return (
    <div className={`chat-panel ${className}`}>
      {/* Header */}
      <div className="chat-header">
        <div className="chat-header-title">
          <Bot size={18} className="text-accent-primary" />
          <span className="text-sm font-semibold text-text-primary">AI Chat</span>
        </div>
        <div className="chat-header-actions">
          <button onClick={() => setModeOpen(!modeOpen)} className="chat-model-selector">
            {currentMode.icon}
            {currentMode.label}
            <ChevronDown size={12} />
            {modeOpen && (
              <div className="chat-model-dropdown">
                {MODE_OPTIONS.map((opt) => (
                  <div key={opt.value} className="chat-model-option" onClick={() => { setMode(opt.value); setModeOpen(false); }}>
                    {opt.icon} {opt.label}
                    {opt.value === mode && <Check size={12} style={{ marginLeft: 'auto' }} />}
                  </div>
                ))}
              </div>
            )}
          </button>
          <IconButton icon={X} size="sm" tooltip="Close" onClick={onClose} />
        </div>
      </div>

      {/* Warning banner */}
      {notReady && (
        <div style={{ padding: '7px 12px', background: 'rgba(240,160,48,0.08)', borderBottom: '1px solid var(--color-border-primary)', display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: 'var(--color-text-secondary)' }}>
          <AlertCircle size={13} style={{ color: 'var(--color-accent-warning, #f0a030)', flexShrink: 0 }} />
          ai_agent binary not found. Run <code style={{ fontSize: 10 }}>cargo build --release</code>.
        </div>
      )}

      {/* Error banner */}
      {error && (
        <div style={{ padding: '7px 12px', background: 'rgba(239,68,68,0.08)', borderBottom: '1px solid var(--color-border-primary)', display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: 'var(--color-accent-error, #ef4444)' }}>
          <AlertCircle size={13} style={{ flexShrink: 0 }} />
          <span style={{ flex: 1 }}>{error}</span>
          <button onClick={() => setError(null)} style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'inherit', display: 'flex' }}><X size={12} /></button>
        </div>
      )}

      {/* Messages */}
      <div className="chat-messages">
        {messages.length === 0 && !streamingText && !isLoading ? (
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--color-text-tertiary)', textAlign: 'center', gap: 8 }}>
            <Bot size={32} style={{ opacity: 0.3 }} />
            <p style={{ fontSize: 13 }}>
              {mode === 'agent' ? 'Chat with the Rust AI agent.\nOr run a sprint from the Agents panel.' : 'Ask a question or request code.'}
            </p>
          </div>
        ) : (
          <>
            {messages.map((msg) =>
              msg.role === 'user' ? (
                <div key={msg.id} className="chat-message-user">
                  <div className="chat-message-user-meta"><span>{msg.timestamp}</span><User size={12} className="text-text-tertiary" /></div>
                  <div className="chat-message-user-bubble">{msg.content}</div>
                </div>
              ) : (
                <div key={msg.id} className="chat-message-ai">
                  <div className="chat-message-ai-avatar"><Bot size={14} className="text-accent-primary" /></div>
                  <div className="chat-message-ai-body">
                    <span className="chat-message-ai-meta">{msg.timestamp}</span>
                    <div className="chat-message-ai-bubble">{renderMessageContent(msg.content)}</div>
                  </div>
                </div>
              )
            )}
            {streamingText && (
              <div className="chat-message-ai">
                <div className="chat-message-ai-avatar"><Bot size={14} className="text-accent-primary" /></div>
                <div className="chat-message-ai-body">
                  <div className="chat-message-ai-bubble">{renderMessageContent(streamingText)}</div>
                </div>
              </div>
            )}
            {isLoading && !streamingText && (
              <div className="chat-message-ai">
                <div className="chat-message-ai-avatar"><Loader2 size={14} className="text-accent-primary" style={{ animation: 'spin 1s linear infinite' }} /></div>
                <div className="chat-message-ai-body">
                  <div className="chat-message-ai-bubble" style={{ color: 'var(--color-text-tertiary)' }}>Thinking...</div>
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Input */}
      <div className="chat-input-area">
        <div className="chat-input-row">
          <div className="flex-1 relative">
            <textarea
              rows={2}
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={notReady ? 'Build ai_agent first (cargo build --release)' : 'Type a message...'}
              className="chat-textarea"
              disabled={isLoading || notReady}
            />
          </div>
          <button
            className="chat-send-btn"
            title="Send"
            onClick={handleSend}
            disabled={isLoading || notReady || !message.trim()}
            style={{ opacity: isLoading || notReady || !message.trim() ? 0.5 : 1 }}
          >
            {isLoading ? <Loader2 size={16} style={{ animation: 'spin 1s linear infinite' }} /> : <SendHorizontal size={16} />}
          </button>
        </div>
        <div className="chat-input-footer">
          <div className="chat-input-tools">
            <IconButton icon={Paperclip} size="sm" tooltip="Attach file" />
            <button className="chat-input-context-btn" title="Context tag"><AtSign size={14} /></button>
          </div>
          <span className="chat-token-info">{currentMode.label} | {messages.length} messages</span>
        </div>
      </div>
    </div>
  );
}
