export interface SmokeResponse {
  service: "animi-core";
  version: string;
  message: string;
}

export interface CommandError {
  code: string;
  message: string;
  retryable: boolean;
}

/** UTC timestamp serialized as an RFC 3339 string, for example 2026-07-16T08:30:00.000Z. */
export type UtcTimestamp = string;
export type CalendarDate = string;

export type MediaKind = "ANIME" | "MANGA";
export type LibraryStatus = "WATCHING" | "COMPLETED" | "PLANNING";
export type ThemePreference = "SYSTEM" | "LATTE" | "MACCHIATO";

export interface ExportBackupResponse {
  schemaVersion: 1;
  exportedAt: UtcTimestamp;
  settingsCount: number;
  libraryEntryCount: number;
  watchActivityCount: number;
}

export interface ImportValidationIssue {
  code: string;
  message: string;
  path: string | null;
}

export interface ImportChangeSummary {
  settingsAdded: number;
  settingsUpdated: number;
  settingsKeptLocal: number;
  libraryAdded: number;
  libraryMerged: number;
  libraryKeptLocal: number;
  libraryConflicts: number;
  activityAdded: number;
  activityUnchanged: number;
  unrelatedLocalLibraryEntriesRetained: number;
}

export interface ImportPreviewResponse {
  fileName: string;
  schemaVersion: number | null;
  exportedAt: UtcTimestamp | null;
  settingsCount: number;
  libraryEntryCount: number;
  watchActivityCount: number;
  changes: ImportChangeSummary;
  validationErrors: ImportValidationIssue[];
  canApply: boolean;
  importToken: string | null;
  preservesUnrelatedLocalEntries: true;
}

export interface ImportBackupRequest {
  importToken: string;
}

export interface ImportBackupResponse {
  schemaVersion: number;
  changes: ImportChangeSummary;
}

export interface ClearLocalDataRequest {
  confirmation: string;
}

export interface ClearLocalDataResponse {
  settingsRemoved: number;
  libraryEntriesRemoved: number;
  watchActivityRemoved: number;
  cachedRecordsRemoved: number;
}

export interface SettingsOverview {
  cachedMediaCount: number;
  staleMediaCount: number;
  airingRecordCount: number;
  staleAiringCount: number;
  responseCacheCount: number;
  staleResponseCacheCount: number;
  latestCacheUpdateAt: UtcTimestamp | null;
}

export interface LibraryEntry {
  anilistId: number;
  mediaKind: MediaKind;
  status: LibraryStatus;
  progress: number;
  score: number | null;
  notes: string | null;
  createdAt: UtcTimestamp;
  updatedAt: UtcTimestamp;
}

export interface LibraryMediaTag {
  name: string;
  rank: number;
}

export interface LibraryMediaSnapshot {
  title: string;
  coverUrl: string | null;
  mediaFormat: string | null;
  genres: string[];
  tags: LibraryMediaTag[];
  status: string | null;
  episodes: number | null;
  durationMinutes: number | null;
  cachedAt: UtcTimestamp;
  expiresAt: UtcTimestamp;
  isStale: boolean;
}

export interface LibraryItem {
  entry: LibraryEntry;
  media: LibraryMediaSnapshot | null;
}

export interface WatchActivity {
  anilistId: number;
  mediaKind: MediaKind;
  progressDelta: number;
  episodeDurationMinutes: number | null;
  occurredAt: UtcTimestamp;
}

export interface DashboardLibraryTotals {
  tracked: number;
  watching: number;
  completed: number;
  planning: number;
  currentProgress: number;
}

export interface DashboardGenreSignal {
  label: string;
  titleCount: number;
}

export interface DashboardTagSignal {
  label: string;
  titleCount: number;
  weightedRank: number;
}

export interface DashboardPreferenceSignals {
  eligibleTitles: number;
  metadataTitles: number;
  missingMetadataTitles: number;
  staleMetadataTitles: number;
  genres: DashboardGenreSignal[];
  tags: DashboardTagSignal[];
}

export interface DashboardActivitySummary {
  eventCount: number;
  recordedEpisodes: number;
  knownDurationEpisodes: number;
  recordedMinutes: number;
  unknownDurationEpisodes: number;
  events: WatchActivity[];
}

export interface DashboardInsights {
  mediaKind: MediaKind;
  libraryItems: LibraryItem[];
  libraryTotals: DashboardLibraryTotals;
  preferenceSignals: DashboardPreferenceSignals;
  activity: DashboardActivitySummary;
}

export interface UpsertLibraryEntryRequest {
  anilistId: number;
  mediaKind: MediaKind;
  status: LibraryStatus;
  progress: number;
  /** Optional personal score in the inclusive 0–10 range. */
  score: number | null;
  notes: string | null;
}

export interface RemoveLibraryEntryRequest {
  anilistId: number;
  mediaKind: MediaKind;
  confirmed: boolean;
}

export interface RemoveLibraryEntryResponse {
  removed: boolean;
}

export interface MediaTag {
  name: string;
  rank: number;
}

export interface MediaSummary {
  anilistId: number;
  title: string;
  coverUrl: string | null;
  mediaFormat: string | null;
  status: string | null;
  episodes: number | null;
  durationMinutes: number | null;
  genres: string[];
  tags: MediaTag[];
  trending: number;
}

export interface Sequel {
  anilistId: number;
  title: string;
  releaseDate: CalendarDate | null;
}

export interface MediaDetails extends MediaSummary {
  sequels: Sequel[];
}

export interface Freshness {
  fetchedAt: UtcTimestamp;
  expiresAt: UtcTimestamp;
  isStale: boolean;
  refreshError: CommandError | null;
}

export interface MediaPageResponse {
  items: MediaSummary[];
  hasNextPage: boolean;
  freshness: Freshness;
}

export interface MediaDetailsResponse {
  media: MediaDetails;
  freshness: Freshness;
}

export interface SearchMediaRequest {
  search: string;
  page: number;
  perPage: number;
}

export interface MediaPageRequest {
  page: number;
  perPage: number;
}

export interface MediaDetailsRequest {
  anilistId: number;
}

export interface MetadataRefreshFailure {
  anilistId: number;
  error: CommandError;
}

export interface RefreshLibraryMetadataResponse {
  attempted: number;
  updated: number;
  stale: number;
  failures: MetadataRefreshFailure[];
}

export function isCommandError(value: unknown): value is CommandError {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.code === "string" &&
    typeof candidate.message === "string" &&
    typeof candidate.retryable === "boolean"
  );
}
