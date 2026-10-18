/**
 * YouTube's home grid (`ytd-rich-grid-renderer`), with the values of its `rich_grid_*` flags. The
 * smallest card grows 20px for every 100px the grid is wider than 1200px, so wide windows get
 * bigger cards rather than ever more columns.
 */
const GUTTER_PX = 16;
/** The feed pages' `px-6`, which YouTube counts in the width it sizes cards by. */
const PAGE_PADDING_PX = 48;
const BASE_MIN_ITEM_WIDTH_PX = 310;
const MIN_WIDTH_ANCHOR_PX = 1200;
const MIN_WIDTH_GROWTH_PER_100_PX = 20;
const MIN_WIDTH_LOWEST_OFFSET_PX = -40;
const MAX_COLUMNS = 6;

/** How many cards fit in one row of a grid whose content box is `contentWidth` wide. */
export function richGridColumns(contentWidth: number): number {
  const hostWidth = contentWidth + PAGE_PADDING_PX;
  const offset = Math.max(
    ((hostWidth - MIN_WIDTH_ANCHOR_PX) / 100) * MIN_WIDTH_GROWTH_PER_100_PX,
    MIN_WIDTH_LOWEST_OFFSET_PX,
  );
  const minItemWidth = BASE_MIN_ITEM_WIDTH_PX + offset;
  const columns = Math.floor((contentWidth + GUTTER_PX) / (minItemWidth + GUTTER_PX));
  return Math.min(MAX_COLUMNS, Math.max(1, columns));
}
