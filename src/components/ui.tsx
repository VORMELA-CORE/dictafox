import { useEffect, useRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { AlertTriangle, Check, Info, X } from "lucide-react";
import type { AppError, Limitation, ToastMessage } from "../types";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger" | "quiet";
  size?: "sm" | "md" | "lg";
  icon?: ReactNode;
};

export function Button({
  variant = "secondary",
  size = "md",
  icon,
  className = "",
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      type="button"
      className={`btn btn-${variant} btn-${size} ${className}`.trim()}
      {...props}
    >
      {icon ? <span className="btn-icon">{icon}</span> : null}
      {children}
    </button>
  );
}

export function IconButton({
  label,
  children,
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`icon-btn ${className}`.trim()}
      aria-label={label}
      title={label}
      {...props}
    >
      {children}
    </button>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
}) {
  return (
    <label className={`toggle-row${disabled ? " is-disabled" : ""}`}>
      <span className="toggle-copy">
        <span className="toggle-label">{label}</span>
        {description ? (
          <span className="toggle-description">{description}</span>
        ) : null}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        className={`switch${checked ? " is-on" : ""}`}
        onClick={() => onChange(!checked)}
        disabled={disabled}
      >
        <span className="switch-thumb" />
      </button>
    </label>
  );
}

export function Field({
  label,
  hint,
  error,
  children,
  className = "",
}: {
  label: string;
  hint?: string;
  error?: string | null;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={`field ${className}`.trim()}>
      <span className="field-label">{label}</span>
      {children}
      {error ? (
        <span className="field-hint is-error">{error}</span>
      ) : hint ? (
        <span className="field-hint">{hint}</span>
      ) : null}
    </div>
  );
}

export function Badge({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "accent" | "success" | "warning" | "danger";
}) {
  return <span className={`badge badge-${tone}`}>{children}</span>;
}

export function ErrorBanner({
  error,
  onDismiss,
}: {
  error: AppError;
  onDismiss?: () => void;
}) {
  return (
    <div className="banner banner-error" role="alert">
      <AlertTriangle size={18} aria-hidden="true" />
      <div className="banner-body">
        <strong>{error.message}</strong>
        {error.hint ? <span>{error.hint}</span> : null}
      </div>
      {onDismiss ? (
        <button
          type="button"
          className="banner-close"
          onClick={onDismiss}
          aria-label="Dismiss error"
        >
          <X size={16} />
        </button>
      ) : null}
    </div>
  );
}

export function WarningBanner({
  title,
  children,
  onDismiss,
}: {
  title: string;
  children: ReactNode;
  onDismiss?: () => void;
}) {
  return (
    <div className="banner banner-warning" role="status">
      <Info size={18} aria-hidden="true" />
      <div className="banner-body">
        <strong>{title}</strong>
        <span>{children}</span>
      </div>
      {onDismiss ? (
        <button
          type="button"
          className="banner-close"
          onClick={onDismiss}
          aria-label="Dismiss notice"
        >
          <X size={16} />
        </button>
      ) : null}
    </div>
  );
}

export function LimitationsList({
  limitations,
}: {
  limitations: Limitation[];
}) {
  if (!limitations.length) return null;
  return (
    <div className="limitations">
      {limitations.map((limitation) => (
        <div className="limitation" key={limitation.code}>
          <AlertTriangle size={16} aria-hidden="true" />
          <div>
            <strong>{limitation.title}</strong>
            <p>{limitation.description}</p>
            <p className="limitation-hint">{limitation.hint}</p>
          </div>
        </div>
      ))}
    </div>
  );
}

export function Modal({
  title,
  description,
  children,
  footer,
  onClose,
}: {
  title: string;
  description?: string;
  children?: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
}) {
  const dialogRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    dialogRef.current?.focus();
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="modal-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        ref={dialogRef}
      >
        <div className="modal-head">
          <div>
            <h2>{title}</h2>
            {description ? <p>{description}</p> : null}
          </div>
          <IconButton label="Close" onClick={onClose}>
            <X size={18} />
          </IconButton>
        </div>
        {children ? <div className="modal-body">{children}</div> : null}
        {footer ? <div className="modal-footer">{footer}</div> : null}
      </div>
    </div>
  );
}

export function Toasts({
  toasts,
  onDismiss,
}: {
  toasts: ToastMessage[];
  onDismiss: (id: number) => void;
}) {
  return (
    <div className="toast-stack" aria-live="polite" aria-atomic="false">
      {toasts.map((toast) => (
        <div key={toast.id} className={`toast toast-${toast.kind}`}>
          <span className="toast-icon" aria-hidden="true">
            {toast.kind === "success" ? (
              <Check size={16} />
            ) : toast.kind === "error" ? (
              <AlertTriangle size={16} />
            ) : (
              <Info size={16} />
            )}
          </span>
          <span className="toast-message">{toast.message}</span>
          <button
            type="button"
            className="toast-close"
            onClick={() => onDismiss(toast.id)}
            aria-label="Dismiss notification"
          >
            <X size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}

export function Spinner({ label }: { label: string }) {
  return (
    <span className="spinner-wrap" role="status">
      <span className="spinner" aria-hidden="true" />
      <span className="sr-only">{label}</span>
    </span>
  );
}

export function EmptyState({
  icon,
  title,
  children,
  action,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <span className="empty-icon" aria-hidden="true">
        {icon}
      </span>
      <h3>{title}</h3>
      <p>{children}</p>
      {action}
    </div>
  );
}
