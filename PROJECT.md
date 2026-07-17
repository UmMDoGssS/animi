# Animi Desktop-First Plan

## Summary

Build Animi as a private, local-first Windows application:

- **Tauri 2** desktop shell
- **Svelte 5 + TypeScript + Vite** SPA
- **Rust** application and persistence layer
- **SQLite with `sqlx`** as the source of truth
- **`reqwest` + AniList GraphQL** for on-demand metadata
- **Tailwind CSS** for a responsive UI
- **Bun** for package management and frontend scripts

This matches the preference for TypeScript and Rust, produces a lightweight desktop application, and preserves a later Tauri mobile path. Tauri supports static web frontends across desktop and mobile without bundling Chromium. ([Tauri overview](https://v2.tauri.app/start/), [frontend configuration](https://v2.tauri.app/start/frontend/))

## Stack Decision

| Option | Advantages | Disadvantages | Decision |
|---|---|---|---|
| Tauri + Svelte + Rust + SQLite | Lightweight, genuinely local, TS/Rust, desktop and future mobile targets | More setup; requires learning web UI and Tauri IPC | **Selected** |
| TypeScript PWA + IndexedDB | Simplest phone/browser access and deployment | Per-browser storage, weaker backups, less-native desktop experience | Best runner-up |
| Flutter + SQLite | Strong native desktop/mobile UI from one codebase | Requires Dart; less web-native | Choose only if mobile becomes the immediate priority |
| Expo + React Native | Excellent TypeScript mobile development | No first-class Windows desktop target; SQLite web support remains alpha ([Expo SQLite](https://docs.expo.dev/versions/latest/sdk/sqlite/)) | Not desktop-first |
| Electron + React + SQLite | Mature and easy JS/TS desktop ecosystem | Bundles Chromium; typical zipped apps are around 80-100 MB; no mobile route ([Electron](https://www.electronjs.org/docs/latest/why-electron)) | Unnecessarily heavy |

## Implementation Architecture

- Create a Svelte SPA with Home, Discover, Dashboard, and Settings as the four primary destinations. Present media details as a secondary view and recommendations within Discover rather than as primary navigation. Use Svelte state primitives rather than adding a global state library.
- Keep all durable operations behind typed Tauri commands. Rust owns SQLite migrations, transactions, AniList requests, caching, throttling, and backup handling; the UI never executes SQL directly.
- Model anime and manga through a shared `MediaKind` schema from the start, but expose only anime workflows in v1. Manga becomes a later UI/API addition without redesigning storage.
- Store only locally meaningful records and metadata for titles the user searches, opens, or tracks. Do not bulk-copy AniList.
- Refresh airing information at application launch and every 30 minutes while open. Display cached data offline with a "last updated" indicator. Closed-app notifications are outside v1.
- Show sequel dates only when AniList contains an official sequel relation and release information. Never predict an unannounced season.
- Cache media details for 24 hours, discovery/recommendation results for 6 hours, and airing data for 30 minutes. Honor AniList rate-limit headers and `Retry-After`; its documented temporary limit is currently 30 requests/minute. ([AniList rate limits](https://docs.anilist.co/guide/rate-limiting))
- Generate recommendations locally: build a preference vector from genres and weighted AniList tags in completed/watching entries, rank untracked trending candidates using 55% tag similarity, 25% genre similarity, and 20% normalized trending score, and show the strongest matching tags as the explanation. Fall back to trending when history is insufficient.

## Primary Experience

- Home is the launch view. It prioritizes airing anime in the local `WATCHING` library through a keyboard-operable cover carousel. Each item shows local progress, the next known airing countdown, cached-data freshness, and a catch-up count derived from the next known episode and local progress. A bounded quick action advances watched progress. Missing schedules remain explicitly unknown.
- Discover provides anime search, trending, and recommendation sections as cover-focused card collections. Opening a card uses the shared secondary media-details presentation.
- Dashboard is a local personal overview, not an account profile. It presents library and status totals, supported genre and tag preferences, monthly and yearly watch activity, and a filterable, sortable library index paginated at five titles per page across watching, completed, and planning statuses.
- Dashboard preference signals use only `WATCHING` and `COMPLETED` anime with cached metadata. Planning and missing-metadata entries remain visible in library totals and the library index but do not fabricate preference signals; stale metadata remains usable offline and its coverage is disclosed. Genres count distinct titles, while tags retain each title's strongest rank and aggregate that weight deterministically.
- Settings owns theme, live cache-freshness information, backup, recovery, and destructive local-data controls. Clearing local data requires the exact confirmation phrase `CLEAR MY LOCAL DATA`; cancellation and every other confirmation value preserve the library.
- A floating segmented navigation rail on the left provides the four primary destinations. It remains keyboard operable and compact at narrow widths without hiding or clipping page content.
- Frosted-glass surfaces, rounded cover-focused layouts, and restrained cat kaomoji provide the visual character. Kaomoji may reinforce empty or catch-up states, but adjacent text must communicate the same status and decorative characters must not obscure controls.
- Monthly and yearly watch-time statistics come only from durable UTC watch activity recorded when progress changes, never from mutable library timestamps. Activity records retain the best-known episode-duration snapshot used for their calculation; unknown durations remain excluded and visibly identified. Existing data from before the activity migration is not fabricated retroactively.
- Progress history uses explicit baseline and correction semantics. Creating a tracked entry with nonzero progress establishes its current baseline without fabricating activity. Progress beyond an existing entry's already-recorded high-water mark appends exactly one activity event in the same transaction, containing only the newly unrecorded delta and an immutable snapshot of the best-known episode duration, which remains nullable when unknown. Equal or decreased progress appends no activity; a decrease is a correction to current state and never rewrites prior events, and restoring progress only up to the high-water mark does not count it twice. Removing a library entry preserves its user-owned activity history.

## Interfaces and Data

Core types:

- `MediaKind`: `ANIME | MANGA`
- `LibraryStatus`: `WATCHING | COMPLETED | PLANNING`
- `LibraryEntry`: AniList ID, kind, status, progress, timestamps, optional score and notes
- `MediaSnapshot`: titles, cover URL, format, genres, tags, status, episode/chapter totals, nullable episode duration, and cached timestamp
- `AiringInfo`: episode number, airing timestamp and fetched timestamp
- `WatchActivity`: AniList ID, media kind, progress delta, immutable nullable best-known episode-duration snapshot and UTC occurrence time
- `Recommendation`: media summary, local score and explanation labels

Tauri command boundary:

- `search_media`, `get_media_details`, `list_trending`
- `list_library`, `upsert_library_entry`, `advance_library_progress`, `remove_library_entry`
- `refresh_library_metadata`, `get_recommendations`
- `get_dashboard_insights`
- `get_settings_overview`, `export_backup`, `preview_import`, `import_backup`, `clear_local_data`

Backups use versioned JSON containing `schemaVersion`, `exportedAt`, settings, local library entries, and user-owned watch activity. Library backup records retain the internal activity high-water mark needed to prevent progress from being counted twice after recovery, and activity records carry stable event IDs for idempotent merging. An imported event ID with an identical payload is a no-op; the same event ID with a different payload is a validation error before any write transaction begins. Cached discovery data and images are excluded. Rust owns both native file pickers; import preview retains the fully validated payload in an in-memory session and returns only an opaque token to the webview. Apply recomputes the preview within its SQLite write transaction and rejects a stale preview before writing. Library and setting conflicts use the greatest `updatedAt`, with an equal timestamp keeping the local record. Library records with the same `createdAt` represent one incarnation and retain the greatest activity high-water mark; when `createdAt` differs, the winning incarnation keeps its own high-water mark. Import never deletes unrelated local entries.

There is no HTTP server, login, cloud database, analytics, or AniList OAuth in v1. Public AniList metadata does not require authentication. ([AniList authentication](https://docs.anilist.co/guide/auth/))

## Test Plan

- Rust unit and integration tests for migrations, library transitions, progress bounds, caching, recommendation scoring, rate-limit handling, and transactional backup imports.
- Mock AniList contract tests for missing episode counts, unknown sequels, offline responses, `403`, malformed GraphQL data, and `429 Retry-After`.
- Svelte component tests for the four-destination navigation, Home carousel states and progress actions, Discover cover grids, Dashboard summaries and the paginated library index, Settings, and loading, empty, offline, stale, and error states.
- End-to-end flows: search and add an anime, update progress from details and Home, restart and verify persistence, display next airing time and catch-up count, produce recommendations within Discover, render recorded watch activity, export, clear a test database, and restore.
- Acceptance requires a Windows NSIS build that retains data across upgrades and remains usable offline for previously cached titles.

## Assumptions and Prerequisites

- Animi remains private and local-only in v1. AniList's terms restrict competing noncomplementary tracker services and prohibit using its API as backup storage; public distribution must first obtain permission or introduce meaningful AniList account integration. ([AniList terms](https://docs.anilist.co/guide/terms-of-use))
- Phone support and live cross-device sync are deferred. Responsive layouts and platform-neutral DTOs must be maintained so a later Tauri mobile client can reuse them; manual transfer will use the versioned backup format.
- Adult content is excluded by default.
- Bun 1.3.14, Node 24, Rust 1.96, and Cargo are already installed. Use Bun for dependency installation, lockfile management, and frontend scripts.
- Before scaffolding, confirm or install Microsoft C++ Build Tools and WebView2, which Tauri requires on Windows. ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))
