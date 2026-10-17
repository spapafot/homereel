const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const UNITS: ReadonlyArray<[string, number]> = [
  ['year', 365 * DAY],
  ['month', 30 * DAY],
  ['week', 7 * DAY],
  ['day', DAY],
  ['hour', HOUR],
  ['minute', MINUTE],
];

/** YouTube-style relative time such as `3 days ago` or `1 year ago`. */
export function formatRelativeDate(isoDate: string, now: Date = new Date()): string {
  const elapsed = now.getTime() - new Date(isoDate).getTime();
  if (Number.isNaN(elapsed)) return '';
  for (const [unit, unitMs] of UNITS) {
    const count = Math.floor(elapsed / unitMs);
    if (count >= 1) return `${count} ${unit}${count === 1 ? '' : 's'} ago`;
  }
  return 'just now';
}
