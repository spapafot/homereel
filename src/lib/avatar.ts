const AVATAR_COLORS = [
  '#e53935',
  '#d81b60',
  '#8e24aa',
  '#5e35b1',
  '#3949ab',
  '#1e88e5',
  '#039be5',
  '#00897b',
  '#43a047',
  '#7cb342',
  '#f4511e',
  '#6d4c41',
  '#546e7a',
];

/** First letter or digit of a name, upper-cased, for folder and account avatars. */
export function avatarInitial(name: string): string {
  const match = name.match(/[\p{L}\p{N}]/u);
  return match ? match[0].toLocaleUpperCase() : '?';
}

/** A stable colour per key, so a folder keeps its avatar colour across sessions. */
export function avatarColor(key: string): string {
  let hash = 0;
  for (const char of key) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  }
  return AVATAR_COLORS[hash % AVATAR_COLORS.length];
}
