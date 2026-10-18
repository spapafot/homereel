/**
 * Small UI preferences kept in localStorage. Storage can be unavailable or cleared, so every
 * read falls back to a default and writes never throw.
 */
export function readPreference<T extends string>(
  key: string,
  allowed: readonly T[],
  fallback: T,
): T {
  try {
    const value = window.localStorage.getItem(key);
    return allowed.includes(value as T) ? (value as T) : fallback;
  } catch {
    return fallback;
  }
}

export function writePreference(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Preferences are a convenience; losing one is harmless.
  }
}
