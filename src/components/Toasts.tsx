import { useEffect } from "react";
import { CheckCircle2, CircleAlert, Info, X } from "lucide-react";
import { useApp, type Toast } from "../store/useApp";

const ICONS = {
  info: Info,
  success: CheckCircle2,
  error: CircleAlert,
};

function ToastItem({ toast }: { toast: Toast }) {
  const dismiss = useApp((s) => s.dismissToast);
  const Icon = ICONS[toast.kind];

  useEffect(() => {
    const timer = window.setTimeout(() => dismiss(toast.id), toast.actionLabel ? 9000 : 5200);
    return () => window.clearTimeout(timer);
  }, [toast.id, toast.actionLabel, dismiss]);

  return (
    <div className={`toast toast-${toast.kind}`} role="status">
      <Icon size={17} className="toast-icon" />
      <p className="toast-msg">{toast.message}</p>
      {toast.actionLabel ? (
        <button
          className="toast-action"
          onClick={() => {
            toast.onAction?.();
            dismiss(toast.id);
          }}
        >
          {toast.actionLabel}
        </button>
      ) : null}
      <button className="toast-close" onClick={() => dismiss(toast.id)} aria-label="Fechar aviso">
        <X size={14} />
      </button>
    </div>
  );
}

export function Toasts() {
  const toasts = useApp((s) => s.toasts);
  if (toasts.length === 0) return null;
  return (
    <div className="toasts" aria-live="polite">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
}
