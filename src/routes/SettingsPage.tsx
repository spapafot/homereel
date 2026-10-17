import { getVersion } from '@tauri-apps/api/app';
import { useQuery } from '@tanstack/react-query';
import { useState, type ReactNode } from 'react';

import { Avatar } from '@/components/common/Avatar';
import { Button } from '@/components/common/Button';
import { ConfirmDialog } from '@/components/common/Modal';
import { SignInButton } from '@/components/common/SignInButton';
import { PeopleSettings } from '@/components/people/PeopleSettings';
import { TitlesSettings } from '@/components/settings/TitlesSettings';
import { useAuthStatus, useDisconnectDrive } from '@/hooks/use-auth';
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
  const auth = useAuthStatus();
  const disconnect = useDisconnectDrive();
  const [theme, setTheme] = useThemePreference();
  const version = useQuery({ queryKey: ['app-version'], queryFn: getVersion, staleTime: Infinity });
  const clearHistory = useClearHistory();
  const [confirmClear, setConfirmClear] = useState(false);

  const email = auth.data?.email ?? '';
  const name = auth.data?.displayName ?? (email || 'Google account');

  return (
    <div className="mx-auto max-w-3xl px-6 py-8">
      <h1 className="mb-6 text-4xl font-bold">Settings</h1>

      <SettingsSection title="Google Drive">
        {auth.data?.connected ? (
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex min-w-0 items-center gap-4">
              <Avatar name={name} colorKey={email || name} size={48} />
              <div className="min-w-0">
                <p className="truncate text-base">{name}</p>
                {email && <p className="truncate text-sm text-text-secondary">{email}</p>}
              </div>
            </div>
            <Button onClick={() => disconnect.mutate()} disabled={disconnect.isPending}>
              Disconnect
            </Button>
          </div>
        ) : auth.data?.configured ? (
          <div className="flex flex-col items-start gap-3">
            <p className="text-sm text-text-secondary">
              Sign in with Google to let DriveTube read the videos in your Drive.
            </p>
            <SignInButton />
          </div>
        ) : (
          <p className="text-sm text-text-secondary">
            Add <code>src-tauri/google-oauth-client.json</code> as described in{' '}
            <code>docs/setup.md</code>, then restart DriveTube.
          </p>
        )}
        <p className="mt-4 text-xs text-text-secondary">
          DriveTube only reads your files. It never renames, moves or deletes anything in Drive.
        </p>
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
          Videos stream live from Google Drive. Favorites, Watch later, playlists, watch history and
          titles are stored only on this computer.
        </p>
        <Button icon="delete" className="mt-4" onClick={() => setConfirmClear(true)}>
          Clear watch history
        </Button>
        <ConfirmDialog
          open={confirmClear}
          onOpenChange={setConfirmClear}
          title="Clear watch history?"
          description="Your DriveTube watch history and resume points will be cleared. Your videos in Google Drive are not affected."
          confirmLabel="Clear watch history"
          onConfirm={() => clearHistory.mutate()}
        />
      </SettingsSection>

      <SettingsSection title="About">
        <p className="text-sm text-text-secondary">
          DriveTube {version.data ? `version ${version.data}` : ''}
        </p>
      </SettingsSection>
    </div>
  );
}
