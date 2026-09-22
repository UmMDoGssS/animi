# Animi Delivery Plan

## Plan Status

| Field | Value |
|---|---|
| Product | Animi |
| Release | Version 1.1, Linux-first desktop |
| Overall status | Version 1.0.1 complete; version 1.1 in progress; private distribution only |
| Current phase | P10 complete; P11 in progress |
| Schedule | Dependency-driven; no calendar commitment |
| Last updated | 2026-09-22 |

This document is the living execution roadmap for Animi. [`PROJECT.md`](PROJECT.md) is authoritative for product scope, architecture, fixed behavior, and technical decisions. This plan is authoritative for implementation order, milestone status, dependencies, and release readiness.

If product scope or architecture changes, approve and record the change in `PROJECT.md` first, then update the affected milestones here. Normal implementation progress updates only this file.

## How to Use This Plan

- `[ ]` means not complete and `[x]` means complete. Do not check a milestone for partial work.
- Milestone IDs are stable. Add new work under the relevant phase instead of renumbering existing milestones.
- A phase begins only after its dependencies and entry gate are satisfied.
- A phase is complete only when every required milestone and its exit gate are checked and the evidence field names the commands, tests, or artifacts that proved completion.
- Exit gates are cumulative. Later work must not regress an earlier gate.
- Record blockers beside the affected milestone. Do not mark blocked or deferred work complete.
- Keep implementation changes small enough to verify with focused tests before running the broader phase gate.
- Items under Future Roadmap are context, not approved version 1.1 work.

## Release Boundaries

Version 1.0.1 is the latest completed private, local-first, anime-focused Windows desktop release. Version 1.1 is an additive manga-capability release on the same architecture. The local SQLite database remains authoritative. Rust owns durable state, AniList access, caching, recommendations, backups, and imports; Svelte communicates with Rust only through typed Tauri commands.

Version 1.1 includes:

- AniList `MANGA` media, including manga, novel, and one-shot formats with explicit format labels.
- Manga discovery, details, local library management, Continue Reading, Dashboard insights, and same-kind recommendations.
- Independent chapter and volume progress with durable activity and correction-safe high-water marks.
- Append-only upgrade of every supported 1.0 database and import compatibility for backup schemas 1 and 2.

The following remain outside version 1.1:

- A built-in content reader, per-chapter history, inferred chapter availability or release schedules, and reading-time estimates.
- Mobile clients and live cross-device synchronization.
- A server, cloud database, accounts, analytics, or AniList OAuth.
- Closed-app refresh or notifications.
- Predicted or inferred sequel, continuation, episode, or chapter announcements.
- Bulk collection of AniList data or use of AniList as application storage.
- Public distribution until the AniList terms and permission concern in `PROJECT.md` is resolved.

## Roadmap Overview

| Phase | Objective | Depends on | Status | Exit gate |
|---|---|---|---|---|
| P0 | Scaffold and engineering foundation | Approved project baseline | Complete | Windows app launches and baseline quality checks pass |
| P1 | Local persistence and typed IPC | P0 | Complete | Durable library CRUD survives restart safely |
| P2 | AniList integration and caching | P1 | Complete | Mocked network, cache, offline, and rate-limit behavior passes |
| P3 | Four-destination application shell and design system | P0; coordinate with P1 contracts | Complete | Home, Discover, Dashboard, and Settings shell works accessibly at desktop and narrow widths |
| P4 | Discovery, details, Dashboard, and library workflow | P1, P2, P3 | Complete | Core discovery-to-library flow passes end to end |
| P5 | Home airing workflow, insights, and recommendations | P2, P4 | Complete | Home airing/progress, Dashboard insights, and deterministic recommendation behavior pass |
| P6 | Settings, backup, and recovery | P1, P3, P4 | Complete | Backup round trips and rollback guarantees pass |
| P7 | Release hardening and Windows packaging | P0-P6 | Complete | Full quality gate and production NSIS acceptance pass |
| P8 | Manga domain, persistence, and compatibility | P7; approved 1.1 contracts | Complete | Existing 1.0.x profiles upgrade without loss and manga progress is durable |
| P9 | AniList manga integration and cache isolation | P8 contracts; existing P2 transport | Complete | Mocked manga discovery, details, cache, offline, and throttling behavior passes |
| P10 | Media-aware application workflows | P8, P9; existing P3-P4 UI | Complete | Manga discovery-to-library and Dashboard flows pass end to end |
| P11 | Continue Reading, activity, and recommendations | P8-P10; existing P5 behavior | Not started | Manga Home, reading activity, and same-kind recommendations pass |
| P12 | Version 1.1 release hardening | P8-P11 | Not started | Full gate and NSIS upgrade acceptance pass from Animi 1.0.1 |

## Planned Contracts

The version 1.0 portions of these contracts are implemented. The version 1.1 additions are implementation targets and must remain compatible with the decisions in `PROJECT.md`.

### Shared domain types

- `MediaKind`: `ANIME | MANGA`; version 1.1 exposes both kinds through kind-scoped workflows.
- `LibraryStatus`: `WATCHING | COMPLETED | PLANNING`; `WATCHING` is presented as Reading for manga without changing the durable enum.
- `LibraryEntry`: AniList ID, media kind, status, primary progress, nullable volume progress, independent activity high-water marks, timestamps, optional score, and optional notes. Primary progress means episodes for anime and chapters for manga; volume progress is null for anime and a nonnegative integer for manga.
- `MediaSnapshot`: media kind, titles, cover URL, format, genres, weighted tags, status, anime episode total and duration, manga chapter and volume totals, release date, and cache timestamp.
- `AiringInfo`: anime-only episode number, airing timestamp, and fetch timestamp.
- `ProgressActivity`: AniList ID, media kind, nonnegative primary and volume deltas with at least one positive, optional best-known anime episode duration, stable event ID, and UTC occurrence time.
- `Recommendation`: same-kind media summary, local score, and concise explanation labels.
- `CommandError`: `code`, `message`, and `retryable` at the IPC boundary.

All JSON fields crossing the Tauri boundary use camelCase. Rust may use snake_case internally with explicit Serde renaming. Timestamps cross the boundary as UTC values in one documented representation.

### Tauri commands

| Area | Commands | Planned phase |
|---|---|---|
| Discovery | `search_media`, `get_media_details`, `list_trending` | P2/P4 anime; add `mediaKind` in P9-P10 |
| Library | `list_library`, `upsert_library_entry`, `advance_library_progress`, `remove_library_entry` | P1/P4-P5 anime; extend for manga in P8/P10-P11 |
| Home | `get_home_airing`, `get_home_reading` | P5 anime airing; add manga reading in P11 |
| Metadata | `refresh_library_metadata` | P2/P4-P5 anime; add media-kind scope in P9-P10 |
| Recommendations | `get_recommendations` | P5 anime; add media-kind scope in P9/P11 |
| Dashboard | `get_dashboard_insights` | P4-P5 anime; add media-kind scope and reading aggregates in P8/P10-P11 |
| Backup | `export_backup`, `preview_import`, `import_backup` | P6 schema 1; add schema 2 and schema 1 compatibility in P8 |

Commands remain thin adapters. Validation, database behavior, AniList behavior, recommendation logic, and backup logic live in testable Rust modules.

`list_library` returns authoritative local entry fields enriched with optional cached media presentation fields and freshness needed by Home, Dashboard, and library cards. Missing cached metadata must not hide a local entry, and metadata refreshes never overwrite local status, progress, score, notes, or activity.

### Backup envelope

Animi 1.1 exports versioned JSON with `schemaVersion: 2`, `exportedAt`, settings, local anime and manga library entries, and user-owned generalized progress activity. Backup-only library records preserve both primary and volume activity high-water marks, and activity carries stable event IDs so repeated imports are idempotent. Import accepts schema 1 and schema 2. Schema 1 `progressDelta` is interpreted by its existing `mediaKind` as anime episodes or manga chapters, manga volume progress starts at zero, and event IDs plus valid anime duration snapshots remain unchanged; an impossible manga duration snapshot is a validation error. An existing event ID with an identical payload is a no-op; a reused event ID with a different payload is a validation error before writes. Cache records and downloaded images are excluded. Import validates the complete file before opening one write transaction. Library and setting conflicts use the greatest `updatedAt`, with equal timestamps keeping local data. Same-`createdAt` library records retain the greatest compatible activity high-water marks; different `createdAt` values are separate incarnations and the winning incarnation retains its own high-water marks. Import never deletes unrelated local entries.

## Cross-Cutting Definition of Done

Every applicable milestone must satisfy these requirements, even when they are not repeated in its checklist.

### Data and architecture

- SQLite foreign-key enforcement is enabled and tested on every database connection.
- SQL is parameterized, timestamps are UTC, and multi-record writes are transactional.
- Released migrations are append-only and tested from every supported prior schema.
- No production path uses `unwrap` or `expect`.
- Blocking database or file work stays off the async runtime; locks and transactions are not held across network requests.
- The frontend neither executes SQL nor calls AniList directly.

### Network and offline behavior

- AniList requests are on demand and do not bulk-collect metadata.
- Cache TTLs, stale presentation, rate-limit headers, and `Retry-After` follow `PROJECT.md`.
- Previously cached data remains usable offline and identifies stale data when it affects the current view.
- Automated tests never contact the live AniList API.

### User experience and safety

- Loading, empty, offline, stale, and error states terminate clearly; failures never leave an indefinite spinner.
- Destructive operations require explicit confirmation and never silently discard library entries.
- Both Catppuccin palettes maintain readable contrast, visible focus, keyboard operation, meaningful alternative text, and reduced-motion behavior.
- Important layouts work at a representative desktop viewport and a 390-pixel-wide responsive viewport.
- Cat styling remains decorative and never obscures information or controls.

### Verification

- Changed behavior has focused automated coverage at the lowest practical layer.
- Critical user-flow changes update end-to-end coverage.
- Formatting and lockfiles are current; Bun is the only frontend package manager.
- The final diff contains no unrelated changes, secrets, real user data, or accidental scope expansion.

## P0 - Scaffold and Engineering Foundation

