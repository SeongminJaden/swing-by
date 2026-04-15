import React from 'react';
import type { LucideIcon } from 'lucide-react';

interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  tooltip?: string;
  size?: 'sm' | 'md';
  active?: boolean;
}

const sizeClasses: Record<string, { button: string; icon: number }> = {
  sm: { button: 'h-7 w-7', icon: 14 },
  md: { button: 'h-8 w-8', icon: 16 },
};

export default function IconButton({
  icon: Icon,
  tooltip,
  size = 'md',
  active = false,
  className = '',
  ...rest
}: IconButtonProps) {
  const { button, icon: iconSize } = sizeClasses[size];

  return (
    <button
      title={tooltip}
      className={[
        'inline-flex items-center justify-center rounded-md',
        'transition-colors duration-150 cursor-pointer select-none',
        'focus:outline-none focus:ring-2 focus:ring-accent-primary/50',
        'disabled:opacity-40 disabled:pointer-events-none',
        active
          ? 'bg-bg-elevated text-accent-primary'
          : 'text-text-secondary hover:bg-bg-hover hover:text-text-primary',
        button,
        className,
      ].join(' ')}
      {...rest}
    >
      <Icon size={iconSize} />
    </button>
  );
}
