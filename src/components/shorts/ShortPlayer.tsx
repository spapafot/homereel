import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';

import { Icon } from '@/components/common/Icon';
import { PlaybackErrorOverlay } from '@/components/watch/PlaybackErrorOverlay';
import { ProgressBar } from '@/components/watch/ProgressBar';
import { useThumbnailUrl } from '@/hooks/use-thumbnail-url';
import { usePlaybackSource } from '@/hooks/use-videos';
import { api, type VideoSummary } from '@/lib/api';
import { bufferedEndAt } from '@/lib/buffered-end';
import { cx } from '@/lib/class-names';
import { classifyError, type PlaybackError } from '@/lib/playback-error';
import { isTypingTarget } from '@/lib/typing-target';
import { patchVideoInCaches } from '@/lib/video-cache';
import { readVolume } from '@/lib/volume-preference';

/** Shorter views are Shorts scrolled past, which History leaves out. */
const MIN_VIEW_SECONDS = 1;
/** Rust's threshold for a watched video. */
const WATCHED_FRACTION = 0.9;

type ViewState = { seen: boolean; finished: boolean; lastTime: number };

type ShortPlayerProps = {
  video: VideoSummary;
  isMuted: boolean;
  onToggleMute: () => void;
  onOpenElsewhere: () => void;
};

function OverlayButton({
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
      className="flex size-10 items-center justify-center rounded-full outline-none hover:bg-white/15 focus-visible:ring-2 focus-visible:ring-white"
    >
      <Icon name={icon} filled />
    </button>
  );
}

/**
 * The playing Short. Like YouTube's it loops, pauses on click and never resumes: History records
 * it from 0, or as watched once it has played (nearly) to the end. k/space play, m mutes.
 */
