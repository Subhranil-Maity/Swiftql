const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/** "812 B", "12.4 MB", "3 GB": binary units, at most three significant digits. */
export function formatBytes(bytes: number): string {
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${value.toFixed(digits).replace(/\.0$/, "")} ${UNITS[unit]}`;
}
