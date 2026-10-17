import { readPreference, writePreference } from './local-preference';

export const THEME_PREFERENCES = ['system', 'dark', 'light'] as const;
export type ThemePreference = (typeof THEME_PREFERENCES)[number];

/** Wording from YouTube's Appearance menu. */
export const THEME_LABELS: Record<ThemePreference, string> = {
  system: 'Use device theme',
  dark: 'Dark theme',
  light: 'Light theme',
};

const STORAGE_KEY = 'drivetube.theme';
const DARK_QUERY = '(prefers-color-scheme: dark)';

let currentPreference: ThemePreference = readPreference(STORAGE_KEY, THEME_PREFERENCES, 'system');
const listeners = new Set<() => void>();

export function getThemePreference(): ThemePreference {
  return currentPreference;
}

export function setThemePreference(preference: ThemePreference): void {
  currentPreference = preference;
  writePreference(STORAGE_KEY, preference);
  applyTheme();
  listeners.forEach((listener) => listener());
}

export function subscribeThemePreference(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Sets the `dark` class on <html> from the preference and, for "system", the OS theme. */
export function applyTheme(): void {
  const systemIsDark = window.matchMedia?.(DARK_QUERY).matches ?? false;
  const isDark = currentPreference === 'dark' || (currentPreference === 'system' && systemIsDark);
  document.documentElement.classList.toggle('dark', isDark);
}

/** Re-applies the theme when the OS switches between light and dark. */
export function watchSystemTheme(): () => void {
  const query = window.matchMedia?.(DARK_QUERY);
  if (!query) return () => undefined;
  query.addEventListener('change', applyTheme);
  return () => query.removeEventListener('change', applyTheme);
}
