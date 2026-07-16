import { invoke } from "@tauri-apps/api/core";

import type {
  CommandError,
  LibraryEntry,
  MediaDetailsRequest,
  MediaDetailsResponse,
  MediaPageRequest,
  MediaPageResponse,
  RemoveLibraryEntryRequest,
  RemoveLibraryEntryResponse,
  RefreshLibraryMetadataResponse,
  SmokeResponse,
  SearchMediaRequest,
  UpsertLibraryEntryRequest,
} from "./types";
import { isCommandError } from "./types";

export async function checkBackend(
  simulateError = false,
): Promise<SmokeResponse> {
  return invoke<SmokeResponse>("smoke_check", { simulateError });
}

export async function listLibrary(): Promise<LibraryEntry[]> {
  return invoke<LibraryEntry[]>("list_library");
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
