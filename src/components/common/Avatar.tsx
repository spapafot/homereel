import { avatarColor, avatarInitial } from '@/lib/avatar';
import { cx } from '@/lib/class-names';

type AvatarProps = {
  name: string;
  /** Keeps the colour stable when names repeat, e.g. a folder ID. */
  colorKey?: string;
  size?: number;
  className?: string;
};

/** Initial-letter avatar used for folders (in YouTube's channel slot) and the account. */
export function Avatar({ name, colorKey, size = 36, className }: AvatarProps) {
  return (
    <span
      aria-hidden="true"
      className={cx(
        'flex shrink-0 items-center justify-center rounded-full font-medium text-white select-none',
        className,
      )}
      style={{
        width: size,
        height: size,
        fontSize: Math.round(size * 0.45),
        backgroundColor: avatarColor(colorKey ?? name),
      }}
    >
      {avatarInitial(name)}
    </span>
  );
}
