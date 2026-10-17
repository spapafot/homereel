import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type ReactNode,
  type Ref,
} from 'react';

import { Icon } from '@/components/common/Icon';
import { api } from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import { cx } from '@/lib/class-names';
import { formatDuration } from '@/lib/format-duration';
import { readPreference, writePreference } from '@/lib/local-preference';
import { showToast } from '@/lib/toast';

import { ProgressBar } from './ProgressBar';

const HIDE_CONTROLS_AFTER_MS = 2500;
const SHORT_SEEK_SECONDS = 5;
const LONG_SEEK_SECONDS = 10;
const VOLUME_STEP = 0.05;
const VOLUME_KEY = 'drivetube.volume';
const VOLUME_LEVELS = Array.from({ length: 21 }, (_, step) => (step * VOLUME_STEP).toFixed(2));
/** `MediaError.MEDIA_ERR_NETWORK`; other codes mean the file could not be decoded. */
const MEDIA_ERR_NETWORK = 2;

/** `noPicture`: the sound plays but the video codec can't be decoded, usually HEVC. */
type PlaybackError = 'unsupported' | 'network' | 'noPicture';

const ERROR_MESSAGES: Record<PlaybackError, { title: string; detail: string }> = {
  unsupported: {
    title: 'DriveTube can’t play this video’s format',
    detail: 'Open it in Google Drive to watch it in your browser.',
  },
  network: {
    title: 'The video stopped loading',
    detail: 'Check your internet connection, then try again.',
  },
  noPicture: {
    title: 'DriveTube can play this video’s sound but not its picture',
    detail:
      'It’s most likely HEVC (H.265), the format iPhones record in. Windows needs Microsoft’s ' +
      'HEVC Video Extensions to show it. Install them from the Microsoft Store, then close and ' +
      'reopen DriveTube.',
  },
};

const PRIMARY_BUTTON =
  'rounded-full bg-white px-4 py-2 text-sm font-medium text-black hover:bg-white/85';
const SECONDARY_BUTTON = 'rounded-full bg-white/15 px-4 py-2 text-sm font-medium hover:bg-white/25';

function openHevcStore() {
  api.playbackOpenHevcStore().catch((error: unknown) => showToast(toAppError(error).message));
}

/** Lets the watch page act on the current frame, such as tagging a person in it. */
export type PlayerHandle = {
  /** Pauses playback and returns the position in milliseconds. */
  pauseAt: () => number;
};

type VideoPlayerProps = {
  src: string | undefined;
  title: string;
  hasNext: boolean;
  onNext: () => void;
  onOpenInDrive: () => void;
  /** Resume point; playback starts here instead of at 0. */
  startAtMs?: number | null;
  /** Called every few seconds, on pause, at the end and when leaving, to save the resume point. */
  onProgress?: (positionMs: number, durationMs: number) => void;
  ref?: Ref<PlayerHandle>;
};

const PROGRESS_INTERVAL_MS = 5_000;

function readVolume(): number {
  return Number(readPreference(VOLUME_KEY, VOLUME_LEVELS, '1.00'));
}

function classifyError(error: MediaError | null): PlaybackError {
  return error?.code === MEDIA_ERR_NETWORK ? 'network' : 'unsupported';
}

function isTypingTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

function bufferedEndAt(video: HTMLVideoElement): number {
  const { buffered, currentTime } = video;
  for (let index = 0; index < buffered.length; index += 1) {
    if (buffered.start(index) <= currentTime && currentTime <= buffered.end(index)) {
      return buffered.end(index);
    }
  }
  return 0;
}

function PlayerPill({ children }: { children: ReactNode }) {
  return <div className="flex h-12 items-center rounded-full bg-black/30">{children}</div>;
}

function PlayerButton({
  icon,
  label,
  onClick,
}: {
  icon: string;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="flex size-12 items-center justify-center rounded-full outline-none hover:bg-white/10 focus-visible:ring-2 focus-visible:ring-white"
    >
      <Icon name={icon} filled size={28} />
    </button>
  );
}

/**
 * YouTube-style player for videos streamed through the local media proxy. Keyboard shortcuts
 * follow YouTube: k/space play, j/l and arrows seek, f fullscreen, m mute, Shift+N next.
 */
