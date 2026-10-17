import { cx } from '@/lib/class-names';

type IconProps = {
  name: string;
  filled?: boolean;
  size?: number;
  className?: string;
};

/** A Material Symbols icon, the icon family YouTube's desktop site uses. */
export function Icon({ name, filled = false, size = 24, className }: IconProps) {
  return (
    <span
      aria-hidden="true"
      className={cx('material-symbols-outlined shrink-0', filled && 'filled', className)}
      style={{ fontSize: size, width: size, height: size, lineHeight: 1 }}
    >
      {name}
    </span>
  );
}