**Goal:** Establish a minimal Windows application and reliable development loop before domain features begin.

**Dependencies:** The architecture and v1 boundaries in `PROJECT.md` are approved.

### Entry gate

- [x] **P0-E1** Confirm the repository is still pre-scaffold or reconcile any newly added user work before generating files.
- [x] **P0-E2** Verify Bun, Node, Rust, Cargo, Microsoft C++ Build Tools, WebView2, and the Tauri system prerequisites on the development machine.

### Milestones

- [x] **P0-M1 - Application scaffold:** Create a Tauri 2 application with a Svelte 5, strict TypeScript, Vite, and Bun frontend plus a Rust backend. Configure a stable application identifier and Windows app-data location before persistent data exists.
- [x] **P0-M2 - Styling foundation:** Add Tailwind CSS and the semantic CSS-variable layer needed by the Latte and Macchiato themes without embedding raw palette values in feature components.
- [x] **P0-M3 - Backend structure:** Establish thin command registration and separate Rust modules for commands, domain types, database access, AniList, backups, and recommendations. Empty modules should be added only when immediately needed.
- [x] **P0-M4 - Frontend structure:** Establish focused routes or view composition for Library, Discover, Details, Recommendations, and Settings/Backup using Svelte 5 state primitives and no global state library.
- [x] **P0-M5 - Quality tooling:** Configure Prettier, ESLint, `svelte-check`, Vitest, Svelte Testing Library, Rust formatting, Clippy, and Rust tests. Provide all scripts listed in the Quality Gates section.
- [x] **P0-M6 - Test isolation:** Provide deterministic fixture locations, a temporary database strategy, and a mock HTTP boundary so tests cannot reach AniList or real user storage.
- [x] **P0-M7 - Smoke application:** Render a minimal navigable shell through Tauri, verify frontend-to-Rust command invocation, and return a typed smoke response and structured smoke error.

### Exit gate

- [x] **P0-G1** The development application launches on Windows with no console or Rust panic.
- [x] **P0-G2** Baseline frontend formatting, linting, type checks, and tests pass.
- [x] **P0-G3** Baseline Rust formatting, Clippy, and tests pass with warnings denied.
- [x] **P0-G4** No npm, pnpm, or Yarn lockfile exists.

**Evidence:** Completed 2026-07-16. Entry checks found a clean pre-scaffold repository and verified Bun 1.3.14, Node 24.11.0, Rust/Cargo 1.96.1, the `x86_64-pc-windows-msvc` target, Visual Studio Community 18.1 with the C++ workload, and WebView2 Runtime 150.0.4078.65. `bun run format:check`, `bun run lint`, `bun run check`, and `bun run test` passed (4 frontend tests). `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`, and `cargo test --manifest-path src-tauri/Cargo.toml --all-features` passed (7 Rust tests). `bun run build` and `bun run tauri build --debug --no-bundle` produced `src-tauri/target/debug/animi.exe`; a timed Windows process smoke check confirmed the executable remained running without a startup panic. A lockfile scan found only `bun.lock` and `src-tauri/Cargo.lock`.

## P1 - Local Persistence and Typed IPC

**Goal:** Make SQLite-backed local state safe, testable, and accessible through typed commands.

**Dependencies:** P0 complete.

### Entry gate

- [x] **P1-E1** Database location and test-only temporary database injection are established without opening a real user database in tests.
- [x] **P1-E2** Planned DTO naming and UTC timestamp representation are documented next to the IPC types.

### Milestones

- [x] **P1-M1 - Initial migrations:** Add forward-migratable schema for library entries, settings, media snapshots, airing data, and response caches. Preserve `MediaKind` even though only anime is exposed in v1.
- [x] **P1-M2 - Connection safety:** Enable foreign keys for every connection, apply migrations at startup, map database failures to typed domain errors, and keep blocking work off the async runtime.
- [x] **P1-M3 - Library domain rules:** Implement validated status, progress, score, notes, identifiers, and timestamps. Reject negative progress and progress beyond a known total while handling unknown totals without guessing.
- [x] **P1-M4 - Repositories:** Implement parameterized, transactional repositories for library entries, settings, media snapshots, airing data, and cache records.
- [x] **P1-M5 - Library commands:** Implement `list_library`, `upsert_library_entry`, and `remove_library_entry` as thin adapters with camelCase DTOs and structured errors.
- [x] **P1-M6 - Persistence tests:** Cover fresh migrations, foreign keys, CRUD, status transitions, progress bounds, missing totals, update timestamps, restart persistence, and transaction rollback using temporary databases.
- [x] **P1-M7 - Migration fixtures:** Preserve a reusable pre-upgrade database fixture strategy so later migrations can be tested without rewriting history.

### Exit gate

- [x] **P1-G1** A library entry can be created, read, updated, and explicitly removed through the command boundary.
- [x] **P1-G2** Data survives a close-and-reopen test using an isolated application database.
- [x] **P1-G3** Invalid values and database failures return stable structured errors without partial writes.
- [x] **P1-G4** Migration, repository, and command-boundary tests pass without accessing user data.

**Evidence:** Completed 2026-07-16. Added embedded migration `0001_initial.sql`, async `sqlx` SQLite initialization in Tauri's app-data directory, per-connection foreign-key enforcement, typed repositories and errors, validated library domain rules, camelCase library commands, UTC RFC 3339 IPC timestamps, and an immutable migration-fixture strategy. Tests use only `tempfile` roots and cover migrations, foreign keys, repository round trips, CRUD and status changes, known and unknown totals, identifier/score/notes validation, deterministic update timestamps, explicit removal confirmation, restart persistence, typed database failures, and transaction rollback. `bun run format:check`, `bun run lint`, `bun run check`, and `bun run test` passed (5 frontend tests). `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`, and `cargo test --manifest-path src-tauri/Cargo.toml --all-features` passed (21 Rust tests). `bun run tauri build --no-bundle` produced `src-tauri/target/release/animi.exe` with the migration embedded; the executable was not launched so verification did not open a real user database. A lockfile scan found only `bun.lock` and `src-tauri/Cargo.lock`.

## P2 - AniList Integration and Caching

**Goal:** Provide typed, rate-limit-aware AniList metadata while retaining reliable offline behavior.

**Dependencies:** P1 complete.

### Entry gate

- [x] **P2-E1** AniList response fixtures cover normal, incomplete, malformed, forbidden, rate-limited, and offline cases.
- [x] **P2-E2** The HTTP client can be replaced with a deterministic mock in every automated test.

### Milestones

- [x] **P2-M1 - Typed GraphQL adapter:** Implement narrowly scoped queries and validated response mapping for anime search, details, trending results, airing schedules, and sequel relations. Exclude adult content by default.
- [x] **P2-M2 - Discovery commands:** Implement `search_media`, `get_media_details`, and `list_trending` without exposing raw GraphQL responses to the frontend.
- [x] **P2-M3 - Cache policy:** Apply 24-hour detail, 6-hour discovery/recommendation, and 30-minute airing TTLs. Cache only on-demand results and retain fetch timestamps needed for stale display.
- [x] **P2-M4 - Offline and stale reads:** Return usable cached data with explicit freshness metadata when refresh fails; return a terminal, retryable error when no usable cache exists.
- [x] **P2-M5 - Rate limiting:** Honor response rate-limit headers and `Retry-After`, avoid retry storms, and distinguish retryable throttling from non-retryable contract errors.
- [x] **P2-M6 - Library refresh:** Implement `refresh_library_metadata` so it refreshes only relevant tracked records and never holds a database transaction or lock during network work.
- [x] **P2-M7 - Contract tests:** Cover success, absent episode totals, absent sequel dates, partial GraphQL errors, malformed data, offline failures, `403`, and `429 Retry-After` entirely through mocks.

### Exit gate

- [x] **P2-G1** Discovery and detail commands return stable domain DTOs from live-shaped fixtures.
- [x] **P2-G2** Fresh cache avoids unnecessary requests and expired cache attempts refresh while retaining stale fallback.
- [x] **P2-G3** Rate-limit and offline behavior is deterministic and never corrupts local records.
- [x] **P2-G4** The automated suite proves no live AniList access is possible.

**Evidence:** Completed 2026-07-16. The first slice added a replaceable AniList transport, a network-disabled
test default, a production `reqwest` transport, narrow anime-only/non-adult GraphQL queries, stable
media/detail/airing/sequel mapping, and deterministic fixtures for all P2 contract cases. Verified
with `cargo test --manifest-path src-tauri/Cargo.toml --all-features` (24 unit tests and 4 integration
tests) and `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`.
The second slice added the three typed discovery commands, shared TypeScript IPC contracts, cache
policies with explicit freshness/error metadata, stale fallback, deterministic clock coverage, and
rate-limit cooldowns for `Retry-After` and `X-RateLimit-Reset`. Focused verification passed with 32
Rust unit tests, 4 Rust integration tests, 6 frontend tests, `svelte-check`, and Clippy with warnings
denied.
The final slice added anime-only tracked-library metadata refresh, typed per-title failures, and a
post-network transactional snapshot batch so database locks never span network requests and a
write failure cannot leave a partial refresh. P2 completed 2026-07-16 with the full quality gate
passing: frontend formatting, ESLint, `svelte-check`, 6 Vitest tests, Rust formatting, Clippy with
warnings denied, 34 Rust unit tests, and 5 Rust integration tests.

## P3 - Application Shell and Design System

**Goal:** Build the accessible, responsive four-destination interface foundation shared by every feature.

**Dependencies:** P0 complete. Coordinate state and error adapters with P1 before feature integration.

### Entry gate

- [x] **P3-E1** Semantic design tokens and async-state conventions are agreed in code before feature components consume them.
- [x] **P3-E2** Desktop and 390-pixel-wide test viewports are available to component or end-to-end tests.

### Visual approval checkpoints

Complete these checkpoints in order. After each checkpoint, launch the Tauri development application, provide a focused desktop and narrow-width test list, and wait for the user's visual report before beginning the next checkpoint.

