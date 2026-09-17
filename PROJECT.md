# Animi Product and Architecture

## Summary

Animi is a private, local-first Windows tracker and recommendation application. Version 1.0 shipped anime workflows; version 1.1 adds manga capabilities without weakening the existing anime experience or requiring a server.

- **Tauri 2** desktop shell
- **Svelte 5 + TypeScript + Vite** SPA
- **Rust** application and persistence layer
- **SQLite with `sqlx`** as the source of truth
- **`reqwest` + AniList GraphQL** for on-demand metadata
- **Tailwind CSS** for a responsive UI
- **Bun** for package management and frontend scripts

This stack produces a lightweight desktop application, preserves the TypeScript and Rust boundary, and keeps a later Tauri mobile path open. Tauri supports static web frontends across desktop and mobile without bundling Chromium. ([Tauri overview](https://v2.tauri.app/start/), [frontend configuration](https://v2.tauri.app/start/frontend/))

## Version 1.1 Scope

Version 1.1 exposes both `ANIME` and `MANGA` throughout the existing four-destination application. Manga mode covers AniList's complete `MANGA` media type, including the `MANGA`, `NOVEL`, and `ONE_SHOT` formats, and always shows the precise format returned by AniList. AniList provides chapter and volume totals for manga, while episode duration and airing schedules remain anime-only fields. ([AniList Media reference](https://docs.anilist.co/reference/object/media), [AniList media formats](https://docs.anilist.co/reference/enum/mediaformat))

The release includes:

- Manga search, trending results, details, cached metadata, and offline stale fallback.
- A manga library with Reading, Completed, and Planning presentation labels.
- Independent chapter and volume progress, with known-total validation and safe correction semantics.
- A Continue Reading Home experience that uses local progress and recent local activity.
- Manga-specific Dashboard totals, preference signals, reading activity, filters, and recommendations.
- A forward-only database migration and backup schema 2, while retaining import support for schema 1 backups from Animi 1.0.x.

The release does not include AniList OAuth or account synchronization, a content reader, chapter-level reading history, inferred chapter release schedules, reading-time estimates, mobile clients, cloud sync, background services, closed-app notifications, analytics, or public distribution.

## Implementation Architecture

- Retain Home, Discover, Dashboard, and Settings as the four primary destinations. Present media details as a secondary view and recommendations within Discover.
- Use one consistent media-kind selector where a view can operate on anime or manga. The selector is session-only and defaults to Anime on launch to preserve the version 1.0 startup experience. Do not mix episodes and chapters into one unlabeled progress or activity total.
- Keep all durable operations behind typed Tauri commands. Rust owns SQLite migrations, transactions, AniList requests, caching, throttling, recommendations, and backup handling; the UI never executes SQL or calls AniList directly.
- Resolve production storage from the stable Windows per-user application-data directory. Packaged release QA may redirect only the database root when both the exact `ANIMI_RELEASE_SMOKE=1` guard and a nonempty `ANIMI_RELEASE_SMOKE_DATA_DIR` are present; the directory value alone has no effect.
- Preserve `MediaKind` in identifiers, cache keys, repository keys, request DTOs, and response DTOs even where AniList identifiers are globally distinct. Cross-kind field combinations are validated rather than guessed.
- Store only locally meaningful records and metadata for titles the user searches, opens, or tracks. Do not bulk-copy AniList.
- Cache media details for 24 hours and discovery or recommendation results for 6 hours. Keep the existing 30-minute airing cache and refresh cycle for anime only. Honor AniList rate-limit headers and `Retry-After`. ([AniList rate limits](https://docs.anilist.co/guide/rate-limiting))
- Generate recommendations locally and independently per media kind. Build the preference vector from genres and weighted tags in completed or current entries of the selected kind; rank untracked candidates using 55% tag similarity, 25% genre similarity, and 20% normalized trending score. Fall back to same-kind trending results when history is insufficient.

## Primary Experience

- **Home:** Anime retains its airing-aware carousel, catch-up count, countdown, and bounded quick progress behavior. Manga adds a separate Continue Reading presentation for entries stored as `WATCHING` and labeled Reading in the UI. Continue Reading sorts by most recently changed local entry, offers a `+1 chapter` action bounded by a known chapter total, and makes no claim about unread published chapters or the next release date. Volume progress is edited deliberately in details or the library editor rather than inferred from chapters.
- **Discover:** A shared media-kind selector scopes search, trending, recommendations, empty states, cached results, and details to either anime or manga. Switching kind resets incompatible pagination and selection state. Manga cards and details show format, publication status, release date precision, chapter total, and volume total; unavailable totals are explicitly unknown. Official same-kind continuation relations may be shown, but Animi never predicts unannounced continuations or chapter dates.
- **Dashboard:** A media-kind selector provides separate anime and manga views. Manga presents tracked, Reading, Completed, and Planning totals; chapter and volume progress; reading activity; and genre and tag preferences. Library browsing and editing live on Home. Anime retains monthly and yearly watch-time reporting. Manga reports chapters and volumes only, because AniList does not provide a reliable reading duration. Activity values from different unit types are never added together.
- **Settings:** Theme, backup, recovery, destructive local-data controls, and the session-only state debugger continue to cover the whole application. Cache summaries identify anime and manga data where that distinction matters. Clearing local data still requires the exact confirmation phrase `CLEAR MY LOCAL DATA`.
- **Navigation and presentation:** Follow the approved September frontend mockup with a compact frosted header, Segoe UI/system typography, neutral rounded panel borders, a restrained diagonal Catppuccin rainbow, cover-led recommendation shelves, compact trending rows, four-column desktop statistics, and two-column Settings with theme previews. Keep the four keyboard-operable destinations, Latte and Macchiato themes, visible focus, reduced motion, and 390-pixel responsive layouts. Home exclusively exposes the searchable, filterable library in a single horizontally scrolling row of fixed-width cards so more titles do not increase page height; edits between the anime carousel and Home library refresh the neighboring view. The media selector is page-local in presentation and shared for the session across destinations. The manga Continue Reading carousel and quick chapter action remain P11 work.

## Progress and Activity Semantics

- `LibraryStatus` remains `WATCHING | COMPLETED | PLANNING` in durable and IPC contracts for compatibility. The UI labels `WATCHING` as Watching for anime and Reading for manga.
- `progress` is the primary consumed-unit count: episodes for anime and chapters for manga. `volumeProgress` is null for anime and a nonnegative integer, initially zero, for manga.
- Chapter and volume progress are independent user assertions. Neither is derived from the other. Each is nonnegative and cannot exceed its corresponding known total; an unknown total does not impose a guessed upper bound.
- Initial nonzero progress records the explicitly saved units. Progress beyond the relevant durable high-water mark records only the new delta in the same transaction. Equal or decreased progress is a correction and appends no activity; restoring progress only to the high-water mark does not count it twice.
- Generalized progress activity retains stable event IDs, media kind, primary-unit delta, volume delta, optional anime episode-duration snapshot, and a UTC occurrence time. Deltas are nonnegative and at least one must be positive. Anime events have a zero volume delta; manga events never contain episode-duration snapshots.
- Removing a library entry preserves its user-owned activity history. Metadata refreshes never overwrite local status, progress, score, notes, or activity.

## Interfaces and Data

Core types:

- `MediaKind`: `ANIME | MANGA`
- `LibraryStatus`: `WATCHING | COMPLETED | PLANNING`
- `LibraryEntry`: AniList ID, kind, status, episode-or-chapter progress, optional manga volume progress, timestamps, optional score, and notes
- `MediaSnapshot`: media kind, titles, cover URL, format, genres, tags, status, anime episode total and duration, manga chapter and volume totals, release date, and cached timestamp
- `AiringInfo`: anime-only episode number, airing timestamp, and fetched timestamp
- `ProgressActivity`: AniList ID, media kind, primary progress delta, optional volume delta, optional immutable episode-duration snapshot, and UTC occurrence time
- `Recommendation`: same-kind media summary, local score, and explanation labels

Tauri command boundary:

- `search_media`, `get_media_details`, `list_trending`, and `get_recommendations` accept `mediaKind`.
- `list_library`, `upsert_library_entry`, `advance_library_progress`, and `remove_library_entry` preserve `mediaKind`; manga upserts may include `volumeProgress`.
- `get_home_airing` remains anime-only; `get_home_reading` returns current manga using local library state.
- `refresh_library_metadata` and `get_dashboard_insights` accept a media-kind scope.
- `get_settings_overview`, `export_backup`, `preview_import`, `import_backup`, and `clear_local_data` cover both media kinds.

All IPC JSON fields use camelCase. Rust may retain idiomatic snake_case internally with explicit Serde renaming. Commands return typed success values and structured errors containing `code`, `message`, and `retryable`.

## Backup and Migration Compatibility

- Released database migrations remain append-only. The v1.1 migration upgrades every supported Animi 1.0.x profile in place, preserves all existing anime records and any pre-existing records carrying `MediaKind.MANGA` plus event IDs, initializes new volume fields safely, and remains reopenable after migration.
- Animi 1.1 exports backup `schemaVersion: 2`. Schema 2 contains settings, local anime and manga library entries, generalized progress activity, and both primary and volume activity high-water marks. Cached discovery data and images remain excluded.
- Import accepts schema 1 and schema 2. Schema 1 `progressDelta` is interpreted by its existing `mediaKind`: episodes for anime and chapters for manga; manga starts with zero volume progress. Event IDs and valid anime duration snapshots remain unchanged, while a schema 1 manga event with an impossible episode-duration snapshot is rejected instead of guessed. Export does not downgrade to schema 1.
- Import validates the complete payload before opening its write transaction. Rust-owned native file pickers keep the fully validated preview in memory and return only an opaque token to the webview. Apply recomputes the preview inside the transaction and rejects a stale preview before writing.
- Library and setting conflicts use the greatest `updatedAt`, with equal timestamps keeping local data. Records with the same `createdAt` represent one incarnation and retain the greatest compatible activity high-water marks; a different `createdAt` is a different incarnation. Import never deletes unrelated local entries.

## Test Plan

- Rust unit and integration tests cover append-only migration from every released schema, media-kind validation, independent chapter and volume bounds, correction and high-water behavior, generalized activity, repository keys, cache separation, recommendation isolation, and backup schemas 1 and 2.
- Mock AniList contract tests cover manga, novel, and one-shot formats; known and missing chapter or volume totals; finished and releasing statuses; relations; offline responses; malformed cross-kind data; partial GraphQL errors; `403`; and `429 Retry-After`.
- Svelte component tests cover the media-kind selector, kind-specific labels and units, Continue Reading, manga Discover and details, manga library editing and filters, Dashboard reading activity, recommendations, loading, empty, offline, stale, and error states.
- End-to-end flows cover manga search, details, add, chapter and volume updates, restart persistence, Continue Reading quick progress, Dashboard activity, same-kind recommendations, cached offline use, schema 2 export, schema 1 and 2 import, and preservation of an existing anime library.
- Acceptance requires a Windows NSIS 1.1 build that upgrades an isolated 1.0.1 profile without data loss and remains usable offline for previously cached anime and manga titles.

## Assumptions and Prerequisites

- Animi remains private and local-only in version 1.1. AniList's terms restrict competing noncomplementary tracker services and prohibit using its API as backup storage; public distribution must first obtain permission or introduce a compatible approved product change. ([AniList terms](https://docs.anilist.co/guide/terms-of-use))
- AniList is an on-demand metadata provider, not application storage. Public metadata requires no authentication. ([AniList authentication](https://docs.anilist.co/guide/auth/))
- Adult content remains excluded by default.
- Phone support and live cross-device sync remain deferred. Responsive layouts and platform-neutral DTOs must be maintained; manual transfer continues through the versioned backup format.
- Bun 1.3.14 or a newer compatible release, Node 24 or newer, Rust 1.96 or newer, Cargo, Microsoft C++ Build Tools, and WebView2 are required for development and packaging. ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))
