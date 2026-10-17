import { Link } from 'react-router';

/** DriveTube's own mark: a folder with a play button. */
export function Logo() {
  return (
    <Link
      to="/"
      aria-label="DriveTube home"
      className="flex h-14 items-center gap-1 pr-3 pl-4 outline-none focus-visible:ring-2 focus-visible:ring-focus"
    >
      <svg viewBox="4 8 56 48" className="h-[22px] w-[26px]" aria-hidden="true">
        <path
          d="M6 16a6 6 0 0 1 6-6h14l6 6h20a6 6 0 0 1 6 6v26a6 6 0 0 1-6 6H12a6 6 0 0 1-6-6z"
          fill="var(--color-brand)"
        />
        <path d="M27 26v18l15-9z" fill="#ffffff" />
      </svg>
      <span className="text-[20px] font-bold tracking-[-0.06em]">DriveTube</span>
    </Link>
  );
}