- [x] **P3-V1 - Shell and navigation:** Replace the placeholder shell with the four primary page frames, horizontal top-right navigation in normal page flow, compact narrow layout, and shared frosted-glass visual language.
- [x] **P3-V2 - Home presentation:** Establish the cover-carousel composition and distinct on-time, catch-up, unknown-schedule, and no-active-anime presentations without claiming live airing integration before P5.
- [x] **P3-V3 - Discover presentation:** Establish search, cover-card collections, result states, and the secondary details presentation; live command integration remains P4.
- [x] **P3-V4 - Dashboard presentation:** Establish overview totals, preference summaries, truthful monthly and yearly activity-chart states, and the library-list region. Never fabricate history from mutable timestamps.
- [x] **P3-V5 - Settings and cross-view polish:** Consolidate Settings, kaomoji, async states, responsive behavior, reduced motion, and accessibility across all four destinations.

### Milestones

- [x] **P3-M1 - Theme system:** Implement Catppuccin Latte and Macchiato through semantic surface, overlay, text, muted-text, accent, success, warning, and danger variables.
- [x] **P3-M2 - Theme preference:** Default to the operating-system preference, persist a manual override through the durable settings boundary, and react predictably when the OS preference changes without an override.
- [x] **P3-M3 - Visual foundation:** Add rounded, cover-focused page frames for Home, Discover, Dashboard, and Settings plus restrained frosted-glass panels with subtle borders, restrained blur, and opaque fallbacks.
- [x] **P3-M4 - Navigation shell:** Implement a keyboard-operable floating vertical segmented control for Home, Discover, Dashboard, and Settings, keep it compact at narrow widths, and retain media details as a consistent secondary presentation.
- [x] **P3-M5 - Async state components:** Provide focused loading, empty, offline, stale, and error presentations with retry actions where the underlying error is retryable.
- [x] **P3-M6 - Accessibility:** Add visible focus, meaningful labels and alternative text, logical heading order, sufficient contrast, reduced-motion behavior, and non-color-only status communication.
- [x] **P3-M7 - Cat character:** Use restrained cat iconography or kaomoji for orientation, no-active-anime, catch-up, and other supportive states; provide equivalent status text and keep decoration secondary.
- [x] **P3-M8 - Layout tests:** Cover all four page frames, the horizontal navigation bar, Home carousel shell, Discover cover grid, Dashboard summaries and library region, Settings, and async states in both themes at desktop and narrow widths, including keyboard navigation and reduced motion.

### Exit gate

- [x] **P3-G1** Home, Discover, Dashboard, and Settings are each reachable and operable without a mouse.
- [x] **P3-G2** Both themes are readable and retain visible focus at desktop and narrow widths.
- [x] **P3-G3** Every asynchronous placeholder has a defined terminal success, empty, offline, or error state.
- [x] **P3-G4** Component accessibility and responsive-layout tests pass.

**Evidence:** P3 entry gates and P3-M1 completed 2026-07-16. Added explicit Catppuccin Latte and Macchiato selectors behind semantic color tokens, retained system dark-mode selection when no override exists, established a discriminated async-state convention, and added reusable 1280x800 desktop and 390x844 narrow component-test viewports. The user manually confirmed both palettes and the responsive shell at desktop and narrow widths. P3-M2 added accessible System, Light, and Dark controls; typed Tauri commands persist the preference in SQLite, System mode remains driven by the OS media query, failed saves retain the active palette, and themed text selection replaces the browser default. P3-M3, P3-M4, and P3-M7 were completed through the approved visual checkpoints: the four page frames share a terminal-frosted, translucent surface system with consistent reduced radii and mauve borders; the compact floating icon rail supports roving keyboard focus and activation at desktop and narrow widths; details remain a centered secondary presentation; and decorative cat ASCII and kaomoji accompany equivalent status text without obscuring controls. P3-V1 replaced the placeholder shell with the four primary page frames, a keyboard-operable icon rail, compact desktop and 390-pixel layouts, and that user-approved terminal-frosted visual language. The frontend quality gate passed with 14 tests; Rust formatting, Clippy with warnings denied, 36 unit tests, and 5 integration tests passed. P3-V2 added a cover-led Home carousel with distinct on-time, bounded catch-up, unknown-schedule, and no-active-anime sample states. The user manually approved its desktop and narrow layouts, carousel controls and keyboard behavior, preview-only progress interaction, empty-state route, both themes, and terminal-frosted styling; the frontend gate passed with 20 tests. P3-V3 added bundled fictional search and cover collections, explicit no-result recovery, a centered animated details dialog with unknown-data states, keyboard focus restoration, hidden scrollbar chrome, and a responsive contained cat empty state. The user manually approved the Discover presentation at desktop and narrow widths; the frontend gate passed with 27 tests. P3-V4 added a fictional local ledger with derived library totals, preference signals, truthful empty history, a keyboard-inspectable monthly contribution heatmap, yearly bars, unknown-duration exclusions, and three responsive library shelves. The user manually approved the Dashboard presentation at desktop and narrow widths; the frontend gate passed with 32 tests.

P3-V5 and P3 completed 2026-07-17. Settings now combines the persisted appearance control with a truthful local-state manifest plus preview-only cache-policy, backup, recovery, and destructive-data regions without claiming unfinished P6 behavior. A reusable async-state notice covers loading, empty, offline, stale, and error states; retryable theme load and save failures retain and retry the failed operation. Cross-view polish added accessible Latte text and control contrast, visible scroll affordances, focusable skip-link targeting, reduced-motion contracts, and deterministic scroll-to-top behavior when changing primary destinations. Component coverage increased to 56 frontend tests, including Settings at 1280x800 and 390x844, all async presentations, keyboard interactions, both theme token sets, contrast thresholds, and reduced-motion rules. Real-browser checks found no document-level horizontal overflow across all four destinations at either viewport and caught the fixed cross-view scroll carryover. `bun run format:check`, `bun run lint`, `bun run check`, `bun run test`, `bun run build`, Rust formatting, Clippy with warnings denied, and the full Rust suite passed (36 unit tests and 5 integration tests); the Rust suite used an isolated temporary target because the approved Tauri review process already held the normal debug executable open. The user manually approved the final Settings and cross-view checkpoint in both themes at desktop and narrow widths.

**Shell layout follow-up (2026-07-17):** The primary navigation is now a horizontal segmented control in the top-right application header that scrolls with the rest of the page, with compact icon-only controls at narrow widths and visible labels from the small breakpoint upward. Repeated descriptive title panels and the ambient background cat ASCII were removed so each page opens directly on its working content without decorative interference; screen-reader-only page headings retain the document outline. Navigation labels use a small optical downward adjustment within their unchanged button geometry.

## P4 - Discovery, Details, Dashboard, and Library Workflow

**Goal:** Deliver the core discovery-to-tracking workflow and local Dashboard/library experience backed by authoritative state.

**Dependencies:** P1, P2, and P3 complete.

### Entry gate

- [x] **P4-E1** Library and discovery read models are stable enough to render cover cards, Dashboard sections, aggregates, duration metadata, and watch activity from authoritative records without inferred history.
- [x] **P4-E2** Core user-flow tests run against isolated storage and mocked AniList responses.

### Milestones

- [x] **P4-M1 - Dashboard and library:** Present status counts and supported genre or tag summaries derived from local records, followed by a searchable, status-filtered, sortable library index in one continuously scrolling horizontal rail with cover art, progress, freshness, concise metadata, and useful empty states.
- [x] **P4-M2 - Discover search and trending:** Implement debounced anime search and a trending cover-card collection without persisting unneeded remote data.
- [x] **P4-M3 - Media details:** Present titles, cover, format, genres, tags, status, episode total when known, and cached-data freshness.
- [x] **P4-M4 - Add, update, and activity history:** Let users choose a supported status and update progress, score, and notes. Add an append-only activity migration. Initial nonzero progress establishes a baseline without activity; progress beyond an existing entry's recorded high-water mark appends one event in the same transaction for only the newly unrecorded delta, with an immutable nullable best-known episode-duration snapshot. Equal or decreased progress appends no event, decreases are corrections that do not rewrite history, restoring progress only to the high-water mark is not counted twice, and library removal preserves user-owned activity.
- [x] **P4-M5 - Validation feedback:** Explain progress bounds and malformed input without discarding the user's other edits; handle unknown episode totals explicitly.
- [x] **P4-M6 - Removal safety:** Require explicit confirmation before removal and make clear that removal affects the local library record rather than AniList.
- [x] **P4-M7 - Refresh behavior:** Refresh relevant metadata on demand while continuing to show cached entries offline or during transient failures.
- [x] **P4-M8 - Core end-to-end flow:** Cover Discover search, details, add, status and progress changes, Dashboard/library presentation, activity recording, restart persistence, cached offline use, and confirmed removal.

### Exit gate

- [x] **P4-G1** The complete discovery-to-library and Dashboard flow works from the packaged application boundary using mocks and isolated data.
- [x] **P4-G2** Local edits remain authoritative across metadata refreshes and application restarts.
- [x] **P4-G3** Offline, stale, empty, missing-total, validation, and command-error states are covered.
- [x] **P4-G4** Removal cannot occur without explicit user confirmation.

**Evidence:** P4 entry gates completed 2026-07-17. `list_library` joins authoritative local entries to optional cached presentation metadata without hiding entries that lack a snapshot. AniList details carry nullable episode duration, opened details persist snapshots, legacy duration-less detail caches are bypassed with a versioned key, and stale offline details preserve their original freshness. Append-only migration `0002` adds immutable duration snapshots and retained watch activity, while `0003` adds the persisted correction high-water mark; immutable v0001 and v0002 database fixtures prove upgrade preservation across both boundaries for library, settings, metadata, airing, cache, and activity rows. The typed `get_dashboard_insights` base contract returns one SQLite read snapshot containing anime-only library totals and items, deterministic watching/completed genre and weighted-tag signals with missing/stale coverage, plus retained activity events and known/unknown-duration totals without inferring history. A temporary-database flow now crosses the mocked search/details and library command helpers through add, enriched list, restart, rejected unconfirmed removal, and confirmed removal. Empty state, concurrent snapshot consistency, baselines, multiple increases, correction/restoration, known and unknown duration, removal retention, rollback, invalid duration, freshness overflow, stale fallback, deterministic aggregation, manga exclusion, and aggregate overflow are covered. The cumulative gate after entry-gate completion passed with `bun run format:check`, `bun run lint`, `bun run check`, `bun run test` (59 tests), `bun run build`, Rust formatting, Clippy with warnings denied, and the full Rust suite (51 unit tests and 6 integration tests). The normal Rust target remained locked by the open review application, so Clippy and the complete suite used an isolated temporary target that was removed afterward.

