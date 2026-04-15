import React, { useEffect } from 'react';
import { create } from 'zustand';
import { CheckCircle, XCircle, AlertTriangle, Info, X } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

type ToastType = 'success' | 'error' | 'warning' | 'info';

interface ToastItem {
  id: string;
  type: ToastType;
  message: string;
  duration?: number; // ms, default 4000
}

/* ------------------------------------------------------------------ */
/*  Store                                                              */
/* ------------------------------------------------------------------ */

interface ToastStore {
  toasts: ToastItem[];
  add: (toast: Omit<ToastItem, 'id'>) => string;
  remove: (id: string) => void;
}

export const useToastStore = create<ToastStore>((set) => ({
  toasts: [],
  add: (toast) => {
    const id = crypto.randomUUID();
    set((s) => ({ toasts: [...s.toasts, { ...toast, id }] }));
    return id;
  },
  remove: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

/* ------------------------------------------------------------------ */
/*  Hook                                                               */
/* ------------------------------------------------------------------ */

export function useToast() {
  const add = useToastStore((s) => s.add);

  return {
    success: (message: string, duration?: number) =>
      add({ type: 'success', message, duration }),
    error: (message: string, duration?: number) =>
      add({ type: 'error', message, duration }),
    warning: (message: string, duration?: number) =>
      add({ type: 'warning', message, duration }),
    info: (message: string, duration?: number) =>
      add({ type: 'info', message, duration }),
  };
}

/* ------------------------------------------------------------------ */
/*  Config per type                                                    */
/* ------------------------------------------------------------------ */

const typeConfig: Record<ToastType, { icon: LucideIcon; accent: string; border: string }> = {
  success: {
    icon: CheckCircle,
    accent: 'text-accent-success',
    border: 'border-accent-success/30',
  },
  error: {
    icon: XCircle,
    accent: 'text-accent-error',
    border: 'border-accent-error/30',
  },
  warning: {
    icon: AlertTriangle,
    accent: 'text-accent-warning',
    border: 'border-accent-warning/30',
  },
  info: {
    icon: Info,
    accent: 'text-accent-primary',
    border: 'border-accent-primary/30',
  },
};

/* ------------------------------------------------------------------ */
/*  Single toast                                                       */
/* ------------------------------------------------------------------ */

function ToastCard({ toast }: { toast: ToastItem }) {
  const remove = useToastStore((s) => s.remove);
  const { icon: Icon, accent, border } = typeConfig[toast.type];

  useEffect(() => {
    const timer = setTimeout(() => remove(toast.id), toast.duration ?? 4000);
    return () => clearTimeout(timer);
  }, [toast.id, toast.duration, remove]);

  return (
    <div
      className={[
        'flex items-start gap-3 w-80 px-4 py-3 rounded-lg shadow-lg',
        'bg-bg-secondary border',
        border,
        'animate-toast-in',
      ].join(' ')}
    >
      <Icon size={18} className={`mt-0.5 shrink-0 ${accent}`} />
      <span className="flex-1 text-sm text-text-primary">{toast.message}</span>
      <button
        onClick={() => remove(toast.id)}
        className="shrink-0 p-0.5 rounded text-text-tertiary hover:text-text-primary transition-colors cursor-pointer"
      >
        <X size={14} />
      </button>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Container — render once at app root                                */
/* ------------------------------------------------------------------ */

export default function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);

  return (
    <>
      <div className="fixed bottom-4 right-4 z-[100] flex flex-col-reverse gap-2 pointer-events-auto">
        {toasts.map((t) => (
          <ToastCard key={t.id} toast={t} />
        ))}
      </div>

      <style>{`
        @keyframes toast-in {
          from { opacity: 0; transform: translateX(1rem); }
          to   { opacity: 1; transform: translateX(0); }
        }
        .animate-toast-in { animation: toast-in 200ms ease-out; }
      `}</style>
    </>
  );
}
