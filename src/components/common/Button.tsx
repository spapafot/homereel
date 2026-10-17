import type { ButtonHTMLAttributes, Ref } from 'react';

import { cx } from '@/lib/class-names';

import { Icon } from './Icon';

type ButtonVariant = 'tonal' | 'primary' | 'outline';

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  icon?: string;
  /** Solid icon, used for toggled-on states such as a favorited heart. */
  iconFilled?: boolean;
  ref?: Ref<HTMLButtonElement>;
};

const VARIANT_CLASSES: Record<ButtonVariant, string> = {
  tonal: 'bg-chip hover:bg-chip-hover',
  primary: 'bg-text text-page hover:opacity-85',
  outline: 'border border-divider text-link hover:bg-link/10',
};

/** YouTube's pill-shaped button. */
export function Button({
  variant = 'tonal',
  icon,
  iconFilled = false,
  className,
  children,
  ref,
  ...rest
}: ButtonProps) {
  return (
    <button
      ref={ref}
      type="button"
      className={cx(
        'flex h-9 shrink-0 items-center gap-1.5 rounded-full px-4 text-sm font-medium outline-none',
        'focus-visible:ring-2 focus-visible:ring-focus disabled:cursor-default disabled:opacity-50',
        icon && 'pl-3',
        VARIANT_CLASSES[variant],
        className,
      )}
      {...rest}
    >
      {icon && <Icon name={icon} filled={iconFilled} />}
      {children}
    </button>
  );
}
