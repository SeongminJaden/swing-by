import { useState, useRef, useEffect, useCallback } from 'react';
import {
  MessageCircle,
  X,
  SendHorizontal,
  ChevronDown,
  Bot,
  Paperclip,
  AtSign,
  Loader2,
  Key,
  FileCode,
  Save,
  FolderOpen,
  Check,
  AlertCircle,
} from 'lucide-react';
import { parseCodeBlocks, hasCodeBlocksWithFiles, ParsedCodeBlock } from '../../utils/codeParser';
import { writeCodeBlocks, openFileInEditor, WriteResult } from '../../services/codeGenService';
import { useFileStore } from '../../stores/fileStore';
import { useWorkflowStore } from '../../stores/workflowStore';

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  time: string;
  codeBlocks?: ParsedCodeBlock[];
}

interface CodeSaveState {
  messageId: string;
  blocks: ParsedCodeBlock[];
  results: WriteResult[] | null;
  saving: boolean;
}

type Provider = 'anthropic' | 'openai';

export function ChatWidget() {
  const [isOpen, setIsOpen] = useState(false);
  const [input, setInput] = useState('');
  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [streamingText, setStreamingText] = useState('');
  const [provider, setProvider] = useState<Provider>('anthropic');
  const [hasKey, setHasKey] = useState(false);
  const [showKeyInput, setShowKeyInput] = useState(false);
  const [keyInput, setKeyInput] = useState('');
  const [codeSaveStates, setCodeSaveStates] = useState<Record<string, CodeSaveState>>({});
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const msgIdCounter = useRef(0);

  const workspacePath = useFileStore((s) => s.workspacePath);
  const { setNodeStatus, completeNode, startNode } = useWorkflowStore();

  // Check if API key is set
  useEffect(() => {
    const api = window.electronAPI;
    if (!api) return;
    api.aiHasKey(provider).then(setHasKey);
  }, [provider]);

  // Scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingText]);

  // Setup streaming listeners
  useEffect(() => {
    const api = window.electronAPI;
    if (!api) return;

    api.onAIStream((text) => {
      setStreamingText((prev) => prev + text);
    });

    api.onAIStreamEnd(() => {
      setStreamingText((prev) => {
        if (prev) {
          const id = `msg_${++msgIdCounter.current}`;
          const time = new Date().toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' });

          // 코드 블록 파싱
          const codeBlocks = parseCodeBlocks(prev);

          const newMsg: Message = { id, role: 'assistant', content: prev, time, codeBlocks: codeBlocks.length > 0 ? codeBlocks : undefined };
          setMessages((msgs) => [...msgs, newMsg]);

          // 코드 블록이 있으면 save state 초기화
          if (codeBlocks.length > 0) {
            setCodeSaveStates((prev) => ({
              ...prev,
              [id]: { messageId: id, blocks: codeBlocks, results: null, saving: false },
            }));

            // 워크플로우 노드 업데이트
            completeNode('ai-model');
            setNodeStatus('codegen', 'completed', `${codeBlocks.length}개 파일 생성 완료`);
            startNode('filesystem');
          }
        }
        return '';
      });
      setIsLoading(false);
    });
  }, []);

  const handleSetKey = async () => {
    const api = window.electronAPI;
    if (!api || !keyInput.trim()) return;
    await api.aiSetKey(provider, keyInput.trim());
    setHasKey(true);
    setShowKeyInput(false);
    setKeyInput('');
  };

  const handleSaveAll = async (messageId: string) => {
    const state = codeSaveStates[messageId];
    if (!state || state.saving || !workspacePath) return;

    setCodeSaveStates((prev) => ({
      ...prev,
      [messageId]: { ...prev[messageId], saving: true },
    }));

    // 워크플로우 업데이트
    startNode('filesystem');

    const results = await writeCodeBlocks(state.blocks, workspacePath);

    setCodeSaveStates((prev) => ({
      ...prev,
      [messageId]: { ...prev[messageId], saving: false, results },
    }));

    // 워크플로우 완료
    const allSuccess = results.every((r) => r.success);
    if (allSuccess) {
      completeNode('filesystem');
    } else {
      setNodeStatus('filesystem', 'error', '일부 파일 저장 실패');
    }
  };

  const handleOpenFile = async (fullPath: string) => {
    await openFileInEditor(fullPath);
  };

  const handleSend = useCallback(async () => {
    const api = window.electronAPI;
    if (!api || !input.trim() || isLoading) return;

    const userMsg: Message = {
      id: `msg_${++msgIdCounter.current}`,
      role: 'user',
      content: input.trim(),
      time: new Date().toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' }),
    };

    const newMessages = [...messages, userMsg];
    setMessages(newMessages);
    setInput('');
    setIsLoading(true);
    setStreamingText('');

    // 워크플로우 노드 시작
    startNode('ai-model');

    const chatHistory = newMessages.map(m => ({ role: m.role, content: m.content }));

    if (provider === 'anthropic') {
      const result = await api.aiChatClaude(chatHistory);
      if (result.error) {
        setMessages(prev => [...prev, {
          id: `msg_${++msgIdCounter.current}`,
          role: 'assistant',
          content: `오류: ${result.error}`,
          time: new Date().toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' }),
        }]);
        setIsLoading(false);
      }
      // Streaming result is handled by the listener above
    } else {
      const result = await api.aiChatOpenAI(chatHistory);
      if (result.error) {
        setMessages(prev => [...prev, {
          id: `msg_${++msgIdCounter.current}`,
          role: 'assistant',
          content: `오류: ${result.error}`,
          time: new Date().toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' }),
        }]);
        setIsLoading(false);
      }
    }
  }, [input, messages, isLoading, provider]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  if (!isOpen) {
    return (
      <button onClick={() => setIsOpen(true)} className="chat-widget-trigger">
        <MessageCircle size={22} />
        <span>AI 채팅</span>
        {messages.length > 0 && <span className="chat-widget-badge">{messages.length}</span>}
      </button>
    );
  }

  return (
    <div className="chat-widget">
      {/* Header */}
      <div className="chat-widget-header">
        <div className="flex items-center gap-2">
          <Bot size={18} className="text-accent-primary" />
          <span className="chat-widget-header-title">AI 채팅</span>
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value as Provider)}
            className="chat-widget-model-badge"
            style={{ border: 'none', outline: 'none', cursor: 'pointer', background: 'rgba(88,166,255,0.1)' }}
          >
            <option value="anthropic">Claude</option>
            <option value="openai">GPT-4o</option>
          </select>
        </div>
        <div className="flex items-center gap-1">
          <button onClick={() => setShowKeyInput(!showKeyInput)} className="chat-widget-header-btn" title="API 키 설정">
            <Key size={14} style={{ color: hasKey ? 'var(--color-accent-success)' : 'var(--color-text-tertiary)' }} />
          </button>
          <button onClick={() => setIsOpen(false)} className="chat-widget-header-btn">
            <ChevronDown size={18} />
          </button>
        </div>
      </div>

      {/* API Key input */}
      {showKeyInput && (
        <div style={{ padding: '10px 16px', borderBottom: '1px solid var(--color-border-primary)', display: 'flex', gap: '8px' }}>
          <input
            type="password"
            value={keyInput}
            onChange={(e) => setKeyInput(e.target.value)}
            placeholder={`${provider === 'anthropic' ? 'Claude' : 'OpenAI'} API 키 입력...`}
            className="chat-widget-input"
            style={{ flex: 1 }}
            onKeyDown={(e) => e.key === 'Enter' && handleSetKey()}
          />
          <button onClick={handleSetKey} className="chat-widget-send-btn" style={{ width: 'auto', padding: '0 12px', fontSize: '12px' }}>
            저장
          </button>
        </div>
      )}

      {/* Messages */}
      <div className="chat-widget-messages">
        {messages.length === 0 && !streamingText && (
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', gap: '8px', color: 'var(--color-text-tertiary)', fontSize: '13px' }}>
            <Bot size={32} />
            <p>무엇을 만들어 드릴까요?</p>
          </div>
        )}

        {messages.map((msg) => (
          <div key={msg.id}>
            <div className={msg.role === 'user' ? 'chat-widget-msg-user' : 'chat-widget-msg-ai'}>
              {msg.role === 'assistant' && (
                <div className="chat-widget-msg-avatar"><Bot size={14} /></div>
              )}
              <div className={msg.role === 'user' ? 'chat-widget-bubble-user' : 'chat-widget-bubble-ai'}>
                <p style={{ whiteSpace: 'pre-wrap' }}>{msg.content}</p>
                <span className="chat-widget-msg-time">{msg.time}</span>
              </div>
            </div>

            {/* 코드 블록 저장 액션 카드 */}
            {msg.codeBlocks && msg.codeBlocks.length > 0 && codeSaveStates[msg.id] && (
              <CodeSaveCard
                state={codeSaveStates[msg.id]}
                workspacePath={workspacePath}
                onSaveAll={() => handleSaveAll(msg.id)}
                onOpenFile={handleOpenFile}
              />
            )}
          </div>
        ))}

        {/* Streaming response */}
        {streamingText && (
          <div className="chat-widget-msg-ai">
            <div className="chat-widget-msg-avatar"><Bot size={14} /></div>
            <div className="chat-widget-bubble-ai">
              <p style={{ whiteSpace: 'pre-wrap' }}>{streamingText}</p>
            </div>
          </div>
        )}

        {/* Loading indicator */}
        {isLoading && !streamingText && (
          <div className="chat-widget-msg-ai">
            <div className="chat-widget-msg-avatar"><Loader2 size={14} className="animate-spin" /></div>
            <div className="chat-widget-bubble-ai">
              <p style={{ color: 'var(--color-text-tertiary)' }}>생각하고 있습니다...</p>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="chat-widget-input-area">
        <div className="chat-widget-input-row">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={hasKey ? '메시지를 입력하세요...' : 'API 키를 먼저 설정하세요'}
            className="chat-widget-input"
            onKeyDown={handleKeyDown}
            disabled={isLoading || !hasKey}
          />
          <button
            className="chat-widget-send-btn"
            onClick={handleSend}
            disabled={isLoading || !hasKey || !input.trim()}
            style={{ opacity: isLoading || !hasKey || !input.trim() ? 0.5 : 1 }}
          >
            {isLoading ? <Loader2 size={16} className="animate-spin" /> : <SendHorizontal size={16} />}
          </button>
        </div>
      </div>
    </div>
  );
}

// ===========================================
// 코드 저장 액션 카드
// ===========================================
function CodeSaveCard({
  state,
  workspacePath,
  onSaveAll,
  onOpenFile,
}: {
  state: CodeSaveState;
  workspacePath: string | null;
  onSaveAll: () => void;
  onOpenFile: (fullPath: string) => void;
}) {
  const { blocks, results, saving } = state;

  return (
    <div className="chat-code-save-card">
      <div className="chat-code-save-header">
        <FileCode size={16} className="text-accent-primary" />
        <span className="chat-code-save-title">
          파일 저장 ({blocks.length}개 파일)
        </span>
      </div>

      <div className="chat-code-save-list">
        {blocks.map((block, i) => {
          const result = results?.[i];
          const fullPath = workspacePath ? `${workspacePath}/${block.filePath}` : block.filePath;

          return (
            <div key={block.filePath} className="chat-code-save-item">
              <div className="flex items-center gap-2 flex-1 min-w-0">
                {result ? (
                  result.success ? (
                    <Check size={14} className="shrink-0 text-accent-success" />
                  ) : (
                    <AlertCircle size={14} className="shrink-0 text-accent-error" />
                  )
                ) : (
                  <FileCode size={14} className="shrink-0 text-text-tertiary" />
                )}
                <span className="chat-code-save-filepath">{block.filePath}</span>
                <span className="chat-code-save-lang">{block.language}</span>
              </div>
              {result?.success && (
                <button
                  onClick={() => onOpenFile(fullPath)}
                  className="chat-code-save-open-btn"
                  title="파일 열기"
                >
                  <FolderOpen size={12} />
                  <span>열기</span>
                </button>
              )}
            </div>
          );
        })}
      </div>

      {!results && (
        <button
          onClick={onSaveAll}
          disabled={saving || !workspacePath}
          className="chat-code-save-btn"
        >
          {saving ? (
            <>
              <Loader2 size={14} className="animate-spin" />
              저장 중...
            </>
          ) : !workspacePath ? (
            <>
              <AlertCircle size={14} />
              워크스페이스를 먼저 열어주세요
            </>
          ) : (
            <>
              <Save size={14} />
              모두 저장
            </>
          )}
        </button>
      )}

      {results && (
        <div className="chat-code-save-result">
          {results.every((r) => r.success) ? (
            <span className="text-accent-success flex items-center gap-1.5">
              <Check size={14} />
              모든 파일 저장 완료
            </span>
          ) : (
            <span className="text-accent-error flex items-center gap-1.5">
              <AlertCircle size={14} />
              {results.filter((r) => !r.success).length}개 파일 저장 실패
            </span>
          )}
        </div>
      )}
    </div>
  );
}
