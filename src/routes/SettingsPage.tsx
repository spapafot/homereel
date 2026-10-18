import { getVersion } from '@tauri-apps/api/app';
import { useQuery } from '@tanstack/react-query';
import { useState, type ReactNode } from 'react';

import { Button } from '@/components/common/Button';
import { ConfirmDialog } from '@/components/common/Modal';
import { PeopleSettings } from '@/components/people/PeopleSettings';
import { LibrarySettings } from '@/components/settings/LibrarySettings';
import { TitlesSettings } from '@/components/settings/TitlesSettings';
import { useClearHistory } from '@/hooks/use-library';
import { useThemePreference } from '@/hooks/use-theme-preference';
import { THEME_LABELS, THEME_PREFERENCES } from '@/lib/theme';

function SettingsSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="border-t border-divider py-6">
      <h2 className="mb-4 text-lg font-medium">{title}</h2>
      {children}
    </section>
  );
}

export function SettingsPage() {
  const [theme, setTheme] = useThemePreference();
  const version = useQuery({ queryKey: ['app-version'], queryFn: getVersion, staleTime: Infinity });
  const clearHistory = useClearHistory();
  const [confirmClear, setConfirmClear] = useState(false);

  return (
    <div className="mx-auto max-w-3xl px-6 py-8">
      <h1 className="mb-6 text-4xl font-bold">Settings</h1>

      <SettingsSection title="Video library">
        <LibrarySettings />
      </SettingsSection>

      <SettingsSection title="Appearance">
        <div role="radiogroup" aria-label="Theme" className="flex flex-col gap-1">
          {THEME_PREFERENCES.map((option) => (
            <label key={option} className="flex h-10 cursor-pointer items-center gap-3 text-sm">
              <input
                type="radio"
                name="theme"
                value={option}
                checked={theme === option}
                onChange={() => setTheme(option)}
                className="size-4 accent-link"
              />
              {THEME_LABELS[option]}
            </label>
          ))}
        </div>
      </SettingsSection>

      <SettingsSection title="AI titles">
        <TitlesSettings />
      </SettingsSection>

      <SettingsSection title="People and pets">
        <PeopleSettings />
      </SettingsSection>

      <SettingsSection title="History and library">
        <p className="text-sm text-text-secondary">
          Videos play straight from Google Drive or the folders on this computer. Favorites, Watch
          later, playlists, watch history and titles are stored only in Homereel.
        </p>
        <Button icon="delete" className="mt-4" onClick={() => setConfirmClear(true)}>
          Clear watch history
        </Button>
        <ConfirmDialog
          open={confirmClear}
          onOpenChange={setConfirmClear}
          title="Clear watch history?"
          description="Your Homereel watch history and resume points will be cleared. Your videos are not affected."
          confirmLabel="Clear watch history"
          onConfirm={() => clearHistory.mutate()}
        />
      </SettingsSection>

      <SettingsSection title="About">
        <p className="text-sm text-text-secondary">
          Homereel {version.data ? `version ${version.data}` : ''}
        </p>
      </SettingsSection>
    </div>
  );
}