export function ShortPlayer({ video, isMuted, onToggleMute, onOpenElsewhere }: ShortPlayerProps) {
  const queryClient = useQueryClient();
  const source = usePlaybackSource(video.id, true);
  const src = source.data?.url;
  const posterSrc = useThumbnailUrl(video.id);
  const videoRef = useRef<HTMLVideoElement>(null);
  const viewRef = useRef<ViewState>({ seen: false, finished: false, lastTime: 0 });
  const [isPlaying, setIsPlaying] = useState(false);
  const [isWaiting, setIsWaiting] = useState(true);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [bufferedEnd, setBufferedEnd] = useState(0);
  const [error, setError] = useState<PlaybackError | null>(null);

  useEffect(() => {
    const element = videoRef.current;
    if (element) element.volume = readVolume();
  }, []);

  useEffect(() => {
    const element = videoRef.current;
    if (element) element.muted = isMuted;
  }, [isMuted]);

  const recordView = useCallback(
    (element: HTMLVideoElement, view: ViewState) => {
      const durationMs = Math.round(element.duration * 1000);
      if (!view.seen || !Number.isFinite(durationMs) || durationMs <= 0) return;
      const positionMs = view.finished ? durationMs : 0;
      api
        .recordProgress(video.id, positionMs, durationMs)
        .then(() => {
          patchVideoInCaches(queryClient, video.id, (current) => ({
            ...current,
            lastPositionMs: positionMs,
            watched: current.watched || view.finished,
          }));
          void queryClient.invalidateQueries({ queryKey: ['history'] });
          void queryClient.invalidateQueries({ queryKey: ['library', 'continueWatching'] });
        })
        .catch((cause: unknown) => console.error('Could not add the Short to History', cause));
    },
    [queryClient, video.id],
  );

  // The source is set here rather than as a prop so the cleanup can detach it: that aborts the
  // range request, and leaving a Short stops its download instead of letting it run on.
  useEffect(() => {
    const element = videoRef.current;
    if (!element || !src) return;
    const view = viewRef.current;
    element.src = src;
    return () => {
      recordView(element, view);
      element.pause();
      element.removeAttribute('src');
      element.load();
    };
  }, [src, recordView]);

  const togglePlay = useCallback(() => {
    const element = videoRef.current;
    if (!element) return;
    if (element.paused) element.play().catch(() => undefined);
    else element.pause();
  }, []);

  const seekTo = (time: number) => {
    const element = videoRef.current;
    if (!element || !Number.isFinite(element.duration)) return;
    element.currentTime = Math.min(Math.max(time, 0), element.duration);
    // Seeking back is not a loop, so it must not count as playing to the end.
    viewRef.current.lastTime = element.currentTime;
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || isTypingTarget(event.target)) return;
      const key = event.key.toLowerCase();
      if (key === ' ' && event.target instanceof HTMLButtonElement) return;
      if (key === 'k' || key === ' ') togglePlay();
      else if (key === 'm') onToggleMute();
      else return;
      event.preventDefault();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [togglePlay, onToggleMute]);

  const retry = () => {
    const element = videoRef.current;
    if (!element) return;
    setError(null);
    setIsWaiting(true);
    element.load();
  };

  return (
    <div className="group/short absolute inset-0">
      <video
        ref={videoRef}
        poster={posterSrc}
        autoPlay
        loop
        playsInline
        className="size-full object-contain"
        onClick={togglePlay}
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onLoadedMetadata={(event) => {
          // Chromium drops a video track it can't decode and plays the sound over black.
          if (event.currentTarget.videoWidth === 0) {
            event.currentTarget.pause();
            setError('noPicture');
          }
        }}
        onDurationChange={(event) => setDuration(event.currentTarget.duration || 0)}
        onTimeUpdate={(event) => {
          const element = event.currentTarget;
          const view = viewRef.current;
          if (element.currentTime >= MIN_VIEW_SECONDS) view.seen = true;
          // A loop starts over at 0, so time jumping back means the Short reached its end.
          const looped = element.currentTime < view.lastTime - 1;
          if (looped || element.currentTime >= element.duration * WATCHED_FRACTION) {
            view.finished = true;
          }
          view.lastTime = element.currentTime;
          setCurrentTime(element.currentTime);
        }}
        onProgress={(event) => setBufferedEnd(bufferedEndAt(event.currentTarget))}
        onWaiting={() => setIsWaiting(true)}
        onCanPlay={() => setIsWaiting(false)}
        onPlaying={() => setIsWaiting(false)}
        onError={(event) => setError(classifyError(event.currentTarget.error))}
      />

      {isWaiting && !error && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="size-12 animate-spin rounded-full border-4 border-white/25 border-t-white" />
        </div>
      )}
      {!isPlaying && !isWaiting && !error && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="flex size-16 items-center justify-center rounded-full bg-black/50 text-white">
            <Icon name="play_arrow" filled size={40} />
          </div>
        </div>
      )}

      <div
        className={cx(
          'absolute inset-x-0 top-0 flex items-center gap-1 bg-gradient-to-b from-black/50 to-transparent p-2 pb-10 text-white transition-opacity duration-200',
          'focus-within:opacity-100 group-hover/short:opacity-100',
          isPlaying ? 'opacity-0' : 'opacity-100',
        )}
      >
        <OverlayButton
          icon={isPlaying ? 'pause' : 'play_arrow'}
          label={isPlaying ? 'Pause (k)' : 'Play (k)'}
          onClick={togglePlay}
        />
        <OverlayButton
          icon={isMuted ? 'volume_off' : 'volume_up'}
          label={isMuted ? 'Unmute (m)' : 'Mute (m)'}
          onClick={onToggleMute}
        />
      </div>

      {error ? (
        <PlaybackErrorOverlay
          error={error}
          source={video.source}
          onRetry={retry}
          onOpenElsewhere={onOpenElsewhere}
        />
      ) : (
        <div className="absolute inset-x-0 bottom-0 z-10">
          <ProgressBar
            currentTime={currentTime}
            duration={duration}
            bufferedEnd={bufferedEnd}
            onSeek={seekTo}
          />
        </div>
      )}
    </div>
  );
}
