import type { VideoSummary } from '@/lib/api';

export function makeVideo(overrides: Partial<VideoSummary> = {}): VideoSummary {
  return {
    id: 'vid1',
    title: 'IMG_0817.m4v',
    titleSource: 'fileName',
    customTitle: null,
    aiTitle: null,
    aiTitleStatus: null,
    fileName: 'IMG_0817.m4v',
    mimeType: 'video/mp4',
    durationMs: 75_000,
    width: 1920,
    height: 1080,
    sizeBytes: 22_400_000,
    modifiedTime: '2026-10-14T12:00:00Z',
    createdTime: '2026-10-14T11:00:00Z',
    recordedDate: null,
    folderId: 'fold1',
    folderName: 'Holidays',
    hasThumbnail: true,
    md5Checksum: 'md5',
    source: 'drive',
    isFavorite: false,
    isWatchLater: false,
    lastPositionMs: null,
    watched: false,
    lastWatchedAt: null,
    ...overrides,
  };
}
