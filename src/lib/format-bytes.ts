const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** Formats a byte count with binary multiples, e.g. `1.5 GB`. */
export function formatBytes(bytes: number): string {
  let value = Math.max(0, bytes);
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < UNITS.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const digits = unitIndex === 0 || value >= 100 ? 0 : 1;
  return `${value.toFixed(digits)} ${UNITS[unitIndex]}`;
}
