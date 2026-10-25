/**
 * Execution timeline — markers positioned by actual Layer-07 timestamps.
 * Selecting a timestamp requests the corresponding authoritative state from
 * the backend (via onSelectItem → parent fetches). No fake rewind sliders.
 */
import type { TimelineItem } from "../../api/types";
import { formatLogicalTime, nanosToSecondsFloat } from "../../utils/format";
import "./timeline.css";

interface Props {
  items: ReadonlyArray<TimelineItem>;
  selectedId: string | null;
  currentTimeNanos?: string | null;
  onSelectItem(item: TimelineItem): void;
}

export function Timeline({ items, selectedId, currentTimeNanos, onSelectItem }: Props) {
  const times = items.map((i) => nanosToSecondsFloat(i.time?.nanos) ?? 0);
  const maxT = Math.max(1, ...times);
  const now = nanosToSecondsFloat(currentTimeNanos);

  return (
    <div className="timeline" aria-label="Execution timeline">
      <div className="timeline__track" role="listbox" aria-label="Timeline markers">
        {items.map((item) => {
          const t = nanosToSecondsFloat(item.time?.nanos) ?? 0;
          return (
            <button
              key={item.id}
              role="option"
              aria-selected={selectedId === item.id}
              className="timeline__marker"
              data-type={item.type}
              style={{ left: `${(t / maxT) * 96 + 2}%` }}
              title={`${item.label} @ ${formatLogicalTime(item.time?.nanos)}`}
              onClick={() => onSelectItem(item)}
            />
          );
        })}
        {now !== null ? <div className="timeline__playhead" style={{ left: `${(Math.min(now, maxT) / maxT) * 96 + 2}%` }} aria-hidden /> : null}
      </div>
      <div className="timeline__items">
        {items.length === 0 ? <p className="page__subtitle" style={{ padding: 8 }}>No timeline events recorded yet for this execution.</p> : null}
        {[...items].reverse().slice(0, 200).map((item) => (
          <button key={item.id} className="timeline__item" aria-selected={selectedId === item.id} onClick={() => onSelectItem(item)}>
            <span className="mono">{formatLogicalTime(item.time?.nanos)}</span>
            <span className={`timeline__type`} style={{ color: item.severity === "failure" ? "var(--hq-failure)" : item.severity === "warning" ? "var(--hq-warning)" : undefined }}>{item.type.replace("_", " ")}</span>
            <span>{item.label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
