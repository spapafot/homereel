import type { HomeMix } from './api';

/** Seeds stay below this, as the backend reads them as 32-bit unsigned numbers. */
const SEED_LIMIT = 2 ** 32;

/** `YYYY-MM-DD` in the user's time zone, for Home's On this day shelf. */
export function localDay(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}-${month}-${day}`;
}

function newMix(): HomeMix {
  // The day is fixed with the seed, so the feed does not change under the user at midnight.
  return { seed: Math.floor(Math.random() * SEED_LIMIT), today: localDay(new Date()) };
}

/** Home's current mix: a new one when the app starts and whenever the user clicks Home. */
let mix = newMix();
const listeners = new Set<() => void>();

export function getHomeMix(): HomeMix {
  return mix;
}

export function subscribeHomeMix(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function reshuffleHome(): void {
  mix = newMix();
  listeners.forEach((listener) => listener());
}