**Migration compatibility repair (2026-07-17):** Migration `0002` is frozen at the checksum first written to persistent databases, and its later activity-progress delta now lives in append-only migration `0003`. A narrowly validated one-time bridge canonicalizes databases created by the briefly committed combined-`0002` build without replaying the column alteration; unknown checksums, missing schema, and conflicting histories remain hard failures. Regression coverage starts from both v0001 and v0002 fixtures, preserves representative authoritative and activity data, verifies baseline and next-delta semantics, pins migration checksums, and reopens the upgraded database. The complete frontend gate passed with 86 tests, the complete Rust gate passed with 62 tests and warning-free Clippy, and `bun run tauri build --no-bundle` produced the optimized executable. The real `tauri dev` path migrated the affected app-data database and remained responsive; the release executable also remained responsive during its launch smoke, and the upgraded database passed SQLite integrity and foreign-key checks.

P4 completed 2026-07-17. Discover now uses the typed command boundary for trending, trimmed debounced search, immediate Enter submission, race-safe result replacement, and explicit loading, empty, offline, stale, retry, and error states. Opening a title presents cached freshness, cover fallback, format, release status, episodes, duration, genres, ranked tags, and sequel relations while saving only the useful metadata snapshot. The local editor preserves draft input across validation failures, supports status, bounded or explicitly unknown-total progress, optional score and notes, and requires a second explicit local-removal confirmation that explains AniList is unaffected and retained activity is not deleted. Dashboard renders authoritative totals, coverage-aware genre and weighted-tag signals, metadata-less entries, stale timestamps, raw retained activity and duration exclusions, and a five-title paginated library index with title and status filters plus recent, alphabetical, progress, and score sorting. Index mutations retain resilient focus and cached-offline behavior. A cross-view frontend workflow exercises search, details, baseline add, restart, edits, activity delta, cached offline use, confirmed removal, and retained activity. An official Tauri mock-runtime integration test crosses the real camelCase IPC command names and isolated SQLite storage through the same flow, including invalid-update rollback and unconfirmed removal rejection. Self-directed browser QA covered the populated Discover, details/editor, removal confirmation, and Dashboard at 1280x800 and 390x844 in Catppuccin Macchiato and Latte with no document-level horizontal overflow; keyboard focus, real-cover and fallback-cover presentation, modal containment, and runtime console output were checked, and the four navigation-ref warnings found during review were removed. The final gate passed Prettier, ESLint, `svelte-check` with zero diagnostics, 86 frontend tests across 13 files, the production frontend build, Rust formatting, Clippy with warnings denied, all 58 Rust tests including the Tauri IPC flow, and `tauri build --no-bundle`, which produced the release executable successfully.

**Library-index follow-up (2026-07-17):** The three always-expanded status shelves were consolidated into the documented five-title index. A seven-title component fixture covers both pagination directions, title and status filtering, all four sort modes, no-match recovery, and responsive desktop/narrow rendering; the cross-view P4 workflow continues to cover edits, restart persistence, cached offline use, and confirmed removal through the unified index. The status and sort controls use an accessible custom combobox/listbox with readable frosted surfaces, explicit pointer and keyboard hover states, outside-click dismissal, and trigger focus restoration. The current frontend gate passes 96 tests across 14 files. Standalone browser QA reached the expected Dashboard command-retry state because it does not provide a Tauri runtime, so populated index layout verification remains fixture-driven.

**Continuous-rail follow-up (2026-07-17):** Removed the five-title pagination controls and now renders every filtered and sorted library card in one horizontal rail. The rail exposes the shared themed scrollbar only when its widened cards overflow, and search, status, and sort changes return it to the start. The seven-title component fixture now verifies the complete result sequence, absent pagination controls, visible overflow treatment, all filters and sort modes, no-match recovery, and responsive card sizing. The complete frontend gate passes with 103 tests across 13 files.

## P5 - Home Airing, Dashboard Insights, and Recommendations

**Goal:** Add the Home airing and catch-up workflow, truthful Dashboard insights, and explainable local recommendations without predicting unavailable information.

**Dependencies:** P2 and P4 complete.

### Entry gate

- [x] **P5-E1** Airing and recommendation fixtures include missing dates, unknown sequels, insufficient history, and offline cache cases.
- [x] **P5-E2** Recommendation logic can run as a pure, deterministic Rust domain operation over supplied inputs.

### Milestones

- [x] **P5-M1 - Airing refresh scheduler:** Refresh relevant airing data at application launch and every 30 minutes while the application remains open, respecting cache and rate limits.
- [x] **P5-M2 - Home airing carousel:** Join `WATCHING` entries with next-known airing data. Show the countdown, stale or last-updated state, and catch-up count `max(0, nextEpisode - 1 - progress)`. Bound the quick action by the cached known episode total independently of schedule availability, label finished anime from cached release status, treat other missing schedules as unknown, and provide an explicit no-active-anime state.
- [x] **P5-M3 - Sequel presentation:** Show dates only for official sequel relations with release information. Label unknown sequels as unknown rather than estimating them.
- [x] **P5-M4 - Preference profile:** Build the local recommendation input from genres and weighted AniList tags in completed and watching entries without uploading library data.
- [x] **P5-M5 - Ranking algorithm:** Apply the `PROJECT.md` weighting of 55% tag similarity, 25% genre similarity, and 20% normalized trending score. Define normalization, tie-breaking, and the insufficient-history threshold as deterministic domain behavior before UI integration.
- [x] **P5-M6 - Candidate rules:** Rank untracked trending candidates, exclude already tracked titles, and fall back to trending when local history is insufficient.
- [x] **P5-M7 - Explanations:** Return the strongest matching tags as concise explanation labels without implying opaque certainty.
- [x] **P5-M8 - Recommendation section:** Present ranked, explainable results within Discover with loading, empty, offline, stale, and fallback states.
- [x] **P5-M9 - Deterministic tests:** Cover scoring weights, normalization edges, ties, exclusion, fallback, missing metadata, unknown sequels, cache expiry, refresh scheduling, Home catch-up counts, bounded quick progress, and activity aggregation using a controllable clock.
- [x] **P5-M10 - Dashboard activity insights:** Present recorded watch time as accessible monthly and yearly summaries, using a heatmap or bar treatment selected during visual review. Show the covered date range and unknown-duration exclusions, and provide useful empty states for users without recorded history.

### Exit gate

- [x] **P5-G1** Airing refresh occurs only at launch, on an allowed manual action, or at the in-app 30-minute interval.
- [x] **P5-G2** No test or UI path predicts an unannounced sequel or unknown date.
- [x] **P5-G3** Recommendation results and explanations are reproducible from fixed inputs.
- [x] **P5-G4** Insufficient history and offline operation produce useful, explicit fallbacks.

**Evidence:** Complete. Home remains mounted behind the four-destination shell so its launch check and 30-minute timer continue while the application is open; the only other schedule fetch is the explicit Check schedules action. Every fetch still passes through the 30-minute airing cache and AniList rate-limit handling. Successful schedule responses transactionally replace prior rows, including clearing them for a confirmed empty schedule, and partial per-title failures remain explicit. Home joins only anime `WATCHING` rows, reports stale and last-checked state, computes the documented schedule-based catch-up count, and labels cached `FINISHED` titles explicitly. Quick progress is independently bounded by the cached known episode total in one SQLite transaction, remains available without an airing schedule when episodes are unwatched, and retains activity high-water semantics.

The pure Rust recommendation domain requires two eligible cached `WATCHING` or `COMPLETED` titles before personalization, aggregates only local genre and strongest-per-title tag signals, applies 55% weighted-cosine tag similarity, 25% genre Jaccard overlap, and 20% min-max normalized trending, then breaks ties by score, trending, and AniList ID. Tracked candidates are excluded; insufficient history returns an explicit trending fallback; the strongest matching tags become explanation labels; and six-hour candidate caches remain useful offline with stale disclosure. Official sequel relations keep complete release dates only, while incomplete dates render as unknown without prediction. Dashboard activity now returns twelve ordered monthly buckets, lifetime yearly buckets, exact ledger coverage, and unknown-duration exclusions, rendered as accessible bars and totals.

