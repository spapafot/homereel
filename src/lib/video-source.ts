import { api, type VideoSource, type VideoSummary } from './api';
import { FILE_MANAGER_NAME } from './platform';
import { showToast } from './toast';

/** Shown under a folder's name, where YouTube shows a channel's subscriber count. */
export const FOLDER_KIND_LABELS: Record<VideoSource, string> = {
  drive: 'Google Drive folder',
  local: 'Folder on this computer',
};

/**
 * Plays the video outside Homereel, for formats it can't play: a Drive video on its Drive page
 * in the browser, a local one in the default player.
 */
export function openVideoElsewhere(video: Pick<VideoSummary, 'id' | 'source'>) {
  api.playbackOpenExternal(video.id).catch((error: unknown) => {
    console.error('Could not open the video outside Homereel', error);
    showToast(
      video.source === 'local' ? 'Could not open the video' : 'Could not open Google Drive',
    );
  });
}

/** Opens File Explorer or Finder with a local video selected. */
export function showVideoInFolder(videoId: string) {
  api.videoShowInFolder(videoId).catch((error: unknown) => {
    console.error('Could not show the video in its folder', error);
    showToast(`Could not open ${FILE_MANAGER_NAME}`);
  });
}

/**
 * Where the video lives: on its Drive page for Drive videos, in the file manager for local ones.
 * Used by "Open in Google Drive" and "Show in folder".
 */
export function revealVideo(video: Pick<VideoSummary, 'id' | 'source'>) {
  if (video.source === 'local') showVideoInFolder(video.id);
  else openVideoElsewhere(video);
}
