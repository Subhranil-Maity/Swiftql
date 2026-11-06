const STEPS: [number, string][] = [
  [1e9, "B"],
  [1e6, "M"],
  [1e3, "k"],
];

/** "12", "340k", "1.2M": short form for counts that are estimates anyway. */
export function formatCompactCount(n: number): string {
  for (const [size, suffix] of STEPS) {
    if (Math.abs(n) >= size) {
      const value = n / size;
      return `${value < 10 ? value.toFixed(1).replace(/\.0$/, "") : Math.round(value)}${suffix}`;
    }
  }
  return String(Math.round(n));
}

/** "~340k": a planner estimate, marked as one. */
export function formatEstimate(n: number): string {
  return `~${formatCompactCount(n)}`;
}

/** "1 row", "1,000 rows". */
export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}
