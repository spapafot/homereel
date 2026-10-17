import { describe, expect, it } from 'vitest';

import { avatarColor, avatarInitial } from './avatar';

describe('avatarInitial', () => {
  it('should use the first letter or digit, upper-cased', () => {
    expect(avatarInitial('holidays 2024')).toBe('H');
    expect(avatarInitial('2024 trips')).toBe('2');
    expect(avatarInitial('_camera uploads')).toBe('C');
    expect(avatarInitial('διακοπές')).toBe('Δ');
  });

  it('should fall back to a question mark when there is nothing usable', () => {
    expect(avatarInitial('---')).toBe('?');
  });
});

describe('avatarColor', () => {
  it('should return the same colour for the same key', () => {
    expect(avatarColor('folder-123')).toBe(avatarColor('folder-123'));
    expect(avatarColor('folder-123')).toMatch(/^#[0-9a-f]{6}$/);
  });
});
