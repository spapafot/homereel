import { Link } from 'react-router';

import { reshuffleHome } from '@/lib/home-mix';

// The same shapes as public/favicon.svg, from which the app icons are generated.
const REEL_PATH =
  'M4 32a24 24 0 1 0 48 0a24 24 0 1 0-48 0zM25 32a3 3 0 1 0 6 0a3 3 0 1 0-6 0z' +
  'M23 19.5a5 5 0 1 0 10 0a5 5 0 1 0-10 0zM34.89 28.14a5 5 0 1 0 10 0a5 5 0 1 0-10 0z' +
  'M30.35 42.11a5 5 0 1 0 10 0a5 5 0 1 0-10 0zM15.65 42.11a5 5 0 1 0 10 0a5 5 0 1 0-10 0z' +
  'M11.11 28.14a5 5 0 1 0 10 0a5 5 0 1 0-10 0z';
const FILM_PATH = 'M28 50h30a2 2 0 0 1 2 2v2a2 2 0 0 1-2 2H28z';

/** Homereel's own mark: a film reel. Like YouTube's logo, it opens a fresh Home. */
export function Logo() {
  return (
    <Link
      to="/"
      onClick={reshuffleHome}
      aria-label="Homereel home"
      className="flex h-14 items-center gap-1 pr-3 pl-4 outline-none focus-visible:ring-2 focus-visible:ring-focus"
    >
      <svg viewBox="4 8 56 48" className="h-[22px] w-[26px]" aria-hidden="true">
        <path d={REEL_PATH} fillRule="evenodd" fill="var(--color-brand)" />
        <path d={FILM_PATH} fill="var(--color-brand)" />
      </svg>
      <span className="text-[20px] font-bold tracking-[-0.06em]">Homereel</span>
    </Link>
  );
}
