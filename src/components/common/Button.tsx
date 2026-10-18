import type { ButtonHTMLAttributes, Ref } from 'react';

import { cx } from '@/lib/class-names';

import { Icon } from './Icon';

type ButtonVariant = 'tonal' | 'primary' | 'outline';
/** `large` is the 40px height of the watch page's action buttons. */
type ButtonSize = 'default' | 'large';

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
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

// A height in `className` would not reliably win over these: the generated CSS decides the order.
const SIZE_CLASSES: Record<ButtonSize, string> = {
  default: 'h-9',
  large: 'h-10',
};

/** YouTube's pill-shaped button. */
export function Button({
  variant = 'tonal',
  size = 'default',
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
        'flex shrink-0 items-center gap-1.5 rounded-full px-4 text-sm font-medium outline-none',
        'focus-visible:ring-2 focus-visible:ring-focus disabled:cursor-default disabled:opacity-50',
        icon && 'pl-3',
        SIZE_CLASSES[size],
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
