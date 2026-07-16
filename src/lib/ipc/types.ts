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
