import { convertFileSrc, invoke } from '@tauri-apps/api/core';

/** Typed wrappers for the Rust commands in `src-tauri/src/commands`. */

export type AuthStatus = {
  configured: boolean;
  connected: boolean;
  email: string | null;
  displayName: string | null;
};

export type TitleSource = 'custom' | 'ai' | 'fileName';

/** Where a video's file is: in Google Drive, or in a folder on this computer. */
export type VideoSource = 'drive' | 'local';

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
  /** The file's "Modified" date: for an upload, usually its date on the device it came from. */
  modifiedTime: string | null;
  /** When the file was uploaded to Drive or added to this computer, shown as "Date uploaded". */
  createdTime: string | null;
  /** The day the video was recorded, `YYYY-MM-DD`, from its file name or metadata. */
  recordedDate: string | null;
  folderId: string | null;
  folderName: string;
  hasThumbnail: boolean;
  md5Checksum: string | null;
  source: VideoSource;
  isFavorite: boolean;
  isWatchLater: boolean;
  lastPositionMs: number | null;
  watched: boolean;
  lastWatchedAt: string | null;
};

/** "Created" is the day a video was recorded; "uploaded" is when it was added to Drive. */
export type VideoOrder =
  'createdNewest' | 'createdOldest' | 'uploadedNewest' | 'uploadedOldest' | 'name';

/**
 * Home's shuffle: the same seed always gives the same order. `today` is the local `YYYY-MM-DD`,
 * which only the UI knows, for the On this day shelf.
 */
export type HomeMix = { seed: number; today: string };

export type VideoListRequest = {
  search?: string;
  folderId?: string;
  order?: VideoOrder;
  /** Lists Home's mix in place of `order`. */
  mix?: HomeMix;
  pageToken?: string;
  pageSize?: number;
};

export type VideoPage = {
  videos: VideoSummary[];
  nextPageToken: string | null;
  /** On the first page of a mix: videos recorded on this day in earlier years, newest first. */
  onThisDay?: VideoSummary[];
};

/** Shorts are vertical or square videos up to 3 minutes, newest recorded first or mixed. */
export type ShortsListRequest = {
  folderId?: string;
  mix?: HomeMix;
  pageToken?: string;
  pageSize?: number;
};

export type LibraryList = 'favorites' | 'watchLater' | 'history' | 'continueWatching';

export type FolderSummary = {
  id: string;
  name: string;
  videoCount: number;
  source: VideoSource;
};

export type DriveFolder = { id: string; name: string };

export type LocalFolder = {
  path: string;
  name: string;
  /** False while the folder is missing, for example on an unplugged drive. */
  available: boolean;
};

/** Where the library's videos come from. */
export type LibrarySources = {
  /** The Drive folder the library is limited to; `null` for all of Drive. */
  driveFolder: DriveFolder | null;
  localFolders: LocalFolder[];
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

/** Sent as the background scan reads the duration and size of each video on this computer. */
export type MediaInfoEvent = {
  driveId: string;
  durationMs: number | null;
  width: number | null;
  height: number | null;
};

export const MEDIA_INFO_EVENT = 'media-info://item';

export const api = {
  authStatus: () => invoke<AuthStatus>('auth_status'),
  authConnect: () => invoke<AuthStatus>('auth_connect'),
  authCancel: () => invoke<void>('auth_cancel'),
  authDisconnect: () => invoke<AuthStatus>('auth_disconnect'),

  videosList: (request: VideoListRequest) => invoke<VideoPage>('videos_list', { request }),
  shortsList: (request: ShortsListRequest) => invoke<VideoPage>('shorts_list', { request }),
  videoGet: (driveId: string) => invoke<VideoSummary>('video_get', { driveId }),
  /** The whole list, or one page of it. */
  libraryList: (list: LibraryList, page?: { offset: number; limit: number }) =>
    invoke<VideoSummary[]>('library_list', { list, ...page }),
  foldersList: () => invoke<FolderSummary[]>('folders_list'),

  librarySources: () => invoke<LibrarySources>('library_sources_get'),
  /** Opens the system folder picker; resolves unchanged when the user cancels. */
  libraryLocalFolderAdd: () => invoke<LibrarySources>('library_local_folder_add'),
  libraryLocalFolderRemove: (path: string) =>
    invoke<LibrarySources>('library_local_folder_remove', { path }),
  libraryDriveFolderSet: (folder: DriveFolder | null) =>
    invoke<LibrarySources>('library_drive_folder_set', { folder }),
  /** Folders inside a Drive folder, or at the top of My Drive without `parentId`. */
  driveFoldersList: (parentId?: string) =>
    invoke<DriveFolder[]>('drive_folders_list', { parentId }),

  playbackSource: (driveId: string) => invoke<PlaybackSource>('playback_source', { driveId }),
  /** Plays a Drive video on its Drive page, or a local one in the default player. */
  playbackOpenExternal: (driveId: string) => invoke<void>('playback_open_external', { driveId }),
  videoShowInFolder: (driveId: string) => invoke<void>('video_show_in_folder', { driveId }),
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

  /** A new thumbnail from the frame at `positionMs`, or else the most detailed of several. */
  thumbnailRegenerate: (driveId: string, positionMs?: number) =>
    invoke<void>('thumbnail_regenerate', {
      driveId,
      positionMs: positionMs == null ? null : Math.round(positionMs),
    }),
};

/** URL of a person's photo served from SQLite by the Rust `personphoto` protocol. */
export function personPhotoUrl(photoId: number): string {
  return convertFileSrc(String(photoId), 'personphoto');
}

/**
 * URL of the cached thumbnail served by the Rust `thumb` protocol. A new `version` after a
 * regenerate makes the webview load the new picture; Rust ignores it.
 */
export function thumbnailUrl(driveId: string, version = 0): string {
  const url = convertFileSrc(driveId, 'thumb');
  return version > 0 ? `${url}?v=${version}` : url;
}