Verification covers fixed-clock cache expiry and scheduling, empty schedules, stale offline recommendation candidates, insufficient history, score edges and ties, candidate exclusion, official and unknown sequel dates, responsive Home and Discover states, bounded transactional progress, monthly/yearly bucketing, and the real camelCase Tauri command surface in `src-tauri/tests/p5_ipc.rs`. The phase gate passes `bun run format:check`, `bun run lint`, `bun run check`, `bun run test`, `bun run build`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features`; the final suite contains 93 frontend tests across 13 files, 81 Rust unit tests, and 16 Rust integration/persistence tests.

## P6 - Settings, Backup, and Recovery

**Goal:** Give users control over local preferences and a safe, forward-migratable way to protect their library.

**Dependencies:** P1, P3, and P4 complete.

### Entry gate

- [x] **P6-E1** The current backup schema and supported import versions are documented next to the Rust backup types.
- [x] **P6-E2** File-dialog and filesystem operations can be replaced with temporary paths in automated tests.

### Milestones

- [x] **P6-M1 - Settings page:** Complete the Settings primary destination with theme selection, backup actions, and destructive local-data actions through accessible controls.
- [x] **P6-M2 - Versioned export:** Implement `export_backup` with `schemaVersion`, UTC `exportedAt`, settings, library entries, and user-owned watch activity while excluding caches and images.
- [x] **P6-M3 - Import validation:** Validate file readability, JSON shape, schema version, enums, timestamps, identifiers, progress, watch activity, duplicate records, and conflicting payloads that reuse an activity event ID before opening a write transaction.
- [x] **P6-M4 - Import preview:** Implement `preview_import` with counts, conflicts, validation errors, and a clear statement that unrelated local entries will remain.
- [x] **P6-M5 - Transactional import:** Implement `import_backup` in one write transaction; resolve library and setting duplicates by greatest `updatedAt` with equal timestamps keeping local data; preserve the greatest high-water mark only for library records sharing `createdAt`; let a winning different incarnation keep its own high-water mark; treat identical existing activity events as no-ops; and roll back every write if any application step fails.
- [x] **P6-M6 - Destructive confirmation:** Require explicit, specific confirmation before clearing local data and keep backup/import actions visually distinct from deletion.
- [x] **P6-M7 - Backup tests:** Cover empty and populated round trips including watch activity, malformed JSON, unsupported versions, invalid records, duplicate conflicts, repeated identical activity imports, conflicting reused event IDs, equal and newer local records, same- and different-incarnation high-water conflicts, unrelated local entries, forced mid-import failure, and rollback.
- [x] **P6-M8 - Recovery flow:** Add end-to-end coverage for export, clearing an isolated test database, previewing import, restoring, and verifying library and settings.

### Exit gate

- [x] **P6-G1** Exported backups round-trip without cached data and preserve supported user-owned fields.
- [x] **P6-G2** Invalid imports make no database changes and return actionable structured errors.
- [x] **P6-G3** Conflict previews match the final transactional result.
- [x] **P6-G4** No destructive action can silently discard library entries.

**Evidence:** Complete. Backup schema v1 and the supported import-version set are documented beside the Rust DTOs. Append-only migration `0004_watch_activity_identity.sql` gives activity events portable UUID identities while preserving local ordering, and the immutable v0003 fixture proves both the upgrade and the next post-upgrade event identity. Rust-owned native source and destination pickers keep filesystem paths out of IPC. `export_backup` reads one coherent SQLite snapshot and atomically writes only user-owned state. `preview_import` validates the complete bounded file, reports record counts and merge outcomes, and stores the validated payload behind an opaque in-memory token. `import_backup` recomputes that plan inside one write transaction, rejects stale previews, applies the documented timestamp and incarnation rules, treats identical activity events as no-ops, and rolls back on every failure. Settings presents theme, backup, an accessible restore preview, and an isolated clear-data flow requiring `CLEAR MY LOCAL DATA`. Empty and populated round trips, malformed and unsupported backups, invalid and duplicate records, activity identity conflicts, equal/newer local state, both high-water incarnation paths, unrelated retention, injected mid-apply rollback, picker cancellation, cache freshness, and export-clear-preview-restore all have deterministic coverage. The cumulative phase gate passes 91 frontend tests across 13 files, 71 Rust unit tests, and 15 Rust integration tests; final formatting, lint, type, build, Clippy, and production Tauri build evidence was refreshed at phase completion.

## P7 - Release Hardening and Windows Packaging

**Goal:** Prove v1 is safe to install, upgrade, use offline, back up, and recover on Windows.

**Dependencies:** P0 through P6 complete.

### Entry gate

- [x] **P7-E1** Every earlier phase gate is checked with current evidence and has no unresolved release blocker.
- [x] **P7-E2** Public distribution remains disabled unless the AniList terms and permission prerequisite in `PROJECT.md` has been resolved and documented.

### Milestones

- [x] **P7-M1 - Regression matrix:** Run critical flows on a fresh profile and an upgraded profile in light and dark themes, online and offline, at desktop and narrow widths.
- [x] **P7-M2 - Upgrade safety:** Test every released migration path, application data-directory continuity, settings preservation, and library persistence across an application upgrade.
- [x] **P7-M3 - Accessibility review:** Verify keyboard-only use, focus order, focus visibility, accessible names, alternative text, contrast, reduced motion, and status announcements on critical flows.
- [x] **P7-M4 - Failure review:** Exercise unavailable network, stale cache, `403`, `429`, malformed GraphQL, database failure, invalid backup, and interrupted recovery paths without indefinite spinners or partial writes.
- [x] **P7-M5 - Privacy and scope audit:** Confirm there is no server, analytics, cloud storage, OAuth, hidden synchronization, bulk AniList collection, closed-app notification behavior, or real user/test data in artifacts.
- [x] **P7-M6 - Dependency and configuration audit:** Review maintained dependencies, licenses, Tauri permissions/capabilities, production URLs, application identity, icons, versioning, and Bun lockfile integrity.
- [x] **P7-M7 - Windows package:** Produce the production Tauri NSIS installer and smoke-test installation, launch, persistence, upgrade, uninstall behavior, and documented data-location expectations.
- [x] **P7-M8 - Release notes:** Record delivered features, known limitations, backup compatibility, migration expectations, verification evidence, and any justified dependency additions.

### Version 1 acceptance gate

- [x] **P7-G1** Home airing and quick progress, Discover search and recommendations, details, Dashboard/library management and activity, Settings, export, preview, and import pass their critical user flows.
- [x] **P7-G2** Previously cached tracked titles remain usable offline with accurate freshness indicators.
- [x] **P7-G3** The production NSIS build retains local data across a tested upgrade.
- [x] **P7-G4** The complete frontend and Rust quality gate passes from a clean checkout.
- [x] **P7-G5** The final diff and packaged artifacts contain no secrets, real user data, unrelated changes, or non-Bun lockfiles.
- [x] **P7-G6** Any limitation that remains is documented and does not violate `PROJECT.md` or an earlier gate.

**Evidence:** Complete 2026-07-17. The P0-P6 evidence and gates were re-audited with no release blocker; public distribution remains explicitly disabled by `package.json`, Cargo publishing metadata, the proprietary repository license, updater configuration, release notes, and the unresolved AniList permission prerequisite. `docs/RELEASE_AUDIT.md` records the fresh/upgraded, online/offline, Latte/Macchiato, desktop/narrow regression matrix; critical-flow, failure, accessibility, privacy, dependency-license, capability, URL, identity, version, icon, lockfile, backup, migration, and data-location findings. Component coverage and a live 1280 × 800 / 390 × 844 browser pass verified semantic navigation, no narrow horizontal overflow, keyboard focus/selection, terminal IPC failure presentation, and the user-approved neutral cat icon with circular rainbow frame at 512 px and 32 px. Automated contrast checks keep semantic text at WCAG AA or better on both opaque fallback palettes, and reduced-motion, alternative-text, focus, live-region, dialog, and destructive-control behavior remain covered.

Every immutable v0001, v0002, and v0003 profile fixture upgrades to schema 7, preserves its `SYSTEM` theme and library progress, and reopens cleanly. A guarded packaged-smoke data-root override requires both `ANIMI_RELEASE_SMOKE=1` and a nonempty `ANIMI_RELEASE_SMOKE_DATA_DIR`; the exact-guard test proves normal startup ignores incomplete or malformed overrides. Against an isolated copied v0003 profile, the final package migrated all seven schemas and preserved AniList ID 101 at progress 3, its theme, and two activity events. Silent current-user installation, packaged launch, same-identity in-place reinstall, and uninstall passed; the profile hash stayed stable across reinstall, user data remained after uninstall as documented, installed files were removed, and no Animi uninstall registration remained.

The final clean-tree gate passes `bun run format:check`, `bun run lint`, `bun run check` with zero diagnostics, `bun run test` (110 tests across 14 files), `bun run build`, `bun run release:audit`, Cargo formatting, Clippy with warnings denied, and `cargo test --all-features` (84 unit tests plus 18 integration/persistence tests). No dependency was added. `bun run release:build` produced the private unsigned NSIS installer `src-tauri/target/release/bundle/nsis/Animi_1.0.0_x64-setup.exe` at 4,184,139 bytes with SHA-256 `0393694EC30906839646C7A61D2CA9D720E1A07BB7169F00E7E2993652209BEA`. The artifact audit found no updater, secret, real user data, bundled database/backup/cache, non-Bun frontend lockfile, or expanded Tauri capability; the only committed SQLite files are synthetic immutable migration fixtures.

## Completed Follow-up Work

- [x] **2026-07-19 - Session state debugger:** Add a Settings control that can force Home, Discover, and Dashboard into the shared loading, retryable error, offline, stale, or empty presentation without changing durable settings. Keep Settings reachable, allow the selected presentation to be replayed, reset the override on relaunch, and preserve reduced-motion behavior.

**Evidence:** The controlled Settings radios, inline preview, app-wide forced panel, replay behavior, return-to-Settings path, and reset to live content have component coverage. The frontend gate passes formatting, lint, Svelte type checking with zero diagnostics, all 113 tests across 14 files, and the production web build. Live browser checks at 1280px and 390px confirm the forced loader, control layout, keyboard-accessible semantics, zero document-level horizontal overflow, and no console errors.

- [x] **2026-07-19 - Details release date:** Show episodes, duration, and the authoritative AniList release date as separate summary rows while preserving day, month, or year precision and presenting missing dates explicitly.

**Evidence:** The details dialog renders theme-aware semantic dividers, a semantic `time` value for known release dates, and an explicit unknown state when AniList has no date. The focused component suite passes all 8 tests, Svelte type checking reports zero diagnostics, and formatting and diff checks pass.

- [x] **2026-07-19 - Library save feedback:** Replace the clipped edit-success banner with a disabled Save button that uses the same translucent glass state as Home's caught-up episode action whenever the form matches its durable saved state. Keep add and remove confirmations visible with clear spacing before the editor card.

**Evidence:** The library editor tracks status, progress, score, and notes against the latest successful save, enables the accent `Save changes` action only for a changed draft, and returns it to the muted `glass-control` state labeled `Changes saved` after persistence. All 15 focused editor and details-dialog tests pass without rendering a separate success banner; Svelte type checking reports zero diagnostics, and lint, formatting, and diff checks pass.

- [x] **2026-07-19 - Refresh icon controls:** Replace the visible Home schedule-refresh and Dashboard anime-metadata-refresh labels with a shared refresh icon while retaining accessible names, tooltips, disabled states, and active-refresh feedback.

**Evidence:** Both refresh controls use the same compact glass icon button, expose their purpose without visible text, and animate the icon only during active work under the existing reduced-motion safeguard. All 22 focused Home and Dashboard tests pass with the icon-only presentation and preserved command behavior; Svelte type checking reports zero diagnostics, and lint, formatting, and diff checks pass.

- [x] **2026-07-19 - Unknown episode card density:** Remove the redundant `episode total unknown` warning from Dashboard library cards while retaining `??` in the visible and accessible progress summary.

**Evidence:** All 11 focused Dashboard tests verify that unknown totals remain represented as `progress / ?? eps` without the duplicate warning line. Svelte type checking reports zero diagnostics, and lint, formatting, and diff checks pass.

- [x] **2026-07-19 - Library genre and tag filters:** Add multi-select genre and tag filters to the Dashboard library index, require every selected facet to match, and keep active selections visible and individually removable.

**Evidence:** The reusable glass multi-select and Dashboard integration have focused pointer and keyboard coverage, including multiple simultaneous genre and tag selections, conjunctive matching, visible active-filter chips, individual removal, and full reset. The complete frontend gate passes formatting, lint, Svelte type checking with zero diagnostics, all tests, and the production web build.

## Version 1.1 Execution Roadmap

Version 1.1 work begins at P8. P0-P7 and Completed Follow-up Work describe the shipped Animi 1.0.x baseline and must remain regression-safe.

## P8 - Manga Domain, Persistence, and Compatibility

**Objective:** Establish migration-safe manga contracts and durable chapter, volume, and activity behavior before any new UI or network workflow depends on them.

**Depends on:** P7; approved version 1.1 decisions in `PROJECT.md`.

### Entry gate

- [x] **P8-E1** Audit the current repository state, schema version, immutable migration fixtures, backup schema 1 fixtures, and unrelated user changes before editing.
- [x] **P8-E2** Freeze representative Animi 1.0.1 IPC, database, and backup fixtures that prove the pre-1.1 compatibility baseline.

### Milestones

- [x] **P8-M1 - Kind-aware contracts:** Add `mediaKind` to media summaries and every discovery, details, progress, recommendation, refresh, and Dashboard request where kind cannot be safely inferred. Add nullable `chapters`, `volumes`, and `volumeProgress` fields; require volume progress to be null for anime and nonnegative for manga, and reject other cross-kind combinations such as manga episode duration.
- [x] **P8-M2 - Append-only migration:** Keep the existing `progress` and `total_units` columns as kind-scoped primary values for anime episodes or manga chapters. Add manga volume total, volume progress, an independent volume activity high-water mark, and generalized activity deltas through new forward migrations only. Preserve every existing anime row and any pre-existing row carrying `MediaKind.MANGA`, plus event IDs, valid duration snapshots, timestamps, and every released migration file.
- [x] **P8-M3 - Progress rules:** Implement episode-or-chapter primary progress and independent manga volume progress. Enforce nonnegative values and known totals without guessing unknown totals; apply initial-save, correction, restore, and newly-unrecorded-delta behavior independently to both progress dimensions in one transaction.
- [x] **P8-M4 - Repositories and IPC:** Extend library, snapshot, activity, Dashboard, and settings repositories plus their thin Tauri adapters for both media kinds. Missing manga metadata must not hide a local entry, and removal must preserve generalized user-owned activity.
- [x] **P8-M5 - Backup schema 2:** Export schema 2 with anime and manga library records, generalized progress activity, stable event IDs, and both high-water marks. Import schemas 1 and 2 through one fully validated preview/apply flow; interpret schema 1 progress by its existing media kind, initialize manga volume progress to zero, reject impossible manga duration snapshots, and never export a lossy schema 1 file.
- [x] **P8-M6 - Compatibility fixtures:** Add immutable profile and backup fixtures covering fresh 1.1 data, every released pre-1.1 schema, schema 1 import, schema 2 round trip, duplicate imports, event-ID collision, stale preview, transaction rollback, and mixed anime/manga libraries.

### Exit gate

- [x] **P8-G1** Every supported Animi 1.0.x profile upgrades to the new schema, preserves all anime data and event IDs, and reopens cleanly.
- [x] **P8-G2** Manga library CRUD, chapter progress, volume progress, corrections, high-water behavior, removal, and restart persistence pass through the command boundary.
- [x] **P8-G3** Backup schema 2 round trips mixed libraries, schema 1 restores anime data without loss, and repeated import remains idempotent.
- [x] **P8-G4** Invalid kind/field combinations and simulated database failures return structured errors without partial writes.

**Evidence:** Complete 2026-08-02. Append-only migration `0008_manga_persistence.sql` keeps primary progress kind-scoped, adds independent manga volume totals, progress, high-water state, and activity deltas, and rebuilds constrained tables without changing released migrations. A synthetic immutable Animi 1.0.1 schema-7 profile includes both anime and a pre-existing manga row; upgrade tests preserve local fields, event IDs, valid duration snapshots, timestamps, settings, cache records, and all prior migration checksums through restart. Kind-aware Rust and strict TypeScript contracts now carry `mediaKind`, chapter and volume totals, volume progress, and media-scoped discovery, details, recommendation, refresh, progress, and Dashboard requests. Manga metadata transport remains explicitly unavailable with a structured error until P9 rather than crossing the anime cache boundary.

Manga command-boundary coverage proves chapter and volume bounds, unknown totals, independent initial/correction/restore/high-water deltas, missing metadata, retained activity after removal, transactional validation failures, and restart persistence. Backup export now emits schema 2 only; one normalized preview/apply path imports schemas 1 and 2, initializes schema-1 manga volume state to zero, rejects manga duration snapshots, preserves both high-water marks and stable event identities, and retains the established duplicate, collision, stale-preview, idempotency, and rollback guarantees. Immutable schema-1 and schema-2 mixed-media backup fixtures document the compatibility boundary. The completed gate passes `bun run format:check`, `bun run lint`, `bun run check` with zero diagnostics, `bun run test` (122 tests across 15 files), `bun run build`, Cargo formatting, Clippy with warnings denied, and `cargo test --all-features` (88 unit tests plus 20 integration/persistence tests). No dependency or non-Bun lockfile was added, and automated tests made no live AniList requests.

## P9 - AniList Manga Integration and Cache Isolation

**Objective:** Extend the existing mocked AniList boundary to manga while keeping anime and manga requests, caches, and recommendations isolated.

**Depends on:** P8 contracts and the existing P2 transport, cache, and rate-limit behavior.

### Entry gate

- [x] **P9-E1** Deterministic fixtures exist for manga, novel, one-shot, missing totals, malformed data, partial GraphQL errors, offline, `403`, and `429 Retry-After` responses.
- [x] **P9-E2** The request builder and cache-key strategy can express media kind without duplicating the transport or bypassing test network isolation.

### Milestones

- [x] **P9-M1 - Kind-parameterized queries:** Parameterize search, trending, details, and tracked-library queries by `MediaKind`; request chapters and volumes for manga and episodes and duration for anime. Keep airing queries anime-only and exclude adult content for both kinds.
- [x] **P9-M2 - Validated mapping:** Map manga formats, publication statuses, fuzzy release dates, chapter and volume totals, genres, weighted tags, and official same-kind continuation relations into stable DTOs. Treat missing totals as unknown and reject impossible or negative provider values.
- [x] **P9-M3 - Discovery commands and caches:** Extend `search_media`, `get_media_details`, and `list_trending` with media-kind input. Version cache payloads and include kind in every cache key so one kind cannot satisfy the other's request; retain the existing 24-hour details and 6-hour discovery TTLs.
- [x] **P9-M4 - Metadata refresh:** Refresh tracked anime and manga in bounded same-kind batches without holding a database transaction across network work. Preserve per-title failures, stale fallback, and authoritative local fields.
- [x] **P9-M5 - Recommendation candidates:** Fetch, cache, and rank recommendation candidates within the requested media kind only. Manga profiles use Reading and Completed manga; anime profiles continue to use Watching and Completed anime.
- [x] **P9-M6 - Contract coverage:** Cover pagination, cache hits, expired refresh, offline stale reads, empty uncached failure, kind mismatch, missing chapter or volume totals, supported manga formats, relations, malformed GraphQL data, `403`, and `429 Retry-After` entirely through mocks.

### Exit gate

- [x] **P9-G1** Manga discovery and details return stable kind-aware DTOs from live-shaped fixtures without contacting AniList in tests.
- [x] **P9-G2** Anime and manga cache entries cannot collide, and both kinds retain deterministic fresh, stale, offline, and throttled behavior.
- [x] **P9-G3** Manga metadata refresh updates only cache-owned fields and leaves local progress, status, score, notes, and activity unchanged.
- [x] **P9-G4** Existing anime AniList, caching, airing, and recommendation contract tests remain green.

**Evidence:** Complete 2026-08-09. Search, trending, details, tracked-library refresh, and recommendation-candidate queries now share one `MediaKind`-parameterized AniList transport while selecting anime episode fields or manga chapter and volume fields explicitly. Airing remains anime-only, adult content is excluded in every media query, provider `type` is validated against the requested kind, and mapping preserves manga, novel, one-shot, publication status, fuzzy date, weighted-tag, unknown-total, and same-kind continuation semantics while rejecting negative or incompatible fields.

Kind-scoped version-6 discovery, details, and recommendation cache keys prevent anime/manga collisions while retaining the 24-hour details and 6-hour discovery/recommendation TTLs. Metadata refresh batches only the selected kind, performs network work before snapshot transactions, reports per-title failures and stale fallback, and modifies only cache-owned media snapshots. Recommendation profiles and tracked-title exclusion are filtered by kind before deterministic 55/25/20 ranking, including the same AniList ID tracked under the other media kind. Deterministic fixtures and four real Tauri IPC tests cover pagination, cache hits, isolation, expiry, offline stale reads, uncached errors, manga formats and totals, partial GraphQL data, kind mismatch, invalid totals, same-kind relations, `403`, `429 Retry-After`, refresh authority, and recommendation isolation without live network access. The completed gate passes `bun run format:check`, `bun run lint`, `bun run check` with zero diagnostics, `bun run test` (122 tests across 15 files), Cargo formatting, Clippy with warnings denied, and `cargo test --all-features` (92 unit tests plus 24 integration/persistence tests). No dependency or non-Bun lockfile was added.

## P10 - Media-Aware Application Workflows

**Objective:** Deliver the complete manga discovery-to-library and Dashboard experience within the established four-destination shell.

**Depends on:** P8, P9, and the existing P3-P4 application shell and library workflow.

### Entry gate

- [x] **P10-E1** Kind-aware frontend DTOs and command adapters compile in strict TypeScript without `any` or unvalidated compatibility casts.
- [x] **P10-E2** The shared selector and unit-label conventions are defined for keyboard, screen-reader, desktop, narrow, Latte, and Macchiato behavior.

### Milestones

- [x] **P10-M1 - Media-kind selector:** Add one reusable Anime/Manga selector to Home, Discover, and Dashboard. Selection is session-only and defaults to Anime on launch to preserve the 1.0 startup experience; switching kind clears incompatible search pagination, selected details, retry state, and stale announcements.
- [x] **P10-M2 - Manga Discover and details:** Connect manga search and trending collections to the shared cover-card system. Details show precise format, publication status, release date precision, chapters, volumes, genres, tags, freshness, and official continuation relations with explicit unknown states.
- [x] **P10-M3 - Manga library editor:** Present Reading, Completed, and Planning labels while retaining the durable status enum. Provide separate chapter and volume controls, known-total guidance, unknown-total behavior, draft/save feedback, score and notes, and explicit local removal confirmation.
- [x] **P10-M4 - Manga Dashboard:** Scope totals, preference signals, search, status filters, genre/tag filters, sorting, and library cards to manga. Display chapter and volume progress without combining them with anime units, and keep local entries visible when metadata is missing or stale.
- [x] **P10-M5 - Refresh and failure states:** Extend metadata refresh, loading, empty, offline, stale, retryable error, terminal error, and session debugger behavior to the selected kind. Kind switches and retries must not leave an indefinite spinner or show the previous kind's content.
- [x] **P10-M6 - Responsive and accessible presentation:** Verify kind controls, manga cards, unknown totals, editor controls, dialogs, filter overflow, focus restoration, reduced motion, and alternative text at desktop and 390-pixel widths in both themes.
- [x] **P10-M7 - Core end-to-end flow:** Cover manga search, details, add, chapter and volume edits, corrections, Dashboard presentation, metadata refresh, restart persistence, cached offline use, and confirmed removal while an existing anime library remains unchanged.

### Exit gate

- [x] **P10-G1** The complete manga discovery-to-library and Dashboard flow passes through the packaged application boundary using mocked AniList data and isolated storage.
- [x] **P10-G2** Media-kind switching is keyboard operable, resets incompatible view state, and never mixes anime and manga labels, units, cache content, or preference signals.
- [x] **P10-G3** Manga known-total, unknown-total, missing-metadata, loading, empty, offline, stale, validation, and command-error states are covered.
- [x] **P10-G4** Existing anime Home, Discover, details, Dashboard, library, and accessibility tests remain green.

**Evidence (2026-09-11):** The user-requested redesign follows the supplied mockup's compact frosted header, page-local kind selectors, diagonal rainbow, system typography, four-card recommendation shelf, compact trending list, responsive library grid, statistic tiles, and two-column Settings with theme previews. Home and Dashboard share the kind-scoped library workflow. Manga details preserve format and date precision, explicitly show unknown chapter/volume totals, and omit anime duration. Editing stores chapter and volume drafts independently with bounds, correction-safe backend semantics, save feedback, and confirmed removal. Keyed kind changes clear searches, dialogs, filters, pending search replies, and failure announcements; selection stays session-only and defaults to Anime. Home's anime carousel and library refresh each other after edits, and Home reloads on navigation so imported/restored or newly tracked data cannot remain hidden behind an old view.

The component flow covers search, add, independent volume correction, application remount, Dashboard, refresh, cached offline details, confirmed removal, preserved anime, keyboard kind switching, and late-response isolation. A native Tauri IPC flow uses mocked AniList, a temporary SQLite database, real command registration, correction/high-water assertions, database close/reopen, stale offline refresh, rejected unconfirmed removal, and byte-equivalent anime entry preservation. Existing P5/P8/P9 IPC tests now take their origin from the actual mock webview, fixing a Windows-only test URL without changing production permissions. Live browser checks reviewed desktop and 390-pixel layouts in Latte/Macchiato, caught and fixed narrow navigation overflow, and verified a 374-pixel manga dialog within a 390-pixel document and Escape focus restoration. Temporary QA routes were removed; `.tmp/` is ignored by Git, Prettier, and ESLint and is never staged.

**Quality gate:** `bun run format:check`, `bun run lint`, `bun run check` (zero diagnostics), `bun run test` (126 tests across 16 files), `bun run build`, `bun run release:audit`, Cargo formatting, Clippy with all targets/features and warnings denied, and all Rust unit/integration tests pass. `git diff --check` passes. No dependency or lockfile changed.

**Presentation correction (2026-09-17):** Per user feedback, the searchable library now appears exclusively on Home in one horizontally scrolling row of fixed-width cards. Dashboard retains totals, preferences, and activity without duplicating the library. Anime and manga flow tests now verify library edits on Home and activity on Dashboard. Frontend formatting, lint, strict checks, all 126 tests, and the production web build pass. Rust is unchanged; native suites were not rerun for this presentation-only correction.

**Shelf snapping follow-up (2026-09-17):** Library cards snap to their leading edge and scale using the panel width, fitting one to six whole cards across with gaps included. Container queries preserve the fit when the window or available content width changes. Paired keyboard-operable previous and next controls target measured card boundaries, advance by one visible group, clamp at the last complete group, and recover from direct scrolling or resizing. A browser geometry check using the shelf CSS at 11 panel widths (260–1296px) confirmed whole-card fit within 0.063px and snap alignment within 0.5px of the card edge. Frontend formatting, lint, strict checks, all 126 tests, and the production build pass; Rust is unchanged.

**Linux packaged acceptance (2026-09-22):** `bun run release:build` produced `Animi_1.0.1_amd64.deb`; `scripts/acceptance/linux_packaged.py` extracted and launched that package through WebKitGTK 2.52.6 on Ubuntu 26.04 Wayland. Real webview UI and IPC verified manga search/details/add, independent chapter/volume correction, Dashboard totals, metadata refresh, process restart, expired-cache offline fallback with explicit stale/error assertions, confirmed removal, and unchanged anime records. All profiles and fixtures were temporary. Fixture transport requires both the existing smoke guard and isolated data root plus an explicit fixture directory; missing fixtures and the offline marker never fall through to live AniList. The production CSP and capabilities are unchanged. This closes P10-G1 under the approved Linux priority; it does not claim Windows 1.1 installer acceptance.

Frontend formatting, lint, strict checks (zero diagnostics), 126 tests, production build, release audit, Rust formatting, all-target/all-feature Clippy with warnings denied, and the complete Rust test suite pass. The fixture transport has focused fail-closed tests. P11 Continue Reading and manga activity presentation remain the next work.

## P11 - Continue Reading, Activity, and Recommendations

**Objective:** Complete the manga-specific daily-use loop without claiming schedule or time data that the provider does not supply.

**Depends on:** P8-P10 and the existing P5 Home, activity, and recommendation behavior.

### Entry gate

- [ ] **P11-E1** Manga progress activity crosses the real IPC boundary with stable units and timestamps, including chapter-only and chapter-plus-volume saves.
- [ ] **P11-E2** Recommendation fixtures contain sufficient and insufficient same-kind history, already-tracked candidates, missing metadata, stale cache, and offline fallback cases.

### Milestones

- [ ] **P11-M1 - Continue Reading data:** Implement `get_home_reading` from locally authoritative `WATCHING` manga, enriched by optional cached metadata and ordered by most recently updated local entry. Do not infer unread availability, chapter release dates, or publication cadence.
- [ ] **P11-M2 - Chapter quick progress:** Add an accessible `+1 chapter` action that records progress and activity transactionally, stops at a known chapter total, remains usable for unknown totals, and never changes volume progress implicitly.
- [ ] **P11-M3 - Reading Home presentation:** Add distinct active, completed-at-known-total, unknown-total, missing-metadata, empty, stale, and error presentations. Preserve the anime airing carousel and prevent its 30-minute refresh loop from treating manga as airing media.
- [ ] **P11-M4 - Reading activity:** Generalize Dashboard activity presentation so manga reports chapter and volume deltas in monthly and yearly buckets while anime retains episode counts and watch minutes. Never sum or convert chapters, volumes, episodes, or minutes into one value.
- [ ] **P11-M5 - Manga recommendations:** Reuse the deterministic 55/25/20 scoring model with only Reading and Completed manga as profile signals, exclude tracked manga, explain the strongest same-kind tags, and fall back to manga trending when history is insufficient.
- [ ] **P11-M6 - Daily-loop coverage:** Test chapter and volume activity high-water semantics, quick progress at known and unknown totals, restart persistence, Continue Reading ordering, missing metadata, recommendation isolation, cached offline fallback, and both media kinds in one profile.

### Exit gate

- [ ] **P11-G1** A user can resume a manga from Home, advance one chapter safely, and see the durable reading activity after restart.
- [ ] **P11-G2** Manga activity reports only chapters and volumes; anime activity retains its existing episode and watch-time behavior.
- [ ] **P11-G3** Recommendations never mix profile signals or candidates across media kinds and remain deterministic under offline stale fallback.
- [ ] **P11-G4** Home and recommendation flows pass at desktop and narrow widths with keyboard, focus, screen-reader status, and reduced-motion coverage.

## P12 - Version 1.1 Release Hardening

**Objective:** Prove that Animi 1.1 is a safe private upgrade for fresh and existing Linux profiles.

**Depends on:** P8-P11 and every cumulative P0-P7 release gate.

### Entry gate

- [ ] **P12-E1** Every P8-P11 milestone and exit gate is complete with recorded evidence and no unresolved data-loss, privacy, migration, backup, or accessibility blocker.
- [ ] **P12-E2** Release configuration is ready for version `1.1.0` without enabling public publishing, an updater, or expanded capabilities.

### Milestones

- [ ] **P12-M1 - Migration and backup acceptance:** Upgrade isolated copies of every immutable database fixture and a representative 1.0.1 profile; verify anime preservation, new manga writes, schema 1 import, schema 2 round trip, failure rollback, reinstall retention, and downgrade protection.
- [ ] **P12-M2 - Regression matrix:** Exercise Home, Discover, details, Dashboard, Settings, both media kinds, online and offline states, fresh and stale caches, both themes, desktop and narrow widths, keyboard use, focus, alternative text, contrast, and reduced motion.
- [ ] **P12-M3 - Full automated gate:** Run frontend formatting, lint, strict checks, tests, production build, release audit, Rust formatting, Clippy with warnings denied, all Rust tests, migration fixtures, and critical-flow coverage from a clean worktree.
- [ ] **P12-M4 - Release documentation:** Update the README, release notes, release audit, known limitations, backup compatibility, data location, private-distribution restriction, dependency notices, and any user-facing migration or recovery guidance.
- [ ] **P12-M5 - Linux package acceptance:** Build the private Debian package on Ubuntu 26.04 Wayland, perform fresh install and same-identity upgrade using an isolated 1.0.1 database profile, launch online and offline against isolated profiles, reinstall, uninstall, and verify the documented retained-data behavior.
- [ ] **P12-M6 - Artifact audit:** Confirm the installer and repository contain no secrets, real user data, bundled databases, backups, caches, updater, analytics, public publishing path, non-Bun lockfile, accidental capability expansion, or undocumented dependency.

### Exit gate

- [ ] **P12-G1** Anime and manga critical flows pass from the packaged Linux application, including offline use of previously cached tracked titles.
- [ ] **P12-G2** A production Linux package upgrades an isolated Animi 1.0.1 database profile without losing or changing anime library, settings, watch activity, event IDs, or backup compatibility.
- [ ] **P12-G3** Fresh and upgraded profiles can create, back up, clear in isolation, and restore mixed anime/manga data transactionally.
- [ ] **P12-G4** The complete clean-tree quality gate passes and the release audit records reproducible evidence.
- [ ] **P12-G5** Version 1.1 remains private, local-only, adult-content-filtered, and within every boundary in `PROJECT.md`.

## Quality Gates

Run focused tests throughout a phase. Run the complete gate before release acceptance and after changes to shared contracts, migrations, packaging, or release configuration:

```powershell
bun run format:check
bun run lint
bun run check
bun run test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

