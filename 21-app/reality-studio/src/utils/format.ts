/** Deterministic presentation formatters for engineering values. */

/** Format Layer-07 logical time (ns decimal string) as seconds with ms precision. */
export function formatLogicalTime(nanos: string | undefined | null): string {
  if (!nanos) return "—";
  try {
    const n = BigInt(nanos);
    const whole = n / 1_000_000_000n;
    const frac = n % 1_000_000_000n;
    const ms = Number(frac / 1_000_000n);
    return `${whole.toString()}.${ms.toString().padStart(3, "0")} s`;
  } catch {
    // Fallback for non-BigInt-safe inputs: parse as number conservatively.
    const value = Number(nanos);
    if (!Number.isFinite(value)) return "—";
    return `${(value / 1e9).toFixed(3)} s`;
  }
}

export function nanosToSecondsFloat(nanos: string | undefined | null): number | null {
  if (!nanos) return null;
  const value = Number(nanos);
  if (!Number.isFinite(value) || value < 0) return null;
  return value / 1e9;
}

export function formatQuantity(value: number, unit: string, digits = 3): string {
  if (!Number.isFinite(value)) return "—";
  return `${trimFloat(value, digits)} ${unit}`.trim();
}

export function trimFloat(value: number, digits = 3): string {
  if (!Number.isFinite(value)) return "—";
  const fixed = value.toFixed(digits);
  return fixed.replace(/\.?0+$/, "");
}

export function formatPercent(ratio: number): string {
  if (!Number.isFinite(ratio)) return "—";
  return `${Math.round(ratio * 100)}%`;
}
