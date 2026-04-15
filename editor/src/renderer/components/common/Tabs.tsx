import React from 'react';
import type { LucideIcon } from 'lucide-react';

export interface Tab {
  id: string;
  label: string;
  icon?: LucideIcon;
}

interface TabsProps {
  tabs: Tab[];
  activeTab: string;
  onTabChange: (id: string) => void;
  className?: string;
}

export default function Tabs({ tabs, activeTab, onTabChange, className = '' }: TabsProps) {
  return (
    <div
      className={`flex items-center border-b border-border-primary ${className}`}
      style={{ gap: '4px', padding: '0 12px' }}
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeTab;
        const Icon = tab.icon;

        return (
          <button
            key={tab.id}
            onClick={() => onTabChange(tab.id)}
            className="relative flex items-center cursor-pointer select-none focus:outline-none"
            style={{
              gap: '8px',
              padding: '10px 16px',
              fontSize: '13px',
              fontWeight: isActive ? 600 : 500,
              color: isActive ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
              transition: 'color 0.15s ease',
            }}
          >
            {Icon && <Icon size={16} />}
            {tab.label}

            {isActive && (
              <span
                style={{
                  position: 'absolute',
                  bottom: 0,
                  left: '12px',
                  right: '12px',
                  height: '2px',
                  borderRadius: '2px 2px 0 0',
                  background: 'var(--color-accent-primary)',
                }}
              />
            )}
          </button>
        );
      })}
    </div>
  );
}