Documentation-only changes may skip runtime suites when the handoff states that explicitly. Always inspect the diff and run `git diff --check` before handoff.

## Risks and Gate Controls

| Risk | Control | Release gate |
|---|---|---|
| Missing Windows native prerequisites | Verify them before scaffolding and again on the packaging machine | P0, P7, P12 |
| AniList outage, malformed data, or throttling | Typed adapters, bounded kind-scoped caching, stale fallback, `Retry-After`, and deterministic mocks | P2, P7, P9, P12 |
| Existing 1.0.x data or backups are damaged by manga migrations | Append-only migrations, immutable upgrade fixtures, schema 1 import coverage, stable event IDs, transactional rollback, and packaged upgrade QA | P8, P12 |
| Anime and manga fields, caches, progress units, or preferences leak across kinds | Carry `MediaKind` through keys and DTOs, validate cross-kind fields, and run mixed-profile isolation tests | P8-P12 |
| Chapter and volume progress becomes inconsistent | Treat them as independent user assertions with separate bounds and high-water marks; never infer one from the other | P8, P10, P11 |
| Manga UI implies unavailable schedule or reading-time data | Use local recency for Continue Reading and report chapters or volumes only; explicitly label unknown totals | P9-P12 |
| Recommendation behavior becomes opaque or unstable | Pure deterministic same-kind scoring, fixed fixtures, and user-visible explanation labels | P5, P9, P11 |
| Responsive or accessible behavior regresses | Desktop/narrow tests, keyboard checks, both themes, screen-reader status, and reduced-motion review | P3, P7, P10-P12 |
| Mobile, sync, reader, OAuth, or notification scope enters version 1.1 | Keep future work non-actionable until `PROJECT.md` is approved and revised | P8-P12 |
| Public distribution conflicts with AniList terms | Keep version 1.1 private unless permission or a compatible product change is documented | P12 |

