import { useState, useEffect } from 'react';
import { useAppStore } from '../stores/appStore';
import { Check } from 'lucide-react';

interface PlanCard {
  id: 'free' | 'pro' | 'team' | 'enterprise';
  name: string;
  price: string;
  priceSuffix: string;
  description: string;
  features: string[];
  buttonLabel: string;
  recommended?: boolean;
}

const defaultPlans: PlanCard[] = [
  {
    id: 'free',
    name: '무료로 시작',
    price: '$0',
    priceSuffix: '',
    description: '개인 프로젝트에 적합',
    features: [
      '서비스 1개',
      '기본 보안 검증',
      'AI는 본인 계정 사용',
      '출시 기능 미포함',
    ],
    buttonLabel: '무료로 시작',
  },
  {
    id: 'pro',
    name: '프로 플랜',
    price: '$12',
    priceSuffix: '/월',
    description: '전문 개발자를 위한 플랜',
    features: [
      '서비스 5개',
      '풀 보안검증',
      '출시 3개',
      '디버그 콘솔',
      '우선 지원',
    ],
    buttonLabel: 'Pro 시작하기',
    recommended: true,
  },
  {
    id: 'team',
    name: '팀 플랜',
    price: '$29',
    priceSuffix: '/seat/월',
    description: '팀 협업에 최적화',
    features: [
      '무제한 서비스',
      '팀 협업',
      '출시 10개',
      '워치독 모니터링',
    ],
    buttonLabel: 'Team 시작하기',
  },
  {
    id: 'enterprise',
    name: '엔터프라이즈',
    price: '커스텀',
    priceSuffix: '',
    description: '대규모 조직을 위한 플랜',
    features: [
      '무제한 전부',
      '온프레미스',
      'SSO, 감사 로그',
      '전담 지원, SLA',
    ],
    buttonLabel: '문의하기',
  },
];

function getAPI(): any {
  return (window as any).electronAPI;
}

export default function PricingPage() {
  const { setUserPlan, setCurrentView, userPlan } = useAppStore();
  const [plans, setPlans] = useState<PlanCard[]>(defaultPlans);
  const [loading, setLoading] = useState(false);
  const [subscribingPlan, setSubscribingPlan] = useState<string | null>(null);
  const [error, setError] = useState('');

  // Load real plans on mount
  useEffect(() => {
    const api = getAPI();
    if (!api?.paymentGetPlans) return;

    setLoading(true);
    api.paymentGetPlans()
      .then((result: any) => {
        if (result?.success && Array.isArray(result.plans) && result.plans.length > 0) {
          // Map backend plans to PlanCard format, fall back to defaults if mapping fails
          const mapped: PlanCard[] = result.plans.map((p: any) => ({
            id: p.id,
            name: p.name,
            price: p.price ?? `$${p.amount ?? 0}`,
            priceSuffix: p.priceSuffix ?? (p.interval ? `/${p.interval}` : ''),
            description: p.description ?? '',
            features: p.features ?? [],
            buttonLabel: p.buttonLabel ?? '시작하기',
            recommended: p.recommended ?? false,
          }));
          setPlans(mapped);
        }
        // If no plans returned, keep defaults
      })
      .catch(() => {
        // Keep default plans on error
      })
      .finally(() => setLoading(false));
  }, []);

  const handleSelect = async (planId: 'free' | 'pro' | 'team' | 'enterprise') => {
    setError('');
    const api = getAPI();

    // No API available: fallback to local state
    if (!api?.paymentSubscribe && !api?.authUpdatePlan) {
      setUserPlan(planId);
      setCurrentView('onboarding');
      return;
    }

    setSubscribingPlan(planId);
    try {
      if (planId === 'free') {
        // Free plan: just update the plan on the user record
        if (api.authUpdatePlan) {
          const result = await api.authUpdatePlan('free');
          if (result && !result.success) {
            setError(result.message || '요금제 선택에 실패했습니다');
            return;
          }
        }
      } else {
        // Paid plan: go through payment flow
        if (api.paymentSubscribe) {
          const result = await api.paymentSubscribe(planId);
          if (!result?.success) {
            setError(result?.message || '결제 처리에 실패했습니다');
            return;
          }
        }
      }
      setUserPlan(planId);
      setCurrentView('onboarding');
    } catch {
      setError('요금제 선택 중 오류가 발생했습니다');
    } finally {
      setSubscribingPlan(null);
    }
  };

  return (
    <div className="pricing-wrapper">
      <div className="pricing-container">
        {/* Header */}
        <div className="pricing-header">
          <div className="pricing-logo">V</div>
          <h1 className="heading-1 pricing-title">요금제를 선택하세요</h1>
          <p className="subtitle pricing-subtitle">
            VidEplace를 시작하기 위한 요금제를 선택해주세요
          </p>
          {userPlan && (
            <p className="subtitle" style={{ marginTop: '0.5rem', fontSize: '0.85rem' }}>
              현재 요금제: <strong>{userPlan.toUpperCase()}</strong>
            </p>
          )}
        </div>

        {error && (
          <div style={{ color: 'var(--color-accent-error, #f85149)', fontSize: '0.85rem', textAlign: 'center', marginBottom: '1rem' }}>
            {error}
          </div>
        )}

        {loading ? (
          <div style={{ textAlign: 'center', padding: '3rem', color: 'var(--color-text-secondary)' }}>
            요금제를 불러오는 중...
          </div>
        ) : (
          /* Plans grid */
          <div className="pricing-grid">
            {plans.map((plan) => {
              const isCurrentPlan = userPlan === plan.id;
              const isSubscribing = subscribingPlan === plan.id;
              return (
                <div
                  key={plan.id}
                  className={`pricing-card ${plan.recommended ? 'pricing-card-recommended' : ''}`}
                >
                  {plan.recommended && (
                    <div className="pricing-badge">추천</div>
                  )}

                  <div className="pricing-card-header">
                    <h3 className="pricing-plan-name">{plan.name}</h3>
                    <p className="pricing-plan-desc">{plan.description}</p>
                  </div>

                  <div className="pricing-price">
                    <span className="pricing-price-amount">{plan.price}</span>
                    {plan.priceSuffix && (
                      <span className="pricing-price-suffix">{plan.priceSuffix}</span>
                    )}
                  </div>

                  <ul className="pricing-features">
                    {plan.features.map((feature, i) => (
                      <li key={i} className="pricing-feature-item">
                        <Check size={16} className="pricing-check-icon" />
                        <span>{feature}</span>
                      </li>
                    ))}
                  </ul>

                  <button
                    className={plan.recommended ? 'pricing-btn-primary' : 'pricing-btn-ghost'}
                    onClick={() => handleSelect(plan.id)}
                    disabled={isSubscribing || isCurrentPlan}
                    style={(isSubscribing || isCurrentPlan) ? { opacity: 0.6, cursor: 'not-allowed' } : undefined}
                  >
                    {isCurrentPlan
                      ? '현재 요금제'
                      : isSubscribing
                        ? '처리 중...'
                        : plan.buttonLabel}
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
