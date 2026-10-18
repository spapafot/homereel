import { Icon } from '@/components/common/Icon';
import { api, type VideoSource } from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import type { PlaybackError } from '@/lib/playback-error';
import { showToast } from '@/lib/toast';

type ErrorMessage = { title: string; detail: string };

const NO_PICTURE_MESSAGE: ErrorMessage = {
  title: 'Homereel can play this video’s sound but not its picture',
  detail:
    'It’s most likely HEVC (H.265), the format iPhones record in. Windows needs Microsoft’s ' +
    'HEVC Video Extensions to show it. Install them from the Microsoft Store, then close and ' +
    'reopen Homereel.',
};

/** Where a video that can't play here can still be watched. */
const ELSEWHERE_LABELS: Record<VideoSource, string> = {
  drive: 'Open in Google Drive',
  local: 'Open in another player',
};

const UNSUPPORTED_TITLE = 'Homereel can’t play this video’s format';
const STOPPED_TITLE = 'The video stopped loading';

/** The way out depends on where the video is, except for the missing HEVC codec. */
const ERROR_MESSAGES: Record<VideoSource, Record<PlaybackError, ErrorMessage>> = {
  drive: {
    unsupported: {
      title: UNSUPPORTED_TITLE,
      detail: 'Open it in Google Drive to watch it in your browser.',
    },
    network: {
      title: STOPPED_TITLE,
      detail: 'Check your internet connection, then try again.',
    },
    noPicture: NO_PICTURE_MESSAGE,
  },
  local: {
    unsupported: { title: UNSUPPORTED_TITLE, detail: 'Open it in your default video player.' },
    network: {
      title: STOPPED_TITLE,
      detail: 'Check that its folder or drive is still connected, then try again.',
    },
    noPicture: NO_PICTURE_MESSAGE,
  },
};

const PRIMARY_BUTTON =
  'rounded-full bg-white px-4 py-2 text-sm font-medium text-black hover:bg-white/85';
const SECONDARY_BUTTON = 'rounded-full bg-white/15 px-4 py-2 text-sm font-medium hover:bg-white/25';

function openHevcStore() {
  api.playbackOpenHevcStore().catch((error: unknown) => showToast(toAppError(error).message));
}

type PlaybackErrorOverlayProps = {
  error: PlaybackError;
  source: VideoSource;
  onRetry: () => void;
  onOpenElsewhere: () => void;
};

/** Covers a player whose video can't be shown, with the way out that fits the error. */
export function PlaybackErrorOverlay({
  error,
  source,
  onRetry,
  onOpenElsewhere,
}: PlaybackErrorOverlayProps) {
  const message = ERROR_MESSAGES[source][error];
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-black/85 px-6 text-center text-white">
      <Icon name="error" size={48} />
      <p className="text-lg font-medium">{message.title}</p>
      <p className="max-w-md text-sm text-white/70">{message.detail}</p>
      <div className="mt-2 flex flex-wrap justify-center gap-2">
        {error === 'noPicture' && (
          <button type="button" onClick={openHevcStore} className={PRIMARY_BUTTON}>
            Get HEVC Video Extensions
          </button>
        )}
        {/* WebView2 checks for codecs once at startup, so retrying can't fix noPicture. */}
        {error === 'network' && (
          <button type="button" onClick={onRetry} className={SECONDARY_BUTTON}>
            Retry
          </button>
        )}
        <button
          type="button"
          onClick={onOpenElsewhere}
          className={error === 'noPicture' ? SECONDARY_BUTTON : PRIMARY_BUTTON}
        >
          {ELSEWHERE_LABELS[source]}
        </button>
      </div>
    </div>
  );
}
