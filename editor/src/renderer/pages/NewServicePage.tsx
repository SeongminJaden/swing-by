import { useState, useEffect, useRef } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Loader2,
  FolderOpen,
  Sparkles,
  FileCode,
  Rocket,
  Globe,
  Server,
  Zap,
  Database,
  Key,
  Layout,
} from 'lucide-react';
import { useAppStore } from '../stores/appStore';
import { useFileStore } from '../stores/fileStore';
import { useWorkflowStore } from '../stores/workflowStore';
import { parseCodeBlocks } from '../utils/codeParser';
import { writeCodeBlocks, WriteResult } from '../services/codeGenService';

// ===========================================
// Types
// ===========================================

interface FrameworkOption {
  id: string;
  name: string;
  description: string;
  icon: React.ReactNode;
  color: string;
}

interface DbOption {
  id: string;
  name: string;
}

interface AuthOption {
  id: string;
  name: string;
}

type Step = 1 | 2 | 3 | 4;

const STEP_LABELS = ['기본 정보', '기술 스택 선택', 'AI PRD 생성', '생성 중'];

// ===========================================
// Data
// ===========================================

const frameworks: FrameworkOption[] = [
  { id: 'nextjs', name: 'Next.js', description: 'React, SSR, App Router', icon: <Globe size={24} />, color: 'bg-accent-primary/15 text-accent-primary' },
  { id: 'react-vite', name: 'React + Vite', description: 'SPA, 빠른 HMR', icon: <Zap size={24} />, color: 'bg-accent-warning/15 text-accent-warning' },
  { id: 'astro', name: 'Astro', description: 'Static Site, Islands', icon: <Sparkles size={24} />, color: 'bg-accent-purple/15 text-accent-purple' },
  { id: 'express', name: 'Express.js', description: 'Node.js API Server', icon: <Server size={24} />, color: 'bg-accent-success/15 text-accent-success' },
  { id: 'flask', name: 'Flask', description: 'Python API', icon: <Server size={24} />, color: 'bg-accent-info/15 text-accent-info' },
  { id: 'fastapi', name: 'FastAPI', description: 'Python Async API', icon: <Zap size={24} />, color: 'bg-accent-error/15 text-accent-error' },
];

const databases: DbOption[] = [
  { id: 'supabase', name: 'Supabase' },
  { id: 'firebase', name: 'Firebase' },
  { id: 'postgresql', name: 'PostgreSQL' },
  { id: 'mongodb', name: 'MongoDB' },
  { id: 'none', name: 'None' },
];

const authOptions: AuthOption[] = [
  { id: 'nextauth', name: 'NextAuth' },
  { id: 'clerk', name: 'Clerk' },
  { id: 'auth0', name: 'Auth0' },
  { id: 'none', name: 'None' },
];

// ===========================================
// Component
// ===========================================

