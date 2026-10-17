import { convertFileSrc, invoke } from '@tauri-apps/api/core';

/** Typed wrappers for the Rust commands in `src-tauri/src/commands`. */

export type AuthStatus = {
  configured: boolean;
  connected: boolean;
  email: string | null;
  displayName: string | null;
};

export type TitleSource = 'custom' | 'ai' | 'fileName';

export type VideoSummary = {
  id: string;
  /** Display title: the user's edit, else the AI title, else the filename. */
  title: string;
  titleSource: TitleSource;
  customTitle: string | null;
  aiTitle: string | null;
  aiTitleStatus: 'pending' | 'done' | 'failed' | null;
  fileName: string;
  mimeType: string;
  durationMs: number | null;
  width: number | null;
  height: number | null;
  sizeBytes: number | null;
  /** Drive's "Modified": for an upload, usually the file's date on the device it came from. */
  modifiedTime: string | null;
  /** When the file was uploaded to Drive, shown as "Date uploaded". */
  createdTime: string | null;
  /** The day the video was recorded, `YYYY-MM-DD`, from its file name or metadata. */
  recordedDate: string | null;
  folderId: string | null;
  folderName: string;
  hasThumbnail: boolean;
  md5Checksum: string | null;
  isFavorite: boolean;
  isWatchLater: boolean;
  lastPositionMs: number | null;
  watched: boolean;
  lastWatchedAt: string | null;
};

/** "Created" is the day a video was recorded; "uploaded" is when it was added to Drive. */
export type VideoOrder =
  'createdNewest' | 'createdOldest' | 'uploadedNewest' | 'uploadedOldest' | 'name';

export type VideoListRequest = {
  search?: string;
  folderId?: string;
  order?: VideoOrder;
  pageToken?: string;
  pageSize?: number;
};

export type VideoPage = {
  videos: VideoSummary[];
  nextPageToken: string | null;
};

export type LibraryList = 'favorites' | 'watchLater' | 'history' | 'continueWatching';

export type FolderSummary = {
  id: string;
  name: string;
  videoCount: number;
};

export type PlaylistSummary = {
  id: number;
  name: string;
  videoCount: number;
  firstVideoId: string | null;
  updatedAt: string;
};

export type PlaylistDetail = {
  playlist: PlaylistSummary;
  videos: VideoSummary[];
};

export type PlaylistMembership = {
  id: number;
  name: string;
  contains: boolean;
};

export type PlaybackSource = {
  url: string;
};

export type GeminiSettings = {
  keyConfigured: boolean;
  model: string;
  /** The owner's notes sent with every title request; empty when there are none. */
  instructions: string;
};

/** Every title request carries the notes, so a long text costs tokens on each video. */
export const MAX_GEMINI_INSTRUCTIONS = 2_000;

export type TitleQueueStatus = {
  running: boolean;
  scanning: boolean;
  stopping: boolean;
  total: number;
  completed: number;
  failed: number;
  pauseReason: string | null;
};

export type TitlesOverview = {
  queue: TitleQueueStatus;
  counts: { done: number; failed: number };
};

/** Which videos a run of the title queue covers; `all` skips videos the user titled. */
export type TitleScope = 'missing' | 'missingAndFailed' | 'all';

export type PersonKind = 'person' | 'animal';

/** A person or pet whose photos Gemini compares against every video it titles. */
export type Person = {
  id: number;
  name: string;
  kind: PersonKind;
  description: string | null;
  photoIds: number[];
};

export type PersonInput = {
  /** `null` creates a new person. */
  id: number | null;
  name: string;
  kind: PersonKind;
  description: string | null;
  keepPhotoIds: number[];
  /** Base64 JPEGs, without the `data:` prefix. */
  newPhotos: string[];
};

/** Each photo is sent with every title request, so the count is kept small. */
export const MAX_PERSON_PHOTOS = 3;

export type TitleItemEvent = {
  driveId: string;
  title: string | null;
  error: string | null;
};

export const TITLE_PROGRESS_EVENT = 'titles://progress';
export const TITLE_ITEM_EVENT = 'titles://item';

/** Sent as the background scan reads each video's recording date. */
export type RecordedDateEvent = {
  driveId: string;
  recordedDate: string;
};

export const RECORDED_DATE_EVENT = 'recorded-dates://item';
export const RECORDED_DATES_DONE_EVENT = 'recorded-dates://done';