## Future Roadmap - Not Approved for Version 1.1

Future phases have no schedule, implementation status, or permission to begin. Promote one into the active roadmap only after its product and architecture decisions are added to `PROJECT.md`.

### F2 - Tauri mobile clients

- Reuse platform-neutral DTOs, Rust domain behavior where supported, and responsive Svelte layouts.
- Design mobile navigation, storage locations, permissions, backup file exchange, and packaging explicitly.
- Begin with manual versioned backup transfer; do not imply live synchronization.
- Re-run accessibility, narrow-layout, migration, backup, and offline gates on each supported platform.

### F3 - Optional synchronization and background features

- Require a separately approved privacy, authentication, encryption, conflict-resolution, deletion, and recovery architecture.
- Decide whether synchronization uses a user-operated service, a managed service, or an external account provider before implementation.
- Treat background refresh and notifications as platform-specific capabilities with explicit permission and failure behavior.
- Revisit AniList OAuth, API terms, public distribution, data minimization, and account-deletion requirements.
- Preserve a fully local mode and provide a safe migration path for existing v1 users.

## Decision Log

| Date | Decision | Reason |
|---|---|---|
| 2026-07-16 | `PROJECT.md` owns product and architecture; `PLAN.md` owns delivery and status | Separates durable decisions from a frequently updated implementation roadmap |
| 2026-07-16 | Plan by dependencies without dates or effort estimates | Avoids speculative commitments before the application is scaffolded |
| 2026-07-16 | Track stable milestone IDs, phase gates, and evidence | Makes progress reviewable and prevents partial work from appearing complete |
| 2026-07-16 | Detail v1 and keep later platforms as an unapproved outline | Preserves future direction without expanding the current product scope |
| 2026-07-16 | Use Home, Discover, Dashboard, and Settings as the four primary destinations; keep details secondary and recommendations within Discover | Matches the approved information architecture without introducing accounts or additional backend ownership |
| 2026-07-16 | Treat Dashboard as a local personal overview rather than an account profile | Preserves Animi's private, account-free v1 boundary |
| 2026-07-16 | Record watch activity durably instead of inferring it from library timestamps | Accurate monthly and yearly viewing history requires progress events and duration data that the initial schema does not contain |
| 2026-07-16 | Deliver the P3 frontend rewrite through user-reviewed visual checkpoints | Allows desktop and narrow-layout approval before proceeding to the next page |
| 2026-07-17 | Treat initial progress as a baseline and retain an activity high-water mark across corrections | Prevents fabricated history and avoids counting corrected then restored progress twice without mutating append-only events |
| 2026-07-17 | Establish anime-only Dashboard base insights during P4 and add time series in P5 | Lets the P4 Dashboard render authoritative totals, preference coverage, and raw activity while keeping calendar bucketing in its planned phase |
| 2026-07-17 | Record explicit nonzero progress on a title's initial save and repair skipped initial progress in existing profiles | User testing showed that treating a first save as an uncounted baseline left completed titles and their watch time out of Dashboard history |
| 2026-07-17 | Preserve activity high-water marks and stable event IDs in backup schema v1 | Recovery must not count corrected progress twice, and repeated imports need portable watch-event identity |
| 2026-07-17 | Bound Home quick progress by known episode totals independently of airing schedules | Users may record watched episodes even when AniList has no future schedule; release state and viewing progress are separate concerns |
| 2026-07-17 | Keep version 1 private and package only current-user NSIS with no updater | AniList permission remains unresolved, while a stable identity, downgrade protection, and documented retained app data make private upgrades recoverable |
| 2026-07-17 | Guard the packaged release-smoke data root behind two explicit environment values | Windows known-folder resolution ignores a simple `APPDATA` override; the paired guard enables isolated installer QA without changing normal profile resolution |
| 2026-07-19 | Keep forced presentation debugging in session-only frontend state | The control must exercise shared UI states without changing the authoritative SQLite settings, backups, or normal startup behavior |
| 2026-08-02 | Promote future manga work into version 1.1 phases P8-P12 | Manga is now approved release scope and needs a dependency-ordered path from compatibility contracts through Windows packaging |
| 2026-08-02 | Keep `LibraryStatus.WATCHING` durable and label it Reading for manga | Contextual presentation adds manga language without a risky status-enum migration or loss of backup compatibility |
| 2026-08-02 | Treat manga primary progress as chapters and store volumes independently | AniList exposes both totals, but neither reliably determines the other; separate validation and high-water marks preserve user intent |
| 2026-08-02 | Use local recency for Continue Reading and omit manga schedules or time estimates | AniList has no airing-equivalent chapter feed or reliable reading duration, so inferred catch-up and time claims would be misleading |
| 2026-08-02 | Export backup schema 2 and continue importing schema 1 | Manga requires new progress and activity fields, while every Animi 1.0.x backup must remain recoverable without a lossy downgrade format |
| 2026-08-02 | Isolate discovery, caches, activity presentation, preferences, and recommendations by media kind | Episodes, chapters, volumes, and profile signals have different meanings and must never be combined implicitly |

### Linux priority (2026-09-22)

The user approved Linux-first delivery, especially this Ubuntu 26.04 x86_64 Wayland machine. P10 packaged acceptance and P12 packaging now target native Linux WebKitGTK and a Debian package. Windows configuration and historical P7 acceptance remain intact; Windows 1.1 installer execution is deferred to a Windows host and is not claimed by Linux evidence. The package gate still requires real webview IPC, isolated profiles, deterministic mocked metadata, offline recovery, retained user data, and upgrade/downgrade checks. No existing unchecked gate is completed by this scope update.