export function VideoPlayer({
  src,
  title,
  hasNext,
  onNext,
  onOpenInDrive,
  startAtMs,
  onProgress,
  ref,
}: VideoPlayerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const hideTimerRef = useRef<number | undefined>(undefined);
  const resumeAtRef = useRef<number | null>(null);
  const hasStartedRef = useRef(false);
  const lastReportRef = useRef(0);
  const onProgressRef = useRef(onProgress);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isWaiting, setIsWaiting] = useState(true);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [bufferedEnd, setBufferedEnd] = useState(0);
  const [volume, setVolume] = useState(readVolume);
  const [isMuted, setIsMuted] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [error, setError] = useState<PlaybackError | null>(null);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    video.volume = volume;
    video.muted = isMuted;
  }, [volume, isMuted]);

  useEffect(() => {
    onProgressRef.current = onProgress;
  }, [onProgress]);

  useImperativeHandle(
    ref,
    () => ({
      pauseAt: () => {
        const video = videoRef.current;
        if (!video) return 0;
        video.pause();
        return Math.round(video.currentTime * 1000);
      },
    }),
    [],
  );

  const reportProgress = useCallback((video: HTMLVideoElement) => {
    if (!Number.isFinite(video.duration) || video.duration <= 0 || !hasStartedRef.current) return;
    lastReportRef.current = performance.now();
    onProgressRef.current?.(
      Math.round(video.currentTime * 1000),
      Math.round(video.duration * 1000),
    );
  }, []);

  // Saves the resume point, then detaches the source: that aborts the in-flight range request,
  // so leaving the page stops the download instead of letting it run in the background.
  useEffect(() => {
    const video = videoRef.current;
    return () => {
      window.clearTimeout(hideTimerRef.current);
      if (!video) return;
      reportProgress(video);
      video.pause();
      video.removeAttribute('src');
      video.load();
    };
  }, [reportProgress]);

  const revealControls = useCallback(() => {
    setControlsVisible(true);
    window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = window.setTimeout(
      () => setControlsVisible(false),
      HIDE_CONTROLS_AFTER_MS,
    );
  }, []);

  const togglePlay = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused || video.ended) {
      video.play().catch(() => undefined);
    } else {
      video.pause();
    }
  }, []);

  const seekTo = useCallback((time: number) => {
    const video = videoRef.current;
    if (!video || !Number.isFinite(video.duration)) return;
    video.currentTime = Math.min(Math.max(time, 0), video.duration);
  }, []);

  const changeVolume = useCallback((next: number) => {
    const clamped = Math.min(Math.max(next, 0), 1);
    setVolume(clamped);
    setIsMuted(clamped === 0);
    writePreference(VOLUME_KEY, clamped.toFixed(2));
  }, []);

  const toggleMute = useCallback(() => setIsMuted((muted) => !muted), []);

  const toggleFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      document.exitFullscreen().catch(() => undefined);
    } else {
      containerRef.current?.requestFullscreen().catch(() => undefined);
    }
  }, []);

  // Element fullscreen only fills the webview; resizing the native window makes it real
  // fullscreen.
  useEffect(() => {
    const onFullscreenChange = () => {
      const fullscreen = document.fullscreenElement === containerRef.current;
      setIsFullscreen(fullscreen);
      getCurrentWindow()
        .setFullscreen(fullscreen)
        .catch(() => undefined);
    };
    document.addEventListener('fullscreenchange', onFullscreenChange);
    return () => document.removeEventListener('fullscreenchange', onFullscreenChange);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || isTypingTarget(event.target)) return;
      const video = videoRef.current;
      if (!video) return;
      const key = event.key.toLowerCase();
      if (key === ' ' && event.target instanceof HTMLButtonElement) return;

      if (key === ' ' || key === 'k') togglePlay();
      else if (key === 'j') seekTo(video.currentTime - LONG_SEEK_SECONDS);
      else if (key === 'l') seekTo(video.currentTime + LONG_SEEK_SECONDS);
      else if (key === 'arrowleft') seekTo(video.currentTime - SHORT_SEEK_SECONDS);
      else if (key === 'arrowright') seekTo(video.currentTime + SHORT_SEEK_SECONDS);
      else if (key === 'arrowup') changeVolume(video.volume + VOLUME_STEP);
      else if (key === 'arrowdown') changeVolume(video.volume - VOLUME_STEP);
      else if (key === 'f') toggleFullscreen();
      else if (key === 'm') toggleMute();
      else if (key === 'n' && event.shiftKey && hasNext) onNext();
      else return;
      event.preventDefault();
      revealControls();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [
    togglePlay,
    seekTo,
    changeVolume,
    toggleFullscreen,
    toggleMute,
    hasNext,
    onNext,
    revealControls,
  ]);

  const retry = () => {
    const video = videoRef.current;
    if (!video) return;
    resumeAtRef.current = video.currentTime;
    setError(null);
    setIsWaiting(true);
    video.load();
  };

  const controlsShown = controlsVisible || !isPlaying || error != null;
  const volumeIcon =
    isMuted || volume === 0 ? 'volume_off' : volume < 0.5 ? 'volume_down' : 'volume_up';

  return (
    <div
      ref={containerRef}
      onMouseMove={revealControls}
      onMouseLeave={() => isPlaying && setControlsVisible(false)}
      className={cx(
        'relative w-full overflow-hidden bg-black',
        isFullscreen ? 'h-full' : 'aspect-video max-h-[calc(100vh-169px)]',
        !controlsShown && 'cursor-none',
      )}
    >
      <video
        ref={videoRef}
        src={src}
        autoPlay
        playsInline
        className="size-full object-contain"
        onClick={togglePlay}
        onDoubleClick={toggleFullscreen}
        onPlay={() => setIsPlaying(true)}
        onPause={(event) => {
          setIsPlaying(false);
          reportProgress(event.currentTarget);
        }}
        onEnded={(event) => {
          setIsPlaying(false);
          reportProgress(event.currentTarget);
          if (hasNext) onNext();
        }}
        onLoadedMetadata={(event) => {
          const video = event.currentTarget;
          // Chromium drops a video track it can't decode and plays the sound over black.
          if (video.videoWidth === 0) {
            video.pause();
            setError('noPicture');
            return;
          }
          if (resumeAtRef.current != null) {
            video.currentTime = resumeAtRef.current;
            resumeAtRef.current = null;
          } else if (!hasStartedRef.current && startAtMs) {
            video.currentTime = startAtMs / 1000;
          }
          hasStartedRef.current = true;
        }}
        onDurationChange={(event) => setDuration(event.currentTarget.duration || 0)}
        onTimeUpdate={(event) => {
          const video = event.currentTarget;
          setCurrentTime(video.currentTime);
          if (performance.now() - lastReportRef.current >= PROGRESS_INTERVAL_MS) {
            reportProgress(video);
          }
        }}
        onProgress={(event) => setBufferedEnd(bufferedEndAt(event.currentTarget))}
        onWaiting={() => setIsWaiting(true)}
        onCanPlay={() => setIsWaiting(false)}
        onPlaying={() => setIsWaiting(false)}
        onError={(event) => setError(classifyError(event.currentTarget.error))}
      />

      {isWaiting && !error && src && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="size-16 animate-spin rounded-full border-4 border-white/25 border-t-white" />
        </div>
      )}

      {error && (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-black/85 px-6 text-center text-white">
          <Icon name="error" size={48} />
          <p className="text-lg font-medium">{ERROR_MESSAGES[error].title}</p>
          <p className="max-w-md text-sm text-white/70">{ERROR_MESSAGES[error].detail}</p>
          <div className="mt-2 flex flex-wrap justify-center gap-2">
            {error === 'noPicture' && (
              <button type="button" onClick={openHevcStore} className={PRIMARY_BUTTON}>
                Get HEVC Video Extensions
              </button>
            )}
            {/* WebView2 checks for codecs once at startup, so retrying can't fix noPicture. */}
            {error === 'network' && (
              <button type="button" onClick={retry} className={SECONDARY_BUTTON}>
                Retry
              </button>
            )}
            <button
              type="button"
              onClick={onOpenInDrive}
              className={error === 'noPicture' ? SECONDARY_BUTTON : PRIMARY_BUTTON}
            >
              Open in Google Drive
            </button>
          </div>
        </div>
      )}

      {isFullscreen && controlsShown && (
        <div className="pointer-events-none absolute inset-x-0 top-0 bg-gradient-to-b from-black/70 to-transparent px-6 pt-5 pb-12">
          <p className="truncate text-lg text-white">{title}</p>
        </div>
      )}

      <div
        className={cx(
          'absolute inset-x-0 bottom-0 transition-opacity duration-200',
          controlsShown && !error ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
      >
        <div className="pointer-events-none absolute inset-x-0 bottom-0 h-36 bg-gradient-to-t from-black/70 to-transparent" />
        <div className="relative px-3 pb-2">
          <ProgressBar
            currentTime={currentTime}
            duration={duration}
            bufferedEnd={bufferedEnd}
            onSeek={seekTo}
          />
          <div className="mt-1 flex items-center justify-between text-white">
            <div className="flex items-center gap-2">
              <PlayerPill>
                <PlayerButton
                  icon={isPlaying ? 'pause' : 'play_arrow'}
                  label={isPlaying ? 'Pause (k)' : 'Play (k)'}
                  onClick={togglePlay}
                />
                {hasNext && (
                  <PlayerButton icon="skip_next" label="Next (Shift+N)" onClick={onNext} />
                )}
              </PlayerPill>
              <div className="group/volume flex h-12 items-center rounded-full bg-black/30">
                <PlayerButton
                  icon={volumeIcon}
                  label={isMuted ? 'Unmute (m)' : 'Mute (m)'}
                  onClick={toggleMute}
                />
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={VOLUME_STEP}
                  value={isMuted ? 0 : volume}
                  aria-label="Volume"
                  onChange={(event) => changeVolume(Number(event.target.value))}
                  className="w-0 accent-white opacity-0 transition-all duration-200 group-hover/volume:mr-4 group-hover/volume:w-16 group-hover/volume:opacity-100 focus:mr-4 focus:w-16 focus:opacity-100"
                />
              </div>
              <PlayerPill>
                <span className="px-4 text-[13px] tabular-nums">
                  {formatDuration(currentTime * 1000)} / {formatDuration(duration * 1000)}
                </span>
              </PlayerPill>
            </div>
            <PlayerPill>
              <PlayerButton
                icon={isFullscreen ? 'fullscreen_exit' : 'fullscreen'}
                label={isFullscreen ? 'Exit full screen (f)' : 'Full screen (f)'}
                onClick={toggleFullscreen}
              />
            </PlayerPill>
          </div>
        </div>
      </div>
    </div>
  );
}
