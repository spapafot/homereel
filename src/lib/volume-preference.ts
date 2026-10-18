import { readPreference, writePreference } from './local-preference';

/** The volume both players share, kept between sessions. */
const VOLUME_KEY = 'homereel.volume';

export const VOLUME_STEP = 0.05;
const VOLUME_LEVELS = Array.from({ length: 21 }, (_, step) => (step * VOLUME_STEP).toFixed(2));

export function readVolume(): number {
  return Number(readPreference(VOLUME_KEY, VOLUME_LEVELS, '1.00'));
}

export function saveVolume(volume: number): void {
  writePreference(VOLUME_KEY, volume.toFixed(2));
}
