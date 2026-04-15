import React from 'react';

type Status = 'dev' | 'verifying' | 'deployed' | 'error';

interface StatusBadgeProps {
  status: Status;
  className?: string;
}

const config: Record<Status, { label: string; dot: string; text: string }> = {
  dev: {
    label: '개발중',
    dot: 'bg-accent-primary',
    text: 'text-accent-primary',
  },
  verifying: {
    label: '검증중',
    dot: 'bg-accent-warning',
    text: 'text-accent-warning',
  },
  deployed: {
    label: '출시완료',
    dot: 'bg-accent-success',
    text: 'text-accent-success',
  },
  error: {
    label: '오류',
    dot: 'bg-accent-error',
    text: 'text-accent-error',
  },
};

export default function StatusBadge({ status, className = '' }: StatusBadgeProps) {
  const { label, dot, text } = config[status];

  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ${text} bg-bg-elevated ${className}`}
    >
      <span className={`inline-block h-2 w-2 rounded-full ${dot}`} />
      {label}
    </span>
  );
}
