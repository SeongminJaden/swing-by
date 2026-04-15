import React from 'react';
import type { LucideIcon } from 'lucide-react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  icon?: LucideIcon;
  children: React.ReactNode;
}

const variantClasses: Record<string, string> = {
  primary:
    'bg-accent-primary text-white hover:brightness-110 active:brightness-90',
  secondary:
    'bg-bg-elevated text-text-primary hover:bg-bg-hover active:brightness-90',
  ghost:
    'bg-transparent text-text-secondary hover:bg-bg-hover hover:text-text-primary',
  danger:
    'bg-accent-error text-white hover:brightness-110 active:brightness-90',
};

const sizeClasses: Record<string, string> = {
  sm: 'px-2.5 py-1 text-xs gap-1.5',
  md: 'px-3.5 py-1.5 text-sm gap-2',
  lg: 'px-5 py-2.5 text-base gap-2.5',
};

const iconSizes: Record<string, number> = {
  sm: 14,
  md: 16,
  lg: 18,
};

export default function Button({
  variant = 'primary',
  size = 'md',
  icon: Icon,
  children,
  className = '',
  disabled,
  ...rest
}: ButtonProps) {
  return (
    <button
      disabled={disabled}
      className={[
        'inline-flex items-center justify-center rounded-md font-medium',
        'transition-all duration-150 cursor-pointer select-none',
        'focus:outline-none focus:ring-2 focus:ring-accent-primary/50',
        'disabled:opacity-40 disabled:pointer-events-none',
        variantClasses[variant],
        sizeClasses[size],
        className,
      ].join(' ')}
      {...rest}
    >
      {Icon && <Icon size={iconSizes[size]} />}
      {children}
    </button>
  );
}
