/**
 * A vertical video with a play mark, in the Shorts slot of YouTube's guide. Material Symbols has
 * no Shorts glyph, so it is drawn here at the same 24px grid and stroke weight.
 */
export function ShortsIcon({ filled = false, size = 24 }: { filled?: boolean; size?: number }) {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="currentColor"
      className="shrink-0"
    >
      {filled ? (
        <path
          fillRule="evenodd"
          d="M8 2h8a3 3 0 0 1 3 3v14a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V5a3 3 0 0 1 3-3Zm2 6.5v7l5.5-3.5L10 8.5Z"
        />
      ) : (
        <>
          <path d="M8 2h8a3 3 0 0 1 3 3v14a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V5a3 3 0 0 1 3-3Zm0 2a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V5a1 1 0 0 0-1-1H8Z" />
          <path d="M10 8.5v7l5.5-3.5L10 8.5Z" />
        </>
      )}
    </svg>
  );
}
