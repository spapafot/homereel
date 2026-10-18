/** `MediaError.MEDIA_ERR_NETWORK`; other codes mean the file could not be decoded. */
const MEDIA_ERR_NETWORK = 2;

/** `noPicture`: the sound plays but the video codec can't be decoded, usually HEVC. */
export type PlaybackError = 'unsupported' | 'network' | 'noPicture';

export function classifyError(error: MediaError | null): PlaybackError {
  return error?.code === MEDIA_ERR_NETWORK ? 'network' : 'unsupported';
}
