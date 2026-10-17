import type { ButtonHTMLAttributes, Ref } from 'react';

import { cx } from '@/lib/class-names';

import { Icon } from './Icon';

type IconButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  icon: string;
  label: string;
  filled?: boolean;
  ref?: Ref<HTMLButtonElement>;
};

export function IconButton({ icon, label, filled, className, ref, ...rest }: IconButtonProps) {
  return (
    <button
      ref={ref}
      type="button"
      aria-label={label}
      title={label}
      className={cx(
        'flex size-10 shrink-0 items-center justify-center rounded-full outline-none',
        'enabled:hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus',
        'enabled:active:bg-chip-hover disabled:cursor-default disabled:opacity-50',
        className,
      )}
      {...rest}
    >
      <Icon name={icon} filled={filled} />
    </button>
  );
}
