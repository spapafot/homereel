/**
 * Small UI preferences kept in localStorage. Storage can be unavailable or cleared, so every
 * read falls back to a default and writes never throw.
 */

const PREFIX = 'homereel.';
/** Homereel was called DriveTube; preferences saved then count until they are saved again. */
const FORMER_PREFIX = 'drivetube.';

function formerKey(key: string): string | null {
  return key.startsWith(PREFIX) ? FORMER_PREFIX + key.slice(PREFIX.length) : null;
}

export function readPreference<T extends string>(
  key: string,
  allowed: readonly T[],
  fallback: T,
): T {
  try {
    const former = formerKey(key);
    const value =
      window.localStorage.getItem(key) ??
      (former === null ? null : window.localStorage.getItem(former));
    return allowed.includes(value as T) ? (value as T) : fallback;
  } catch {
    return fallback;
  }
}

export function writePreference(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
    const former = formerKey(key);
    if (former !== null) window.localStorage.removeItem(former);
  } catch {
    // Preferences are a convenience; losing one is harmless.
  }
}
