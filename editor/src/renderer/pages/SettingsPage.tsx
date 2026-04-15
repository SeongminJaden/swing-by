import React, { useState, useRef, useCallback, useEffect } from 'react';
import { ArrowLeft, Check, Plus, Trash2, UserPlus, Users, DollarSign } from 'lucide-react';
import { useAppStore, Theme, Language, AppView } from '../stores/appStore';

type SettingsTab =
  | 'general'
  | 'theme'
  | 'language'
  | 'editor'
  | 'ai'
  | 'git'
  | 'deploy'
  | 'costs'
  | 'team'
  | 'notifications'
  | 'keybindings'
  | 'shortcuts';

const tabs: { key: SettingsTab; label: string }[] = [
  { key: 'general', label: '일반' },
  { key: 'theme', label: '테마' },
  { key: 'language', label: '언어' },
  { key: 'editor', label: '에디터' },
  { key: 'ai', label: 'AI' },
  { key: 'git', label: 'Git' },
  { key: 'deploy', label: '출시' },
  { key: 'costs', label: '비용' },
  { key: 'team', label: '팀' },
  { key: 'notifications', label: '알림' },
  { key: 'keybindings', label: '키바인딩' },
  { key: 'shortcuts', label: '단축키' },
];

export default function SettingsPage() {
  const { currentView, setCurrentView } = useAppStore();
  const previousView = useRef<AppView>(
    currentView === 'settings' ? 'ide' : currentView
  );
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');

  const handleBack = useCallback(() => {
    const target = previousView.current;
    setCurrentView(target === 'settings' ? 'ide' : target);
  }, [setCurrentView]);

  return (
    <div className="settings-layout">
      <div className="settings-body">
        {/* Sidebar */}
        <nav className="settings-sidebar">
          {tabs.map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={
                activeTab === tab.key
                  ? 'settings-sidebar-item-active'
                  : 'settings-sidebar-item'
              }
            >
              {tab.label}
            </button>
          ))}
        </nav>

        {/* Content */}
        <div className="settings-content">
          <div className="settings-content-inner">
            {activeTab === 'general' && <GeneralSection />}
            {activeTab === 'theme' && <ThemeSection />}
            {activeTab === 'language' && <LanguageSection />}
            {activeTab === 'editor' && <EditorSection />}
            {activeTab === 'costs' && <CostsSection />}
            {activeTab === 'team' && <TeamSection />}
            {!['general', 'theme', 'language', 'editor', 'costs', 'team'].includes(activeTab) && (
              <PlaceholderSection />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/* ────────────────────────────── General ────────────────────────────── */

function GeneralSection() {
  return (
    <div className="settings-section">
      <h2 className="settings-section-title">일반</h2>

      {/* Version info card */}
      <div className="settings-card">
        <div className="settings-card-title">앱 정보</div>
        <div className="settings-row">
          <div>
            <div className="settings-version-label">앱 버전</div>
            <div className="settings-version-value">VidEplace v0.1.0-alpha</div>
          </div>
        </div>
      </div>

      {/* Actions card */}
      <div className="settings-card">
        <div className="settings-card-title">관리</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <div>
            <button className="settings-btn-primary">업데이트 확인</button>
          </div>
          <div>
            <button className="settings-btn-danger">모든 설정 초기화</button>
            <p className="settings-description" style={{ marginTop: '10px' }}>
              모든 설정을 기본값으로 되돌립니다. 이 작업은 되돌릴 수 없습니다.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ────────────────────────────── Theme ────────────────────────────── */

interface ThemeCard {
  id: Theme | string;
  name: string;
  available: boolean;
  colors: { bg: string; sidebar: string; accent: string; text: string };
}

const themeCards: ThemeCard[] = [
  {
    id: 'dark',
    name: 'Dark',
    available: true,
    colors: { bg: '#0d1117', sidebar: '#161b22', accent: '#58a6ff', text: '#e6edf3' },
  },
  {
    id: 'light',
    name: 'Light',
    available: true,
    colors: { bg: '#ffffff', sidebar: '#f6f8fa', accent: '#0969da', text: '#1f2328' },
  },
  {
    id: 'monokai',
    name: 'Monokai',
    available: true,
    colors: { bg: '#272822', sidebar: '#1e1f1c', accent: '#f92672', text: '#f8f8f2' },
  },
  {
    id: 'solarized',
    name: 'Solarized',
    available: false,
    colors: { bg: '#002b36', sidebar: '#073642', accent: '#b58900', text: '#839496' },
  },
  {
    id: 'nord',
    name: 'Nord',
    available: false,
    colors: { bg: '#2e3440', sidebar: '#3b4252', accent: '#88c0d0', text: '#eceff4' },
  },
  {
    id: 'catppuccin',
    name: 'Catppuccin',
    available: false,
    colors: { bg: '#1e1e2e', sidebar: '#181825', accent: '#cba6f7', text: '#cdd6f4' },
  },
  {
    id: 'dracula',
    name: 'Dracula',
    available: false,
    colors: { bg: '#282a36', sidebar: '#21222c', accent: '#bd93f9', text: '#f8f8f2' },
  },
  {
    id: 'github',
    name: 'GitHub Dark',
    available: false,
    colors: { bg: '#0d1117', sidebar: '#161b22', accent: '#58a6ff', text: '#c9d1d9' },
  },
  {
    id: 'ayu',
    name: 'Ayu',
    available: false,
    colors: { bg: '#0b0e14', sidebar: '#0d1016', accent: '#e6b450', text: '#bfbdb6' },
  },
];

function ThemeSection() {
  const { theme, setTheme } = useAppStore();

  return (
    <div className="settings-section">
      <h2 className="settings-section-title">테마</h2>

      <div className="settings-card">
        <div className="settings-card-title">테마 선택</div>
        <div className="settings-theme-grid">
          {themeCards.map((card) => {
            const selected = card.available && theme === card.id;
            return (
              <button
                key={card.id}
                disabled={!card.available}
                onClick={() => card.available && setTheme(card.id as Theme)}
                className={
                  selected
                    ? 'settings-theme-card-active'
                    : 'settings-theme-card'
                }
                style={!card.available ? { opacity: 0.45, cursor: 'not-allowed' } : undefined}
              >
                {/* Mini preview */}
                <div
                  className="settings-theme-card-preview"
                  style={{ backgroundColor: card.colors.bg }}
                >
                  <div
                    style={{
                      width: '25%',
                      backgroundColor: card.colors.sidebar,
                    }}
                  />
                  <div
                    style={{
                      flex: 1,
                      display: 'flex',
                      flexDirection: 'column',
                      justifyContent: 'center',
                      gap: '6px',
                      padding: '0 10px',
                    }}
                  >
                    <div
                      style={{
                        height: '5px',
                        width: '75%',
                        borderRadius: '9999px',
                        backgroundColor: card.colors.text,
                        opacity: 0.6,
                      }}
                    />
                    <div
                      style={{
                        height: '5px',
                        width: '50%',
                        borderRadius: '9999px',
                        backgroundColor: card.colors.accent,
                      }}
                    />
                    <div
                      style={{
                        height: '5px',
                        width: '65%',
                        borderRadius: '9999px',
                        backgroundColor: card.colors.text,
                        opacity: 0.3,
                      }}
                    />
                  </div>
                </div>

                <div className="settings-theme-card-name">
                  <span className="settings-label">{card.name}</span>
                  {selected && <Check size={16} className="text-accent-primary" />}
                  {!card.available && (
                    <span className="settings-description" style={{ margin: 0 }}>
                      Coming soon
                    </span>
                  )}
                </div>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

/* ────────────────────────────── Language ────────────────────────────── */

interface LangOption {
  value: Language | string;
  label: string;
  available: boolean;
}

const languages: LangOption[] = [
  { value: 'ko', label: '한국어', available: true },
  { value: 'en', label: 'English', available: true },
  { value: 'ja', label: '日本語', available: false },
  { value: 'zh', label: '中文', available: false },
];

function LanguageSection() {
  const { language, setLanguage } = useAppStore();
  const [aiResponseLang, setAiResponseLang] = useState('same');
  const [aiCommentLang, setAiCommentLang] = useState('ko');
  const [commitMsgLang, setCommitMsgLang] = useState('en');

  return (
    <div className="settings-section">
      <h2 className="settings-section-title">언어</h2>

      {/* Interface language card */}
      <div className="settings-card">
        <div className="settings-card-title">인터페이스 언어</div>
        {languages.map((lang) => {
          const isActive = language === lang.value;
          return (
            <label
              key={lang.value}
              className={isActive ? 'settings-radio-active' : 'settings-radio'}
              style={!lang.available ? { opacity: 0.45, cursor: 'not-allowed' } : undefined}
            >
              <input
                type="radio"
                name="language"
                value={lang.value}
                checked={isActive}
                disabled={!lang.available}
                onChange={() => lang.available && setLanguage(lang.value as Language)}
                className="accent-accent-primary"
              />
              <span className="settings-label">{lang.label}</span>
              {!lang.available && (
                <span className="settings-description" style={{ margin: '0 0 0 auto' }}>
                  Coming soon
                </span>
              )}
            </label>
          );
        })}
      </div>

      {/* AI language settings card */}
      <div className="settings-card">
        <div className="settings-card-title">AI 언어 설정</div>

        <div className="settings-row">
          <div>
            <div className="settings-label">AI 응답 언어</div>
            <div className="settings-description">AI가 응답할 때 사용할 언어</div>
          </div>
          <select
            className="settings-select"
            value={aiResponseLang}
            onChange={(e) => setAiResponseLang(e.target.value)}
          >
            <option value="same">인터페이스 언어와 동일</option>
            <option value="en">항상 영어</option>
            <option value="ko">항상 한국어</option>
          </select>
        </div>

        <div className="settings-row">
          <div>
            <div className="settings-label">AI 코드 주석 언어</div>
            <div className="settings-description">AI가 코드 주석을 작성할 때 사용할 언어</div>
          </div>
          <select
            className="settings-select"
            value={aiCommentLang}
            onChange={(e) => setAiCommentLang(e.target.value)}
          >
            <option value="ko">한국어</option>
            <option value="en">영어</option>
            <option value="none">없음</option>
          </select>
        </div>

        <div className="settings-row">
          <div>
            <div className="settings-label">커밋 메시지 언어</div>
            <div className="settings-description">자동 생성되는 커밋 메시지의 언어</div>
          </div>
          <select
            className="settings-select"
            value={commitMsgLang}
            onChange={(e) => setCommitMsgLang(e.target.value)}
          >
            <option value="ko">한국어</option>
            <option value="en">영어 (권장)</option>
          </select>
        </div>
      </div>
    </div>
  );
}

/* ────────────────────────────── Editor ────────────────────────────── */

function EditorSection() {
  const [fontFamily, setFontFamily] = useState('JetBrains Mono');
  const [fontSize, setFontSize] = useState(14);
  const [tabSize, setTabSize] = useState(2);
  const [tabStyle, setTabStyle] = useState<'spaces' | 'tabs'>('spaces');
  const [autoSave, setAutoSave] = useState(true);
  const [lineWrap, setLineWrap] = useState(false);
  const [minimap, setMinimap] = useState(true);
  const [bracketMatching, setBracketMatching] = useState(true);
  const [formatOnSave, setFormatOnSave] = useState(true);

  return (
    <div className="settings-section">
      <h2 className="settings-section-title">에디터</h2>

      {/* Font & Size card */}
      <div className="settings-card">
        <div className="settings-card-title">글꼴 설정</div>

        <div className="settings-row">
          <div className="settings-label">글꼴</div>
          <select
            className="settings-select"
            value={fontFamily}
            onChange={(e) => setFontFamily(e.target.value)}
          >
            <option value="JetBrains Mono">JetBrains Mono</option>
            <option value="Fira Code">Fira Code</option>
            <option value="Cascadia Code">Cascadia Code</option>
            <option value="D2Coding">D2Coding</option>
          </select>
        </div>

        <div className="settings-row">
          <div className="settings-label">글꼴 크기</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '14px', minWidth: '200px' }}>
            <input
              type="range"
              min={12}
              max={16}
              value={fontSize}
              onChange={(e) => setFontSize(Number(e.target.value))}
              className="accent-accent-primary"
              style={{ flex: 1 }}
            />
            <span className="settings-label" style={{ width: '28px', textAlign: 'right' }}>
              {fontSize}
            </span>
          </div>
        </div>
      </div>

      {/* Tab settings card */}
      <div className="settings-card">
        <div className="settings-card-title">탭 설정</div>

        <div className="settings-row">
          <div className="settings-label">탭 크기</div>
          <div style={{ display: 'flex', gap: '8px' }}>
            {[2, 4].map((size) => (
              <button
                key={size}
                onClick={() => setTabSize(size)}
                className={tabSize === size ? 'settings-chip-active' : 'settings-chip'}
              >
                {size}칸
              </button>
            ))}
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-label">탭 스타일</div>
          <div style={{ display: 'flex', gap: '8px' }}>
            {([
              { value: 'spaces', label: 'Spaces' },
              { value: 'tabs', label: 'Tabs' },
            ] as const).map((opt) => (
              <button
                key={opt.value}
                onClick={() => setTabStyle(opt.value)}
                className={tabStyle === opt.value ? 'settings-chip-active' : 'settings-chip'}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Toggle settings card */}
      <div className="settings-card">
        <div className="settings-card-title">편집 옵션</div>

        <ToggleRow label="자동 저장" description="파일을 자동으로 저장합니다" checked={autoSave} onChange={setAutoSave} />
        <ToggleRow label="줄 바꿈" description="긴 줄을 자동으로 줄바꿈합니다" checked={lineWrap} onChange={setLineWrap} />
        <ToggleRow label="미니맵" description="코드 미니맵을 표시합니다" checked={minimap} onChange={setMinimap} />
        <ToggleRow label="괄호 매칭" description="매칭되는 괄호를 하이라이트합니다" checked={bracketMatching} onChange={setBracketMatching} />
        <ToggleRow label="저장 시 포맷팅" description="파일 저장 시 자동으로 코드를 정리합니다" checked={formatOnSave} onChange={setFormatOnSave} />
      </div>
    </div>
  );
}

/* ────────────────────────────── Costs ────────────────────────────── */

function CostsSection() {
  const [totalCost, setTotalCost] = useState(0);
  const [budget, setBudget] = useState(50);
  const [budgetInput, setBudgetInput] = useState('50');
  const [providers, setProviders] = useState<{ name: string; cost: number }[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    const load = async () => {
      if (!(window as any).electronAPI) {
        // Mock data for UI development
        setTotalCost(23.45);
        setBudget(50);
        setBudgetInput('50');
        setProviders([
          { name: 'Claude (Anthropic)', cost: 15.20 },
          { name: 'GPT-4 (OpenAI)', cost: 6.85 },
          { name: 'Gemini (Google)', cost: 1.40 },
        ]);
        setLoaded(true);
        return;
      }

      try {
        const [summary, budgetRes] = await Promise.all([
          (window as any).electronAPI.costsGetSummary(),
          (window as any).electronAPI.costsGetBudget(),
        ]);
        if (summary) {
          setTotalCost(summary.total ?? 0);
          setProviders(
            summary.byProvider
              ? Object.entries(summary.byProvider).map(([name, cost]) => ({
                  name,
                  cost: cost as number,
                }))
              : [],
          );
        }
        if (budgetRes) {
          setBudget(budgetRes.limit ?? 50);
          setBudgetInput(String(budgetRes.limit ?? 50));
        }
      } catch {
        // Use mock
        setTotalCost(23.45);
        setBudget(50);
        setBudgetInput('50');
        setProviders([
          { name: 'Claude (Anthropic)', cost: 15.20 },
          { name: 'GPT-4 (OpenAI)', cost: 6.85 },
          { name: 'Gemini (Google)', cost: 1.40 },
        ]);
      }
      setLoaded(true);
    };
    load();
  }, []);

  const handleSetBudget = async () => {
    const val = parseFloat(budgetInput);
    if (isNaN(val) || val <= 0) return;
    setBudget(val);
    try {
      await (window as any).electronAPI?.costsSetBudget(val);
    } catch {
      // ignore
    }
  };

  const usagePercent = budget > 0 ? Math.min((totalCost / budget) * 100, 100) : 0;

  return (
    <div className="settings-section">
      <h2 className="settings-section-title">비용</h2>

      {/* Total cost card */}
      <div className="settings-card">
        <div className="settings-card-title">이번 달 총 비용</div>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: '8px', marginBottom: '20px' }}>
          <span style={{ fontSize: '36px', fontWeight: 700, color: 'var(--color-text-primary)' }}>
            ${totalCost.toFixed(2)}
          </span>
          <span style={{ fontSize: '14px', color: 'var(--color-text-tertiary)' }}>
            / ${budget.toFixed(2)}
          </span>
        </div>

        {/* Usage bar */}
        <div className="cost-bar-track">
          <div
            className="cost-bar-fill"
            style={{
              width: `${usagePercent}%`,
              background: usagePercent > 90
                ? 'var(--color-accent-error)'
                : usagePercent > 70
                  ? 'var(--color-accent-warning)'
                  : 'var(--color-accent-primary)',
            }}
          />
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '8px' }}>
          <span style={{ fontSize: '12px', color: 'var(--color-text-tertiary)' }}>
            {usagePercent.toFixed(0)}% 사용
          </span>
          <span style={{ fontSize: '12px', color: 'var(--color-text-tertiary)' }}>
            ${(budget - totalCost).toFixed(2)} 남음
          </span>
        </div>
      </div>

      {/* By provider */}
      <div className="settings-card">
        <div className="settings-card-title">제공자별 비용</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          {providers.map((p) => (
            <div key={p.name} className="settings-row">
              <div>
                <div className="settings-label">{p.name}</div>
              </div>
              <span style={{ fontWeight: 600, color: 'var(--color-text-primary)', fontSize: '14px' }}>
                ${p.cost.toFixed(2)}
              </span>
            </div>
          ))}
          {providers.length === 0 && (
            <div className="settings-description">사용 기록이 없습니다</div>
          )}
        </div>
      </div>

      {/* Budget setting */}
      <div className="settings-card">
        <div className="settings-card-title">월별 예산 설정</div>
        <div className="settings-row">
          <div>
            <div className="settings-label">예산 한도 (USD)</div>
            <div className="settings-description">예산 초과 시 알림을 받습니다</div>
          </div>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <span style={{ color: 'var(--color-text-tertiary)', fontSize: '14px' }}>$</span>
            <input
              type="number"
              className="settings-select"
              style={{ width: '100px', textAlign: 'right' }}
              value={budgetInput}
              onChange={(e) => setBudgetInput(e.target.value)}
              onBlur={handleSetBudget}
              onKeyDown={(e) => e.key === 'Enter' && handleSetBudget()}
              min={1}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

/* ────────────────────────────── Team ────────────────────────────── */

interface Team {
  id: string;
  name: string;
  role: string;
}

interface TeamMember {
  id: string;
  email: string;
  name: string;
  role: 'owner' | 'admin' | 'member';
}

function TeamSection() {
  const [teams, setTeams] = useState<Team[]>([]);
  const [selectedTeam, setSelectedTeam] = useState<string | null>(null);
  const [members, setMembers] = useState<TeamMember[]>([]);
  const [newTeamName, setNewTeamName] = useState('');
  const [inviteEmail, setInviteEmail] = useState('');
  const [creating, setCreating] = useState(false);
  const [inviting, setInviting] = useState(false);

  useEffect(() => {
    const load = async () => {
      if (!(window as any).electronAPI) {
        // Mock
        setTeams([
          { id: 'team-1', name: 'VidEplace 팀', role: 'owner' },
        ]);
        return;
      }
      try {
        const res = await (window as any).electronAPI.teamGetMyTeams();
        if (Array.isArray(res)) {
          setTeams(res.map((t: any) => ({ id: t.id, name: t.name, role: t.role || 'member' })));
        }
      } catch {
        setTeams([{ id: 'team-1', name: 'VidEplace 팀', role: 'owner' }]);
      }
    };
    load();
  }, []);

  useEffect(() => {
    const loadMembers = async () => {
      if (!selectedTeam) {
        setMembers([]);
        return;
      }
      if (!(window as any).electronAPI) {
        // Mock
        setMembers([
          { id: 'u1', email: 'owner@example.com', name: '팀장', role: 'owner' },
          { id: 'u2', email: 'dev@example.com', name: '개발자', role: 'member' },
        ]);
        return;
      }
      try {
        const res = await (window as any).electronAPI.teamGetMembers(selectedTeam);
        if (Array.isArray(res)) {
          setMembers(res as TeamMember[]);
        }
      } catch {
        setMembers([
          { id: 'u1', email: 'owner@example.com', name: '팀장', role: 'owner' },
          { id: 'u2', email: 'dev@example.com', name: '개발자', role: 'member' },
        ]);
      }
    };
    loadMembers();
  }, [selectedTeam]);

  const handleCreateTeam = async () => {
    if (!newTeamName.trim()) return;
    setCreating(true);
    try {
      if ((window as any).electronAPI) {
        const res = await (window as any).electronAPI.teamCreate(newTeamName.trim());
        if (res?.id) {
          setTeams((prev) => [...prev, { id: res.id, name: newTeamName.trim(), role: 'owner' }]);
        }
      } else {
        const mockId = `team-${Date.now()}`;
        setTeams((prev) => [...prev, { id: mockId, name: newTeamName.trim(), role: 'owner' }]);
      }
      setNewTeamName('');
    } catch {
      // ignore
    }
    setCreating(false);
  };

  const handleInvite = async () => {
    if (!inviteEmail.trim() || !selectedTeam) return;
    setInviting(true);
    try {
      if ((window as any).electronAPI) {
        await (window as any).electronAPI.teamInvite(selectedTeam, inviteEmail.trim());
      }
      setMembers((prev) => [
        ...prev,
        { id: `u-${Date.now()}`, email: inviteEmail.trim(), name: inviteEmail.split('@')[0], role: 'member' },
      ]);
      setInviteEmail('');
    } catch {
      // ignore
    }
    setInviting(false);
  };

  const handleRemoveMember = async (userId: string) => {
    if (!selectedTeam) return;
    try {
      await (window as any).electronAPI?.teamRemoveMember(selectedTeam, userId);
    } catch {
      // ignore
    }
    setMembers((prev) => prev.filter((m) => m.id !== userId));
  };

  const roleLabel = (role: string) => {
    switch (role) {
      case 'owner': return '소유자';
      case 'admin': return '관리자';
      default: return '멤버';
    }
  };

  return (
    <div className="settings-section">
      <h2 className="settings-section-title">팀</h2>

      {/* Create team card */}
      <div className="settings-card">
        <div className="settings-card-title">팀 만들기</div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <input
            type="text"
            className="settings-select"
            style={{ flex: 1 }}
            placeholder="팀 이름을 입력하세요"
            value={newTeamName}
            onChange={(e) => setNewTeamName(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleCreateTeam()}
          />
          <button
            className="settings-btn-primary"
            onClick={handleCreateTeam}
            disabled={!newTeamName.trim() || creating}
          >
            <Plus size={14} />
            만들기
          </button>
        </div>
      </div>

      {/* Team list */}
      <div className="settings-card">
        <div className="settings-card-title">내 팀</div>
        {teams.length === 0 ? (
          <div className="settings-description">참여 중인 팀이 없습니다</div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            {teams.map((team) => (
              <button
                key={team.id}
                onClick={() => setSelectedTeam(team.id === selectedTeam ? null : team.id)}
                className={selectedTeam === team.id ? 'settings-radio-active' : 'settings-radio'}
                style={{ width: '100%' }}
              >
                <Users size={16} style={{ color: 'var(--color-accent-primary)', flexShrink: 0 }} />
                <span className="settings-label" style={{ flex: 1, textAlign: 'left' }}>{team.name}</span>
                <span style={{ fontSize: '11px', color: 'var(--color-text-tertiary)' }}>{roleLabel(team.role)}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Team members & invite */}
      {selectedTeam && (
        <>
          <div className="settings-card">
            <div className="settings-card-title">멤버 초대</div>
            <div style={{ display: 'flex', gap: '8px' }}>
              <input
                type="email"
                className="settings-select"
                style={{ flex: 1 }}
                placeholder="이메일 주소를 입력하세요"
                value={inviteEmail}
                onChange={(e) => setInviteEmail(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleInvite()}
              />
              <button
                className="settings-btn-primary"
                onClick={handleInvite}
                disabled={!inviteEmail.trim() || inviting}
              >
                <UserPlus size={14} />
                초대
              </button>
            </div>
          </div>

          <div className="settings-card">
            <div className="settings-card-title">멤버 목록</div>
            {members.length === 0 ? (
              <div className="settings-description">멤버가 없습니다</div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                {members.map((member) => (
                  <div key={member.id} className="settings-row" style={{ padding: '8px 0' }}>
                    <div>
                      <div className="settings-label">{member.name || member.email}</div>
                      <div className="settings-description" style={{ marginTop: '2px' }}>{member.email}</div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                      <span className={`team-role-badge team-role-${member.role}`}>
                        {roleLabel(member.role)}
                      </span>
                      {member.role !== 'owner' && (
                        <button
                          className="team-remove-btn"
                          onClick={() => handleRemoveMember(member.id)}
                          title="멤버 제거"
                        >
                          <Trash2 size={13} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

/* ────────────────────────────── Placeholder ────────────────────────────── */

function PlaceholderSection() {
  return (
    <div className="settings-section">
      <div className="settings-card" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', minHeight: '240px' }}>
        <p className="settings-description" style={{ fontSize: '14px' }}>준비 중...</p>
      </div>
    </div>
  );
}

/* ────────────────────────────── Shared UI ────────────────────────────── */

function ToggleRow({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="settings-row">
      <div>
        <div className="settings-label">{label}</div>
        {description && <div className="settings-description">{description}</div>}
      </div>
      <button
        onClick={() => onChange(!checked)}
        className={`settings-toggle ${checked ? 'settings-toggle-on' : 'settings-toggle-off'}`}
      >
        <span
          className={`settings-toggle-knob ${checked ? 'settings-toggle-knob-on' : 'settings-toggle-knob-off'}`}
        />
      </button>
    </div>
  );
}
