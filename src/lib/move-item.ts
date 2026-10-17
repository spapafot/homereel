/** Returns a copy of `items` with the item at `from` moved to `to`. */
export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
  const result = [...items];
  if (from < 0 || from >= result.length || to < 0 || to >= result.length) return result;
  const [moved] = result.splice(from, 1);
  result.splice(to, 0, moved);
  return result;
}
