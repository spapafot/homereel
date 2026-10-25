/**
 * YouTube's watch page layout (`ytd-watch-grid`), with the values its desktop site serves in its
 * `kevlar_watch_*` experiment flags. The related column takes a share of the window, plus whatever
 * is left once the player reaches the largest size that still fits the window's height.
 */
const PAGE_MARGIN_PX = 16;
const MASTHEAD_HEIGHT_PX = 56;
const COLUMNS_TOP_PADDING_PX = 12;
/** Room kept under the player (`kevlar_watch_flexy_metadata_height`; the code's default is 136). */
const SPACE_BELOW_PLAYER_PX = 48;
const NON_PLAYER_HEIGHT_PX = MASTHEAD_HEIGHT_PX + COLUMNS_TOP_PADDING_PX + SPACE_BELOW_PLAYER_PX;
const PLAYER_ASPECT_RATIO = 16 / 9;
const MIN_PLAYER_WIDTH_PX = 240 * PLAYER_ASPECT_RATIO;
const MAX_PLAYER_WIDTH_PX = 1840;
const TWO_COLUMN_MIN_WIDTH_PX = 1000;
const SECONDARY_SHARE = 0.275;
const SECONDARY_MIN_WIDTH_PX = 320;
const SECONDARY_MAX_WIDTH_PX = 700;
const SECONDARY_MAX_SHARE = 0.35;
/** The masthead plus room for the title under the player, as in YouTube's `100vh - 169px`. */
const THEATER_NON_PLAYER_HEIGHT_PX = 169;
const THEATER_MIN_HEIGHT_PX = 480;
const SINGLE_COLUMN_THEATER_MIN_HEIGHT_PX = 240;

export type WatchColumns = {
  twoColumns: boolean;
  /** Width of the related column; 0 when it moves under the player. */
  secondaryWidth: number;
  /** The player column never grows past this, so the video always fits the window's height. */
  maxPrimaryWidth: number;
};

/** Column sizes for a viewport measured without its scrollbar, as YouTube measures it. */
export function watchColumns(viewportWidth: number, viewportHeight: number): WatchColumns {
  const heightLimitedWidth = (viewportHeight - NON_PLAYER_HEIGHT_PX) * PLAYER_ASPECT_RATIO;
  const maxPrimaryWidth = Math.round(
    Math.min(MAX_PLAYER_WIDTH_PX, Math.max(MIN_PLAYER_WIDTH_PX, heightLimitedWidth)),
  );
  if (viewportWidth < TWO_COLUMN_MIN_WIDTH_PX) {
    return { twoColumns: false, secondaryWidth: 0, maxPrimaryWidth };
  }

  const share = viewportWidth * SECONDARY_SHARE;
  const leftOver = Math.max(0, viewportWidth - (share + 3 * PAGE_MARGIN_PX + heightLimitedWidth));
  const clamped = Math.min(
    SECONDARY_MAX_WIDTH_PX,
    Math.max(SECONDARY_MIN_WIDTH_PX, share + leftOver),
  );
  const secondaryWidth = Math.round(Math.min(clamped, viewportWidth * SECONDARY_MAX_SHARE));
  return { twoColumns: true, secondaryWidth, maxPrimaryWidth };
}

/**
 * Height of the theater mode player, which spans the whole window above both columns
 * (`#player-full-bleed-container`): 16:9 to the window's width, short enough to leave room for
 * the title, but never shorter than YouTube's minimum.
 */
export function theaterPlayerHeight(viewportWidth: number, viewportHeight: number): number {
  const minHeight =
    viewportWidth < TWO_COLUMN_MIN_WIDTH_PX
      ? SINGLE_COLUMN_THEATER_MIN_HEIGHT_PX
      : THEATER_MIN_HEIGHT_PX;
  const fittedHeight = Math.min(
    viewportWidth / PLAYER_ASPECT_RATIO,
    viewportHeight - THEATER_NON_PLAYER_HEIGHT_PX,
  );
  return Math.round(Math.max(minHeight, fittedHeight));
}
