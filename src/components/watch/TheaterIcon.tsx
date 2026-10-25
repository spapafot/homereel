/**
 * The theater mode button's glyph from YouTube's player: a player frame with its control bar and
 * arrows pointing out to widen it, or in to bring it back to the default view. Material Symbols
 * has no such glyph, so it is drawn here from YouTube's, on a 24px grid at the size it shows.
 */
export function TheaterIcon({ isTheater }: { isTheater: boolean }) {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      width={24}
      height={24}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      className="shrink-0"
    >
      <rect x="3.3" y="5" width="17.4" height="14" rx="0.8" />
      <path d="M3.3 15.5h17.4" />
      <path
        d={
          isTheater
            ? 'M6.6 8l2.25 2.25L6.6 12.5M17.4 8l-2.25 2.25 2.25 2.25'
            : 'M8.85 8 6.6 10.25l2.25 2.25M15.15 8l2.25 2.25-2.25 2.25'
        }
      />
    </svg>
  );
}
