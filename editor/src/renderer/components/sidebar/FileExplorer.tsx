import React, { useState, useCallback } from 'react';
import {
  ChevronRight,
  ChevronDown,
  File,
  FolderOpen,
  Folder,
  RefreshCw,
  FolderPlus,
} from 'lucide-react';
import { useFileStore, FileNode } from '../../stores/fileStore';

function getFileColor(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase() || '';
  const colors: Record<string, string> = {
    tsx: '#58a6ff', ts: '#58a6ff', jsx: '#58a6ff', js: '#d29922',
    json: '#d29922', css: '#bc8cff', scss: '#bc8cff',
    html: '#f85149', md: '#8b949e', py: '#3fb950',
    go: '#79c0ff', rs: '#f85149', java: '#f85149',
    env: '#3fb950', gitignore: '#8b949e', yml: '#f85149', yaml: '#f85149',
    toml: '#8b949e', lock: '#8b949e', svg: '#d29922',
  };
  return colors[ext] || '#8b949e';
}

function TreeItem({
  node,
  depth,
  activeFilePath,
  onFileClick,
}: {
  node: FileNode;
  depth: number;
  activeFilePath: string | null;
  onFileClick: (path: string) => void;
}) {
  const [expanded, setExpanded] = useState(depth < 1);
  const isDir = node.type === 'directory';
  const isActive = node.path === activeFilePath;

  const handleClick = () => {
    if (isDir) {
      setExpanded(!expanded);
    } else {
      onFileClick(node.path);
    }
  };

  return (
    <>
      <div
        className={`file-tree-item ${isActive ? 'file-tree-item-selected' : ''}`}
        style={{ paddingLeft: `${12 + depth * 16}px` }}
        onClick={handleClick}
      >
        {isDir ? (
          expanded ? <ChevronDown size={14} className="file-tree-chevron" /> : <ChevronRight size={14} className="file-tree-chevron" />
        ) : (
          <span style={{ width: 14, flexShrink: 0 }} />
        )}

        {isDir ? (
          expanded ? <FolderOpen size={15} className="file-tree-folder" /> : <Folder size={15} className="file-tree-folder" />
        ) : (
          <File size={14} style={{ color: getFileColor(node.name), flexShrink: 0 }} />
        )}

        <span className="file-tree-name">{node.name}</span>
      </div>

      {isDir && expanded && node.children?.map((child) => (
        <TreeItem key={child.path} node={child} depth={depth + 1} activeFilePath={activeFilePath} onFileClick={onFileClick} />
      ))}
    </>
  );
}

export function FileExplorer() {
  const { workspacePath, fileTree, loading, openFile, activeFilePath, openFolder, refreshTree } = useFileStore();

  const handleFileClick = useCallback((filePath: string) => {
    openFile(filePath);
  }, [openFile]);

  if (!workspacePath) {
    return (
      <div className="file-tree" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', padding: '24px', gap: '16px' }}>
        <FolderOpen size={40} style={{ color: 'var(--color-text-tertiary)' }} />
        <p style={{ color: 'var(--color-text-tertiary)', fontSize: '13px', textAlign: 'center' }}>
          폴더를 열어서 시작하세요
        </p>
        <button onClick={openFolder} className="btn-primary" style={{ padding: '10px 20px', fontSize: '13px' }}>
          <FolderPlus size={16} />
          폴더 열기
        </button>
      </div>
    );
  }

  return (
    <div className="file-tree">
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '6px 12px', borderBottom: '1px solid var(--color-border-primary)',
      }}>
        <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--color-text-tertiary)', textTransform: 'uppercase', letterSpacing: '0.5px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {workspacePath.split('/').pop()}
        </span>
        <button
          onClick={refreshTree}
          style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '24px', height: '24px', borderRadius: '6px', color: 'var(--color-text-tertiary)', background: 'none', border: 'none', cursor: 'pointer' }}
          title="새로고침"
        >
          <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      <div style={{ overflow: 'auto', flex: 1 }}>
        {fileTree.map((node) => (
          <TreeItem key={node.path} node={node} depth={0} activeFilePath={activeFilePath} onFileClick={handleFileClick} />
        ))}
      </div>
    </div>
  );
}

export default FileExplorer;
