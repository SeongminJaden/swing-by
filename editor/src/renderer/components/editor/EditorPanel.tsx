import { useCallback, useEffect } from 'react';
import Editor from '@monaco-editor/react';
import {
  X,
  FileCode2,
  MoreHorizontal,
  FileText as FileTextIcon,
  FolderOpen,
  Bot,
} from 'lucide-react';
import { useFileStore } from '../../stores/fileStore';

function getFileIcon(name: string) {
  const ext = name.split('.').pop()?.toLowerCase() || '';
  if (['tsx', 'ts', 'jsx', 'js'].includes(ext)) return <FileCode2 size={14} className="text-accent-primary" />;
  return <FileTextIcon size={14} style={{ color: '#8b949e' }} />;
}

export function EditorPanel() {
  const { openFiles, activeFilePath, setActiveFile, closeFile, updateFileContent, saveActiveFile, openFolder } = useFileStore();

  const activeFile = openFiles.find(f => f.path === activeFilePath);

  // Ctrl+S to save
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        saveActiveFile();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [saveActiveFile]);

  const handleEditorChange = useCallback((value: string | undefined) => {
    if (activeFilePath && value !== undefined) {
      updateFileContent(activeFilePath, value);
    }
  }, [activeFilePath, updateFileContent]);

  // No files open → welcome screen
  if (openFiles.length === 0) {
    return (
      <div className="editor-panel">
        <div className="editor-welcome">
          <div className="editor-welcome-icon">
            <Bot size={40} className="text-accent-primary" />
          </div>
          <h2 className="editor-welcome-title">VidEplace</h2>
          <p className="editor-welcome-desc">
            파일을 선택하거나 AI에게 코드 생성을 요청하세요
          </p>
          <button
            onClick={openFolder}
            className="btn-primary"
            style={{ marginTop: '20px', padding: '10px 24px', fontSize: '13px' }}
          >
            <FolderOpen size={16} />
            폴더 열기
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="editor-panel">
      {/* Tab bar */}
      <div className="editor-tab-bar">
        {openFiles.map((file) => (
          <div
            key={file.path}
            className={activeFilePath === file.path ? 'editor-tab-active' : 'editor-tab'}
            onClick={() => setActiveFile(file.path)}
          >
            {getFileIcon(file.name)}
            <span className="editor-tab-label">{file.name}</span>
            {file.modified && <span className="editor-tab-modified" />}
            <button
              className="editor-tab-close"
              onClick={(e) => { e.stopPropagation(); closeFile(file.path); }}
            >
              <X size={13} />
            </button>
          </div>
        ))}
        <div className="editor-tab-actions" />
      </div>

      {/* Breadcrumb */}
      {activeFile && (
        <div className="editor-breadcrumb">
          {activeFile.path.split('/').slice(-3).map((seg, i, arr) => (
            <span key={i} className="flex items-center">
              {i > 0 && <span className="editor-breadcrumb-sep" style={{ margin: '0 4px' }}>&rsaquo;</span>}
              <span className={i === arr.length - 1 ? 'editor-breadcrumb-current' : 'editor-breadcrumb-segment'}>
                {seg}
              </span>
            </span>
          ))}
        </div>
      )}

      {/* Monaco Editor */}
      <div style={{ flex: 1, overflow: 'hidden' }}>
        {activeFile ? (
          <Editor
            key={activeFile.path}
            defaultValue={activeFile.content}
            language={activeFile.language}
            theme="vs-dark"
            onChange={handleEditorChange}
            options={{
              fontSize: 14,
              fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
              fontLigatures: true,
              lineNumbers: 'on',
              minimap: { enabled: true },
              scrollBeyondLastLine: false,
              wordWrap: 'off',
              tabSize: 2,
              renderWhitespace: 'selection',
              bracketPairColorization: { enabled: true },
              cursorBlinking: 'smooth',
              smoothScrolling: true,
              padding: { top: 12 },
              automaticLayout: true,
            }}
          />
        ) : null}
      </div>

      {/* Status bar */}
      <div className="editor-status-bar">
        <div className="editor-status-bar-group">
          {activeFile && (
            <>
              <span className="editor-status-bar-item">{activeFile.language}</span>
              <span className="editor-status-bar-divider" />
              <span className="editor-status-bar-item">UTF-8</span>
              <span className="editor-status-bar-divider" />
              <span className="editor-status-bar-item">Spaces: 2</span>
            </>
          )}
        </div>
        <div className="editor-status-bar-group">
          {activeFile?.modified && (
            <span className="editor-status-bar-item editor-status-bar-item-accent">수정됨 (Ctrl+S로 저장)</span>
          )}
        </div>
      </div>
    </div>
  );
}

export default EditorPanel;
