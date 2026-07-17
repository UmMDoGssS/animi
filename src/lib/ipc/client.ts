import { invoke } from "@tauri-apps/api/core";

import type {
  CommandError,
  DashboardInsights,
  ExportBackupResponse,
  LibraryEntry,
  LibraryItem,
  MediaDetailsRequest,
  MediaDetailsResponse,
  MediaPageRequest,
  MediaPageResponse,
  RemoveLibraryEntryRequest,
  RemoveLibraryEntryResponse,
  RefreshLibraryMetadataResponse,
  SmokeResponse,
  ThemePreference,
  SearchMediaRequest,
  UpsertLibraryEntryRequest,
} from "./types";
import { isCommandError } from "./types";

export async function checkBackend(
  simulateError = false,
): Promise<SmokeResponse> {
  return invoke<SmokeResponse>("smoke_check", { simulateError });
}

export async function listLibrary(): Promise<LibraryItem[]> {
  return invoke<LibraryItem[]>("list_library");
}

export async function getDashboardInsights(): Promise<DashboardInsights> {
  return invoke<DashboardInsights>("get_dashboard_insights");
}

export async function upsertLibraryEntry(
  request: UpsertLibraryEntryRequest,
): Promise<LibraryEntry> {
  return invoke<LibraryEntry>("upsert_library_entry", { request });
}

export async function removeLibraryEntry(
  request: RemoveLibraryEntryRequest,
): Promise<RemoveLibraryEntryResponse> {
  return invoke<RemoveLibraryEntryResponse>("remove_library_entry", {
    request,
  });
}

export async function getThemePreference(): Promise<ThemePreference> {
  return invoke<ThemePreference>("get_theme_preference");
}

export async function setThemePreference(
  preference: ThemePreference,
): Promise<ThemePreference> {
  return invoke<ThemePreference>("set_theme_preference", { preference });
}

export async function exportBackup(): Promise<ExportBackupResponse | null> {
  return invoke<ExportBackupResponse | null>("export_backup");
}

export async function searchMedia(
  request: SearchMediaRequest,
): Promise<MediaPageResponse> {
  return invoke<MediaPageResponse>("search_media", { request });
}

export async function listTrending(
  request: MediaPageRequest,
): Promise<MediaPageResponse> {
  return invoke<MediaPageResponse>("list_trending", { request });
}

export async function getMediaDetails(
  request: MediaDetailsRequest,
): Promise<MediaDetailsResponse> {
  return invoke<MediaDetailsResponse>("get_media_details", { request });
}

export async function refreshLibraryMetadata(): Promise<RefreshLibraryMetadataResponse> {
  return invoke<RefreshLibraryMetadataResponse>("refresh_library_metadata");
}

export function normalizeCommandError(error: unknown): CommandError {
  if (isCommandError(error)) {
    return error;
  }

  return {
    code: "UNEXPECTED_ERROR",
    message: "Animi could not reach its local core.",
    retryable: true,
  };
}
