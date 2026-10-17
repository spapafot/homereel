import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { Link } from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import { Icon } from '@/components/common/Icon';
import {
  MENU_CONTENT_CLASS,
  MENU_ITEM_CLASS,
  MENU_SEPARATOR_CLASS,
} from '@/components/common/menu-styles';
import { SignInButton } from '@/components/common/SignInButton';
import { useAuthStatus, useDisconnectDrive } from '@/hooks/use-auth';
import { useThemePreference } from '@/hooks/use-theme-preference';
import { THEME_LABELS, THEME_PREFERENCES, type ThemePreference } from '@/lib/theme';

const APPEARANCE_LABELS: Record<ThemePreference, string> = {
  system: 'Device theme',
  dark: 'Dark',
  light: 'Light',
};

/** The avatar at the top right with account, appearance and sign-out options. */
export function AccountMenu() {
  const auth = useAuthStatus();
  const disconnect = useDisconnectDrive();
  const [theme, setTheme] = useThemePreference();

  if (!auth.data?.configured) return null;
  if (!auth.data.connected) return <SignInButton compact />;

  const email = auth.data.email ?? '';
  const name = auth.data.displayName ?? (email || 'Google account');

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger
        aria-label="Account menu"
        className="ml-2 flex size-10 items-center justify-center rounded-full outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        <Avatar name={name} colorKey={email || name} size={32} />
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content align="end" sideOffset={4} className={`${MENU_CONTENT_CLASS} w-75`}>
          <div className="flex gap-4 px-4 pt-2 pb-3">
            <Avatar name={name} colorKey={email || name} size={40} />
            <div className="min-w-0">
              <p className="truncate text-base">{name}</p>
              {email && <p className="truncate text-sm text-text-secondary">{email}</p>}
            </div>
          </div>
          <DropdownMenu.Separator className={MENU_SEPARATOR_CLASS} />
          <DropdownMenu.Sub>
            <DropdownMenu.SubTrigger className={MENU_ITEM_CLASS}>
              <Icon name="dark_mode" />
              Appearance: {APPEARANCE_LABELS[theme]}
              <Icon name="chevron_right" className="ml-auto" />
            </DropdownMenu.SubTrigger>
            <DropdownMenu.Portal>
              <DropdownMenu.SubContent sideOffset={4} className={MENU_CONTENT_CLASS}>
                <DropdownMenu.RadioGroup
                  value={theme}
                  onValueChange={(value) => setTheme(value as ThemePreference)}
                >
                  {THEME_PREFERENCES.map((option) => (
                    <DropdownMenu.RadioItem key={option} value={option} className={MENU_ITEM_CLASS}>
                      <span className="size-6">
                        <DropdownMenu.ItemIndicator>
                          <Icon name="check" />
                        </DropdownMenu.ItemIndicator>
                      </span>
                      {THEME_LABELS[option]}
                    </DropdownMenu.RadioItem>
                  ))}
                </DropdownMenu.RadioGroup>
              </DropdownMenu.SubContent>
            </DropdownMenu.Portal>
          </DropdownMenu.Sub>
          <DropdownMenu.Item asChild className={MENU_ITEM_CLASS}>
            <Link to="/settings">
              <Icon name="settings" />
              Settings
            </Link>
          </DropdownMenu.Item>
          <DropdownMenu.Separator className={MENU_SEPARATOR_CLASS} />
          <DropdownMenu.Item className={MENU_ITEM_CLASS} onSelect={() => disconnect.mutate()}>
            <Icon name="logout" />
            Disconnect Google Drive
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
