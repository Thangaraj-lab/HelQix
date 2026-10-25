/** Shared presentational primitives used across product areas. */
import type { ReactNode } from "react";
import { ApiError } from "../api/errors";
import type { ResourcePhase } from "../hooks/useAsyncResource";

export function Badge({ tone, children }: { tone: "normal" | "active" | "success" | "warning" | "failure" | "info"; children: ReactNode }) {
  return (
    <span className={`hq-badge hq-badge--${tone}`}>
      <span className="hq-badge__dot" aria-hidden />
      {children}
    </span>
  );
}

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <div className="page__header">
      <div>
        <h1 className="page__title">{title}</h1>
        {subtitle ? <p className="page__subtitle">{subtitle}</p> : null}
      </div>
      {actions ? <div style={{ display: "flex", gap: "var(--hq-sp-3)" }}>{actions}</div> : null}
    </div>
  );
}

export function EmptyState({ title, body, action }: { title: string; body?: string; action?: ReactNode }) {
  return (
    <div className="hq-empty">
      <div className="hq-empty__title">{title}</div>
      {body ? <div>{body}</div> : null}
      {action ?? null}
    </div>
  );
}

export function Skeleton({ height = 16, width = "100%" }: { height?: number; width?: string | number }) {
  return <div className="hq-skeleton" style={{ height, width }} aria-hidden />;
}

export function LoadingBlock({ rows = 4 }: { rows?: number }) {
  return (
    <div role="status" aria-label="Loading" style={{ display: "grid", gap: "var(--hq-sp-3)", padding: "var(--hq-sp-4)" }}>
      {Array.from({ length: rows }, (_, i) => (
        <Skeleton key={i} height={i === 0 ? 24 : 14} width={i === 0 ? "45%" : `${88 - i * 9}%`} />
      ))}
    </div>
  );
}

/** Honest capability-gap panel: backend contract not implemented yet. */
export function UnavailablePanel({ feature, detail }: { feature: string; detail?: string }) {
  return (
    <section className="hq-panel" aria-live="polite">
      <div className="hq-panel__header">
        <span className="label">Integration status</span>
        <Badge tone="warning">Backend not available</Badge>
      </div>
      <div className="hq-panel__body">
        <p style={{ marginBottom: "var(--hq-sp-2)" }}>
          <strong>{feature}</strong> requires a Layer&nbsp;22 API endpoint that the connected backend has not
          implemented yet. HELQIX does not fabricate engineering data — this view will populate automatically
          when the service is available.
        </p>
        {detail ? <p className="mono" style={{ color: "var(--hq-text-muted)", fontSize: "var(--hq-text-xs)" }}>{detail}</p> : null}
      </div>
    </section>
  );
}

export function ErrorPanel({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  const message =
    error instanceof ApiError
      ? error.userMessage
      : error instanceof Error
        ? error.message
        : "Unknown error";
  return (
    <div className="hq-banner hq-banner--error" role="alert">
      <span style={{ flex: 1 }}>{message}</span>
      {onRetry ? (
        <button className="hq-button hq-button--sm" onClick={onRetry}>
          Retry
        </button>
      ) : null}
    </div>
  );
}

/** Renders loading / unavailable / error / empty states uniformly. */
export function ResourceGate<T>(props: {
  phase: ResourcePhase;
  error: unknown;
  data: T | null;
  feature: string;
  onRetry?: () => void;
  isEmpty?: (data: T) => boolean;
  emptyTitle?: string;
  emptyBody?: string;
  emptyAction?: ReactNode;
  children: (data: T) => ReactNode;
}) {
  const { phase, error, data, feature, onRetry, isEmpty, children } = props;
  if (phase === "idle" || phase === "loading") return <LoadingBlock />;
  if (phase === "unavailable") return <UnavailablePanel feature={feature} detail={error instanceof ApiError ? error.endpoint ?? "" : undefined} />;
  if (phase === "error") return <ErrorPanel error={error} onRetry={onRetry} />;
  if (data === null) return <EmptyState title={props.emptyTitle ?? "No data"} />;
  if (isEmpty?.(data))
    return (
      <EmptyState
        title={props.emptyTitle ?? "Nothing here yet"}
        body={props.emptyBody}
        action={props.emptyAction}
      />
    );
  return <>{children(data)}</>;
}