export const api = {
  authStatus: () => invoke<AuthStatus>('auth_status'),
  authConnect: () => invoke<AuthStatus>('auth_connect'),
  authCancel: () => invoke<void>('auth_cancel'),
  authDisconnect: () => invoke<AuthStatus>('auth_disconnect'),

  videosList: (request: VideoListRequest) => invoke<VideoPage>('videos_list', { request }),
  videoGet: (driveId: string) => invoke<VideoSummary>('video_get', { driveId }),
  /** The whole list, or one page of it. */
  libraryList: (list: LibraryList, page?: { offset: number; limit: number }) =>
    invoke<VideoSummary[]>('library_list', { list, ...page }),
  foldersList: () => invoke<FolderSummary[]>('folders_list'),

  playbackSource: (driveId: string) => invoke<PlaybackSource>('playback_source', { driveId }),
  playbackOpenInBrowser: (driveId: string) => invoke<void>('playback_open_in_browser', { driveId }),
  playbackOpenHevcStore: () => invoke<void>('playback_open_hevc_store'),

  setFavorite: (driveId: string, value: boolean) =>
    invoke<VideoSummary>('video_set_favorite', { driveId, value }),
  setWatchLater: (driveId: string, value: boolean) =>
    invoke<VideoSummary>('video_set_watch_later', { driveId, value }),
  setCustomTitle: (driveId: string, title: string | null) =>
    invoke<VideoSummary>('video_set_custom_title', { driveId, title }),
  recordProgress: (driveId: string, positionMs: number, durationMs: number) =>
    invoke<void>('video_record_progress', { driveId, positionMs, durationMs }),
  historyRemove: (driveId: string) => invoke<void>('history_remove', { driveId }),
  historyClear: () => invoke<void>('history_clear'),

  playlistsList: () => invoke<PlaylistSummary[]>('playlists_list'),
  playlistGet: (playlistId: number) => invoke<PlaylistDetail>('playlist_get', { playlistId }),
  playlistCreate: (name: string, driveId?: string) =>
    invoke<PlaylistSummary>('playlist_create', { name, driveId }),
  playlistRename: (playlistId: number, name: string) =>
    invoke<void>('playlist_rename', { playlistId, name }),
  playlistDelete: (playlistId: number) => invoke<void>('playlist_delete', { playlistId }),
  playlistAddVideo: (playlistId: number, driveId: string) =>
    invoke<void>('playlist_add_video', { playlistId, driveId }),
  playlistRemoveVideo: (playlistId: number, driveId: string) =>
    invoke<void>('playlist_remove_video', { playlistId, driveId }),
  playlistReorder: (playlistId: number, driveIds: string[]) =>
    invoke<void>('playlist_reorder', { playlistId, driveIds }),
  playlistsForVideo: (driveId: string) =>
    invoke<PlaylistMembership[]>('playlists_for_video', { driveId }),

  geminiSettings: () => invoke<GeminiSettings>('gemini_settings_get'),
  geminiKeySet: (apiKey: string) => invoke<GeminiSettings>('gemini_key_set', { apiKey }),
  geminiKeyRemove: () => invoke<GeminiSettings>('gemini_key_remove'),
  geminiModels: () => invoke<string[]>('gemini_models_list'),
  geminiModelSet: (model: string) => invoke<GeminiSettings>('gemini_model_set', { model }),
  geminiInstructionsSet: (instructions: string) =>
    invoke<GeminiSettings>('gemini_instructions_set', { instructions }),
  titlesStatus: () => invoke<TitlesOverview>('titles_status'),
  titlesStart: (scope: TitleScope) => invoke<TitleQueueStatus>('titles_start', { scope }),
  titlesPause: () => invoke<TitleQueueStatus>('titles_pause'),
  titleRegenerate: (driveId: string) => invoke<VideoSummary>('title_regenerate', { driveId }),

  peopleList: () => invoke<Person[]>('people_list'),
  personSave: (input: PersonInput) => invoke<Person>('person_save', { input }),
  personDelete: (personId: number) => invoke<void>('person_delete', { personId }),
  videoPeople: (driveId: string) => invoke<string[]>('video_people', { driveId }),
  /** One frame as a JPEG data URL. */
  videoFrameCapture: (driveId: string, positionMs: number) =>
    invoke<string>('video_frame_capture', { driveId, positionMs: Math.round(positionMs) }),
};

/** URL of a person's photo served from SQLite by the Rust `personphoto` protocol. */
export function personPhotoUrl(photoId: number): string {
  return convertFileSrc(String(photoId), 'personphoto');
}

/** URL of the cached Drive thumbnail served by the Rust `thumb` protocol. */
export function thumbnailUrl(driveId: string): string {
  return convertFileSrc(driveId, 'thumb');
}