export default function NewServicePage() {
  const { setCurrentView } = useAppStore();
  const { loadFolder } = useFileStore();
  const { resetAll, startNode, completeNode, setNodeStatus, setNodeProgress } = useWorkflowStore();

  // Step state
  const [step, setStep] = useState<Step>(1);

  // Step 1: 기본 정보
  const [serviceName, setServiceName] = useState('');
  const [serviceDesc, setServiceDesc] = useState('');
  const [targetDir, setTargetDir] = useState('');

  // Step 2: 기술 스택
  const [selectedFramework, setSelectedFramework] = useState<string | null>(null);
  const [selectedDb, setSelectedDb] = useState<string>('none');
  const [selectedAuth, setSelectedAuth] = useState<string>('none');

  // Step 3: PRD
  const [prdText, setPrdText] = useState('');
  const [generatingPrd, setGeneratingPrd] = useState(false);

  // Step 4: 생성 중
  const [generating, setGenerating] = useState(false);
  const [progress, setProgress] = useState(0);
  const [createdFiles, setCreatedFiles] = useState<string[]>([]);
  const [completed, setCompleted] = useState(false);

  const streamingRef = useRef('');

  // 폴더 선택
  const handleSelectDir = async () => {
    const api = window.electronAPI;
    if (!api) return;
    const dir = await api.openFolder();
    if (dir) setTargetDir(dir);
  };

  // Step 유효성 검사
  const canGoNext = (): boolean => {
    switch (step) {
      case 1:
        return serviceName.trim().length > 0 && targetDir.trim().length > 0;
      case 2:
        return selectedFramework !== null;
      case 3:
        return prdText.trim().length > 0;
      default:
        return false;
    }
  };

  // PRD 생성
  const generatePRD = async () => {
    setGeneratingPrd(true);

    const frameworkInfo = frameworks.find((f) => f.id === selectedFramework);
    const dbInfo = databases.find((d) => d.id === selectedDb);
    const authInfo = authOptions.find((a) => a.id === selectedAuth);

    const prd = `# PRD: ${serviceName}

## 개요
${serviceDesc || `${serviceName} 서비스`}

## 기술 스택
- **프레임워크**: ${frameworkInfo?.name || 'N/A'} (${frameworkInfo?.description || ''})
- **데이터베이스**: ${dbInfo?.name || 'None'}
- **인증**: ${authInfo?.name || 'None'}

## 프로젝트 구조
${getProjectStructure(selectedFramework || '', selectedDb, selectedAuth)}

## 주요 기능
1. 프로젝트 초기 설정 및 구성 파일 생성
2. 기본 라우팅 및 페이지 구조
3. ${selectedDb !== 'none' ? `${dbInfo?.name} 데이터베이스 연결 설정` : '기본 데이터 레이어'}
4. ${selectedAuth !== 'none' ? `${authInfo?.name} 인증 시스템 통합` : '기본 레이아웃 구성'}
5. 스타일링 및 UI 컴포넌트
6. 개발 환경 설정 (ESLint, TypeScript)

## 생성될 파일 목록
${getFileList(selectedFramework || '', selectedDb, selectedAuth)}

## 배포 대상
- 대상 디렉토리: ${targetDir}/${toKebabCase(serviceName)}
`;

    setPrdText(prd);
    setGeneratingPrd(false);
  };

  // Step 3 진입 시 PRD 자동 생성
  useEffect(() => {
    if (step === 3 && !prdText) {
      generatePRD();
    }
  }, [step]);

  // 코드 생성 시작
  const startGeneration = async () => {
    setStep(4);
    setGenerating(true);
    setProgress(0);
    setCreatedFiles([]);
    setCompleted(false);

    // 워크플로우 초기화 및 시작
    resetAll();
    startNode('prd');

    const projectDir = `${targetDir}/${toKebabCase(serviceName)}`;

    // PRD 노드 완료
    await simulateDelay(800);
    completeNode('prd');
    setProgress(10);

    // 코드 생성 노드 시작
    startNode('codegen');
    startNode('ai-model');
    setNodeStatus('codegen', 'in-progress', '코드 생성 요청 중...');

    const api = window.electronAPI;
    if (!api) {
      setNodeStatus('codegen', 'error', 'electronAPI를 사용할 수 없습니다');
      setGenerating(false);
      return;
    }

    // AI에게 코드 생성 요청
    const prompt = buildCodeGenPrompt(
      serviceName,
      serviceDesc,
      selectedFramework || '',
      selectedDb,
      selectedAuth,
      prdText
    );

    // 스트리밍 수신 설정
    streamingRef.current = '';
    let fileCount = 0;

    api.onAIStream((text: string) => {
      streamingRef.current += text;

      // 파일 경로 패턴 감지하여 실시간 업데이트
      const blocks = parseCodeBlocks(streamingRef.current);
      if (blocks.length > fileCount) {
        const newFiles = blocks.slice(fileCount).map((b) => b.filePath);
        setCreatedFiles((prev) => [...prev, ...newFiles]);
        fileCount = blocks.length;
        const newProgress = Math.min(10 + (fileCount / 15) * 70, 80);
        setProgress(newProgress);
        setNodeProgress('codegen', newProgress);
        setNodeStatus('codegen', 'in-progress', `${fileCount}개 파일 생성 중...`);
      }
    });

    api.onAIStreamEnd(async () => {
      completeNode('ai-model');
      setProgress(80);

      // 코드 블록 파싱
      const codeBlocks = parseCodeBlocks(streamingRef.current);
      setNodeStatus('codegen', 'completed', `${codeBlocks.length}개 파일 생성 완료`);
      completeNode('codegen');

      // 파일 시스템 쓰기
      startNode('filesystem');
      setProgress(85);

      if (codeBlocks.length > 0) {
        const results = await writeCodeBlocks(codeBlocks, projectDir);
        const successCount = results.filter((r) => r.success).length;

        setCreatedFiles(codeBlocks.map((b) => b.filePath));
        setNodeStatus('filesystem', 'completed', `${successCount}/${codeBlocks.length}개 파일 저장`);
        completeNode('filesystem');
      } else {
        // AI가 코드 블록을 생성하지 않은 경우 기본 파일 생성
        await generateFallbackFiles(projectDir);
        completeNode('filesystem');
      }

      setProgress(100);
      setGenerating(false);
      setCompleted(true);
    });

    // AI 요청
    const messages = [{ role: 'user', content: prompt }];

    try {
      const hasAnthropicKey = await api.aiHasKey('anthropic');
      const hasOpenAIKey = await api.aiHasKey('openai');

      if (hasAnthropicKey) {
        const result = await api.aiChatClaude(messages);
        if (result.error) {
          // 스트리밍 에러 - 폴백으로 기본 파일 생성
          await handleGenerationFallback(projectDir);
        }
      } else if (hasOpenAIKey) {
        const result = await api.aiChatOpenAI(messages);
        if (result.error) {
          await handleGenerationFallback(projectDir);
        }
      } else {
        // API 키 없음 - 폴백으로 기본 파일 생성
        await handleGenerationFallback(projectDir);
      }
    } catch {
      await handleGenerationFallback(projectDir);
    }
  };

  // API 키 없는 경우의 폴백 생성
  const handleGenerationFallback = async (projectDir: string) => {
    completeNode('ai-model');
    setNodeStatus('codegen', 'in-progress', '템플릿 기반 생성 중...');

    await generateFallbackFiles(projectDir);

    completeNode('codegen');
    completeNode('filesystem');
    setProgress(100);
    setGenerating(false);
    setCompleted(true);
  };

  // 템플릿 기반 기본 파일 생성 (AI 없이)
  const generateFallbackFiles = async (projectDir: string) => {
    const api = window.electronAPI;
    if (!api) return;

    const files = getFallbackFiles(selectedFramework || '', serviceName, selectedDb, selectedAuth);

    for (const file of files) {
      const fullPath = `${projectDir}/${file.path}`;
      const dirPath = fullPath.substring(0, fullPath.lastIndexOf('/'));
      await api.mkdir(dirPath);
      await api.writeFile(fullPath, file.content);
      setCreatedFiles((prev) => [...prev, file.path]);
    }

    // 파일 트리 새로고침
    try {
      await useFileStore.getState().refreshTree();
    } catch {
      // ignore
    }
  };

  // 프로젝트 열기
  const handleOpenProject = async () => {
    const projectDir = `${targetDir}/${toKebabCase(serviceName)}`;
    await loadFolder(projectDir);
    setCurrentView('ide');
  };

  return (
    <div className="newservice-container">
      <div className="newservice-inner">
        {/* Step Indicator */}
        <div className="newservice-steps">
          {STEP_LABELS.map((label, i) => {
            const stepNum = (i + 1) as Step;
            const isActive = step === stepNum;
            const isCompleted = step > stepNum;

            return (
              <div key={label} className="newservice-step">
                <div className="flex items-center gap-3">
                  <div
                    className={`newservice-step-dot ${
                      isCompleted
                        ? 'newservice-step-dot-completed'
                        : isActive
                        ? 'newservice-step-dot-active'
                        : 'newservice-step-dot-inactive'
                    }`}
                  >
                    {isCompleted ? <Check size={18} /> : stepNum}
                  </div>
                  <span
                    className={`newservice-step-label ${
                      isActive || isCompleted ? 'newservice-step-label-active' : 'newservice-step-label-inactive'
                    }`}
                  >
                    {label}
                  </span>
                </div>
                {i < STEP_LABELS.length - 1 && (
                  <div
                    className={`newservice-step-line ${
                      isCompleted ? 'newservice-step-line-active' : 'newservice-step-line-inactive'
                    }`}
                  />
                )}
              </div>
            );
          })}
        </div>

        {/* Step Content */}
        <div className="newservice-step-content">
          {step === 1 && (
            <StepBasicInfo
              serviceName={serviceName}
              setServiceName={setServiceName}
              serviceDesc={serviceDesc}
              setServiceDesc={setServiceDesc}
              targetDir={targetDir}
              setTargetDir={setTargetDir}
              onSelectDir={handleSelectDir}
            />
          )}

          {step === 2 && (
            <StepTechStack
              selectedFramework={selectedFramework}
              setSelectedFramework={setSelectedFramework}
              selectedDb={selectedDb}
              setSelectedDb={setSelectedDb}
              selectedAuth={selectedAuth}
              setSelectedAuth={setSelectedAuth}
            />
          )}

          {step === 3 && (
            <StepPRDPreview
              prdText={prdText}
              generatingPrd={generatingPrd}
            />
          )}

          {step === 4 && (
            <StepGenerating
              generating={generating}
              progress={progress}
              createdFiles={createdFiles}
              completed={completed}
              onOpenProject={handleOpenProject}
            />
          )}
        </div>

        {/* Navigation */}
        {step < 4 && (
          <div className="newservice-nav">
            <button
              onClick={() => {
                if (step === 1) {
                  setCurrentView('ide');
                } else {
                  setStep((s) => (s - 1) as Step);
                }
              }}
              className="newservice-btn-back"
            >
              <ArrowLeft size={18} />
              {step === 1 ? '대시보드로' : '이전'}
            </button>

            {step < 3 && (
              <button
                onClick={() => setStep((s) => (s + 1) as Step)}
                disabled={!canGoNext()}
                className="newservice-btn-next"
              >
                다음
                <ArrowRight size={18} />
              </button>
            )}

            {step === 3 && (
              <button
                onClick={startGeneration}
                disabled={!canGoNext() || generatingPrd}
                className="newservice-btn-start"
              >
                <Rocket size={18} />
                생성 시작
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ===========================================
// Step 1: 기본 정보
// ===========================================
function StepBasicInfo({
  serviceName,
  setServiceName,
  serviceDesc,
  setServiceDesc,
  targetDir,
  setTargetDir,
  onSelectDir,
}: {
  serviceName: string;
  setServiceName: (v: string) => void;
  serviceDesc: string;
  setServiceDesc: (v: string) => void;
  targetDir: string;
  setTargetDir: (v: string) => void;
  onSelectDir: () => void;
}) {
  return (
    <>
      <h2 className="newservice-step-title">기본 정보</h2>
      <p className="newservice-step-subtitle">새 서비스의 이름과 설명, 저장 위치를 입력하세요.</p>

      <div className="newservice-field">
        <label className="newservice-label">서비스 이름</label>
        <input
          type="text"
          value={serviceName}
          onChange={(e) => setServiceName(e.target.value)}
          placeholder="예: my-awesome-app"
          className="newservice-input"
          autoFocus
        />
      </div>

      <div className="newservice-field">
        <label className="newservice-label">서비스 설명 (선택)</label>
        <textarea
          value={serviceDesc}
          onChange={(e) => setServiceDesc(e.target.value)}
          placeholder="이 서비스가 무엇을 하는지 설명하세요..."
          className="newservice-textarea"
        />
      </div>

      <div className="newservice-field">
        <label className="newservice-label">대상 디렉토리</label>
        <div className="newservice-dir-row">
          <input
            type="text"
            value={targetDir}
            onChange={(e) => setTargetDir(e.target.value)}
            placeholder="/home/user/projects"
            className="newservice-dir-input"
          />
          <button onClick={onSelectDir} className="newservice-dir-btn">
            <FolderOpen size={18} />
            폴더 선택
          </button>
        </div>
      </div>
    </>
  );
}

// ===========================================
// Step 2: 기술 스택 선택
// ===========================================
function StepTechStack({
  selectedFramework,
  setSelectedFramework,
  selectedDb,
  setSelectedDb,
  selectedAuth,
  setSelectedAuth,
}: {
  selectedFramework: string | null;
  setSelectedFramework: (v: string) => void;
  selectedDb: string;
  setSelectedDb: (v: string) => void;
  selectedAuth: string;
  setSelectedAuth: (v: string) => void;
}) {
  return (
    <>
      <h2 className="newservice-step-title">기술 스택 선택</h2>
      <p className="newservice-step-subtitle">프레임워크와 데이터베이스, 인증 방식을 선택하세요.</p>

      <p className="newservice-section-label">프레임워크</p>
      <div className="newservice-card-grid">
        {frameworks.map((fw) => (
          <button
            key={fw.id}
            onClick={() => setSelectedFramework(fw.id)}
            className={`newservice-tech-card relative ${
              selectedFramework === fw.id ? 'newservice-tech-card-selected' : ''
            }`}
          >
            {selectedFramework === fw.id && (
              <div className="newservice-tech-card-check">
                <Check size={14} />
              </div>
            )}
            <div className={`newservice-tech-card-icon ${fw.color}`}>
              {fw.icon}
            </div>
            <div className="newservice-tech-card-name">{fw.name}</div>
            <div className="newservice-tech-card-desc">{fw.description}</div>
          </button>
        ))}
      </div>

      <p className="newservice-section-label">데이터베이스 (선택)</p>
      <div className="flex flex-wrap gap-3">
        {databases.map((db) => (
          <button
            key={db.id}
            onClick={() => setSelectedDb(db.id)}
            className={`badge ${selectedDb === db.id ? '!bg-accent-primary/15 !text-accent-primary' : ''}`}
            style={{ cursor: 'pointer', transition: 'all 0.15s ease' }}
          >
            {selectedDb === db.id && <Check size={14} style={{ marginRight: '4px' }} />}
            {db.name}
          </button>
        ))}
      </div>

      <p className="newservice-section-label">인증 (선택)</p>
      <div className="flex flex-wrap gap-3">
        {authOptions.map((auth) => (
          <button
            key={auth.id}
            onClick={() => setSelectedAuth(auth.id)}
            className={`badge ${selectedAuth === auth.id ? '!bg-accent-primary/15 !text-accent-primary' : ''}`}
            style={{ cursor: 'pointer', transition: 'all 0.15s ease' }}
          >
            {selectedAuth === auth.id && <Check size={14} style={{ marginRight: '4px' }} />}
            {auth.name}
          </button>
        ))}
      </div>
    </>
  );
}

// ===========================================
// Step 3: PRD 미리보기
// ===========================================
function StepPRDPreview({
  prdText,
  generatingPrd,
}: {
  prdText: string;
  generatingPrd: boolean;
}) {
  return (
    <>
      <h2 className="newservice-step-title">AI PRD 생성</h2>
      <p className="newservice-step-subtitle">
        선택한 정보를 바탕으로 PRD를 생성했습니다. 확인 후 생성을 시작하세요.
      </p>

      {generatingPrd ? (
        <div className="flex items-center justify-center gap-3 py-16">
          <Loader2 size={24} className="animate-spin text-accent-primary" />
          <span className="text-text-secondary font-semibold">PRD 생성 중...</span>
        </div>
      ) : (
        <div className="newservice-prd-preview">
          <pre>{prdText}</pre>
        </div>
      )}
    </>
  );
}

// ===========================================
// Step 4: 생성 중
// ===========================================
function StepGenerating({
  generating,
  progress,
  createdFiles,
  completed,
  onOpenProject,
}: {
  generating: boolean;
  progress: number;
  createdFiles: string[];
  completed: boolean;
  onOpenProject: () => void;
}) {
  return (
    <div className="newservice-progress-container">
      {generating && (
        <>
          <div className="newservice-progress-spinner">
            <Loader2 size={36} className="animate-spin text-accent-primary" />
          </div>
          <div>
            <p className="newservice-progress-text">서비스 생성 중...</p>
            <p className="newservice-progress-sub">{Math.round(progress)}% 완료</p>
          </div>
        </>
      )}

      {completed && (
        <>
          <div className="newservice-progress-spinner" style={{ background: 'rgba(63,185,80,0.1)' }}>
            <Check size={36} className="text-accent-success" />
          </div>
          <div>
            <p className="newservice-progress-text">서비스 생성 완료!</p>
            <p className="newservice-progress-sub">{createdFiles.length}개 파일이 생성되었습니다</p>
          </div>
        </>
      )}

      {/* Progress bar */}
      <div className="newservice-progress-bar-track">
        <div className="newservice-progress-bar-fill" style={{ width: `${progress}%` }} />
      </div>

      {/* File list */}
      {createdFiles.length > 0 && (
        <div className="newservice-file-list">
          {createdFiles.map((file, i) => (
            <div
              key={`${file}-${i}`}
              className={`newservice-file-item ${
                completed ? 'newservice-file-item-success' : 'newservice-file-item-pending'
              }`}
            >
              {completed ? <Check size={14} /> : <FileCode size={14} />}
              <span>{file}</span>
            </div>
          ))}
        </div>
      )}

      {/* Open project button */}
      {completed && (
        <button onClick={onOpenProject} className="newservice-btn-next" style={{ marginTop: '16px' }}>
          <Layout size={18} />
          IDE에서 프로젝트 열기
        </button>
      )}
    </div>
  );
}

// ===========================================
// Helpers
// ===========================================

function toKebabCase(str: string): string {
  return str
    .toLowerCase()
    .replace(/[^a-z0-9가-힣\s-]/g, '')
    .replace(/[\s]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '') || 'new-project';
}

function simulateDelay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function getProjectStructure(framework: string, db: string, auth: string): string {
  const base = framework === 'nextjs'
    ? `\`\`\`
src/
  app/
    layout.tsx
    page.tsx
    globals.css
  components/
  lib/
public/
package.json
tsconfig.json
next.config.js
\`\`\``
    : framework === 'react-vite'
    ? `\`\`\`
src/
  App.tsx
  main.tsx
  index.css
  components/
  pages/
public/
package.json
tsconfig.json
vite.config.ts
\`\`\``
    : framework === 'astro'
    ? `\`\`\`
src/
  pages/
    index.astro
  layouts/
  components/
public/
package.json
astro.config.mjs
\`\`\``
    : framework === 'express'
    ? `\`\`\`
src/
  index.ts
  routes/
  middleware/
  controllers/
package.json
tsconfig.json
\`\`\``
    : framework === 'flask'
    ? `\`\`\`
app/
  __init__.py
  routes.py
  models.py
templates/
static/
requirements.txt
\`\`\``
    : framework === 'fastapi'
    ? `\`\`\`
app/
  main.py
  routes/
  models/
  schemas/
requirements.txt
\`\`\``
    : '(프레임워크 선택 필요)';

  return base;
}

function getFileList(framework: string, db: string, auth: string): string {
  const files: string[] = [];

  if (framework === 'nextjs') {
    files.push('package.json', 'tsconfig.json', 'next.config.js', 'tailwind.config.ts', 'postcss.config.js');
    files.push('src/app/layout.tsx', 'src/app/page.tsx', 'src/app/globals.css');
    if (db !== 'none') files.push('src/lib/db.ts');
    if (auth !== 'none') files.push('src/lib/auth.ts', 'src/app/api/auth/[...nextauth]/route.ts');
  } else if (framework === 'react-vite') {
    files.push('package.json', 'tsconfig.json', 'vite.config.ts', 'index.html');
    files.push('src/App.tsx', 'src/main.tsx', 'src/index.css');
  } else if (framework === 'astro') {
    files.push('package.json', 'astro.config.mjs', 'tsconfig.json');
    files.push('src/pages/index.astro', 'src/layouts/Layout.astro');
  } else if (framework === 'express') {
    files.push('package.json', 'tsconfig.json');
    files.push('src/index.ts', 'src/routes/index.ts');
  } else if (framework === 'flask') {
    files.push('requirements.txt', 'app/__init__.py', 'app/routes.py');
  } else if (framework === 'fastapi') {
    files.push('requirements.txt', 'app/main.py', 'app/routes/__init__.py');
  }

  return files.map((f) => `- \`${f}\``).join('\n');
}

function buildCodeGenPrompt(
  name: string,
  desc: string,
  framework: string,
  db: string,
  auth: string,
  prd: string,
): string {
  const frameworkInfo = frameworks.find((f) => f.id === framework);

  return `You are VidEplace AI. Generate a complete project based on this PRD.

Project: ${name}
Description: ${desc || name}
Framework: ${frameworkInfo?.name || framework}
Database: ${db !== 'none' ? db : 'None'}
Auth: ${auth !== 'none' ? auth : 'None'}

PRD:
${prd}

IMPORTANT: For each file, use this EXACT format:
\`\`\`filepath:path/to/file.ext
file contents here
\`\`\`

Generate ALL files needed for a working project. Include:
1. Package configuration (package.json / requirements.txt)
2. TypeScript / build config
3. Main entry points
4. Basic routes/pages
5. Styling
6. Database connection (if selected)
7. Auth setup (if selected)
8. README.md with setup instructions

Make sure every file is complete and working. Use modern best practices.`;
}

function getFallbackFiles(
  framework: string,
  name: string,
  db: string,
  auth: string,
): { path: string; content: string }[] {
  const kebabName = name.toLowerCase().replace(/[^a-z0-9-]/g, '-').replace(/-+/g, '-') || 'new-project';

  if (framework === 'nextjs') {
    return [
      {
        path: 'package.json',
        content: JSON.stringify({
          name: kebabName,
          version: '0.1.0',
          private: true,
          scripts: {
            dev: 'next dev',
            build: 'next build',
            start: 'next start',
            lint: 'next lint',
          },
          dependencies: {
            next: '^14.0.0',
            react: '^18.0.0',
            'react-dom': '^18.0.0',
          },
          devDependencies: {
            typescript: '^5.0.0',
            '@types/react': '^18.0.0',
            '@types/node': '^20.0.0',
          },
        }, null, 2) + '\n',
      },
      {
        path: 'tsconfig.json',
        content: JSON.stringify({
          compilerOptions: {
            target: 'es5',
            lib: ['dom', 'dom.iterable', 'esnext'],
            allowJs: true,
            skipLibCheck: true,
            strict: true,
            noEmit: true,
            esModuleInterop: true,
            module: 'esnext',
            moduleResolution: 'bundler',
            resolveJsonModule: true,
            isolatedModules: true,
            jsx: 'preserve',
            incremental: true,
            paths: { '@/*': ['./src/*'] },
          },
          include: ['next-env.d.ts', '**/*.ts', '**/*.tsx'],
          exclude: ['node_modules'],
        }, null, 2) + '\n',
      },
      {
        path: 'next.config.js',
        content: `/** @type {import('next').NextConfig} */
const nextConfig = {};
module.exports = nextConfig;
`,
      },
      {
        path: 'src/app/layout.tsx',
        content: `import './globals.css';

export const metadata = {
  title: '${name}',
  description: '${name} - Built with VidEplace',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="ko">
      <body>{children}</body>
    </html>
  );
}
`,
      },
      {
        path: 'src/app/page.tsx',
        content: `export default function Home() {
  return (
    <main style={{ padding: '2rem', textAlign: 'center' }}>
      <h1>${name}</h1>
      <p>VidEplace로 생성된 프로젝트입니다.</p>
    </main>
  );
}
`,
      },
      {
        path: 'src/app/globals.css',
        content: `* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  color: #333;
}
`,
      },
      {
        path: 'README.md',
        content: `# ${name}

VidEplace로 생성된 Next.js 프로젝트입니다.

## 시작하기

\`\`\`bash
npm install
npm run dev
\`\`\`

http://localhost:3000 에서 확인하세요.
`,
      },
    ];
  }

  if (framework === 'react-vite') {
    return [
      {
        path: 'package.json',
        content: JSON.stringify({
          name: kebabName,
          version: '0.1.0',
          private: true,
          type: 'module',
          scripts: {
            dev: 'vite',
            build: 'tsc && vite build',
            preview: 'vite preview',
          },
          dependencies: {
            react: '^18.0.0',
            'react-dom': '^18.0.0',
          },
          devDependencies: {
            typescript: '^5.0.0',
            vite: '^5.0.0',
            '@vitejs/plugin-react': '^4.0.0',
            '@types/react': '^18.0.0',
            '@types/react-dom': '^18.0.0',
          },
        }, null, 2) + '\n',
      },
      {
        path: 'vite.config.ts',
        content: `import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});
`,
      },
      {
        path: 'index.html',
        content: `<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>${name}</title>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="/src/main.tsx"></script>
</body>
</html>
`,
      },
      {
        path: 'src/main.tsx',
        content: `import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
`,
      },
      {
        path: 'src/App.tsx',
        content: `function App() {
  return (
    <div style={{ padding: '2rem', textAlign: 'center' }}>
      <h1>${name}</h1>
      <p>VidEplace로 생성된 React + Vite 프로젝트입니다.</p>
    </div>
  );
}

export default App;
`,
      },
      {
        path: 'src/index.css',
        content: `* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; color: #333; }
`,
      },
    ];
  }

  if (framework === 'express') {
    return [
      {
        path: 'package.json',
        content: JSON.stringify({
          name: kebabName,
          version: '0.1.0',
          private: true,
          scripts: {
            dev: 'ts-node-dev --respawn src/index.ts',
            build: 'tsc',
            start: 'node dist/index.js',
          },
          dependencies: {
            express: '^4.18.0',
          },
          devDependencies: {
            typescript: '^5.0.0',
            'ts-node-dev': '^2.0.0',
            '@types/express': '^4.17.0',
            '@types/node': '^20.0.0',
          },
        }, null, 2) + '\n',
      },
      {
        path: 'tsconfig.json',
        content: JSON.stringify({
          compilerOptions: {
            target: 'es2020',
            module: 'commonjs',
            outDir: './dist',
            rootDir: './src',
            strict: true,
            esModuleInterop: true,
            skipLibCheck: true,
          },
          include: ['src/**/*'],
        }, null, 2) + '\n',
      },
      {
        path: 'src/index.ts',
        content: `import express from 'express';

const app = express();
const PORT = process.env.PORT || 3000;

app.use(express.json());

app.get('/', (_req, res) => {
  res.json({ message: '${name} API is running' });
});

app.listen(PORT, () => {
  console.log(\`Server running on port \${PORT}\`);
});
`,
      },
    ];
  }

  if (framework === 'flask') {
    return [
      {
        path: 'requirements.txt',
        content: `flask>=3.0.0
python-dotenv>=1.0.0
`,
      },
      {
        path: 'app/__init__.py',
        content: `from flask import Flask

def create_app():
    app = Flask(__name__)

    from . import routes
    app.register_blueprint(routes.bp)

    return app
`,
      },
      {
        path: 'app/routes.py',
        content: `from flask import Blueprint, jsonify

bp = Blueprint('main', __name__)

@bp.route('/')
def index():
    return jsonify({"message": "${name} API is running"})
`,
      },
      {
        path: 'run.py',
        content: `from app import create_app

app = create_app()

if __name__ == '__main__':
    app.run(debug=True)
`,
      },
    ];
  }

  if (framework === 'fastapi') {
    return [
      {
        path: 'requirements.txt',
        content: `fastapi>=0.104.0
uvicorn[standard]>=0.24.0
python-dotenv>=1.0.0
`,
      },
      {
        path: 'app/main.py',
        content: `from fastapi import FastAPI

app = FastAPI(title="${name}")

@app.get("/")
async def root():
    return {"message": "${name} API is running"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=True)
`,
      },
    ];
  }

  if (framework === 'astro') {
    return [
      {
        path: 'package.json',
        content: JSON.stringify({
          name: kebabName,
          version: '0.1.0',
          private: true,
          scripts: {
            dev: 'astro dev',
            build: 'astro build',
            preview: 'astro preview',
          },
          dependencies: {
            astro: '^4.0.0',
          },
        }, null, 2) + '\n',
      },
      {
        path: 'astro.config.mjs',
        content: `import { defineConfig } from 'astro/config';

export default defineConfig({});
`,
      },
      {
        path: 'src/pages/index.astro',
        content: `---
// ${name}
---

<html lang="ko">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width" />
    <title>${name}</title>
  </head>
  <body>
    <h1>${name}</h1>
    <p>VidEplace로 생성된 Astro 프로젝트입니다.</p>
  </body>
</html>
`,
      },
    ];
  }

  // Default fallback
  return [
    {
      path: 'README.md',
      content: `# ${name}\n\nVidEplace로 생성된 프로젝트입니다.\n`,
    },
  ];
}
