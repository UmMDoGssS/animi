# Animi Delivery Plan

## Plan Status

| Field | Value |
|---|---|
| Product | Animi |
| Release | Version 1, Windows desktop |
| Overall status | Version 1 complete; private distribution only |
| Current phase | P7 complete |
| Schedule | Dependency-driven; no calendar commitment |
| Last updated | 2026-07-17 |

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
- Items under Future Roadmap are context, not approved v1 work.

## Version 1 Boundaries

Version 1 is a private, local-first, anime-focused Windows desktop application. The local SQLite database is authoritative. Rust owns durable state, AniList access, caching, backups, and imports; Svelte communicates with Rust only through typed Tauri commands.

The following remain outside v1:

- Manga workflows, even though shared types and storage leave room for them.
- Mobile clients and live cross-device synchronization.
- A server, cloud database, accounts, analytics, or AniList OAuth.
- Closed-app refresh or notifications.
- Predicted or inferred sequel announcements.
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

## Planned Contracts

These are implementation targets, not interfaces already present in the repository. The detailed shapes must remain compatible with the decisions in `PROJECT.md`.

### Shared domain types

- `MediaKind`: `ANIME | MANGA`; v1 commands and views expose only anime workflows.
- `LibraryStatus`: `WATCHING | COMPLETED | PLANNING`.
- `LibraryEntry`: AniList ID, media kind, status, progress, timestamps, optional score, and optional notes.
- `MediaSnapshot`: titles, cover URL, format, genres, weighted tags, status, episode or chapter total, and cache timestamp.
- `AiringInfo`: episode number, airing timestamp, and fetch timestamp.
- `WatchActivity`: AniList ID, media kind, progress delta, best-known episode duration, and UTC occurrence time.
- `Recommendation`: media summary, local score, and concise explanation labels.
- `CommandError`: `code`, `message`, and `retryable` at the IPC boundary.

All JSON fields crossing the Tauri boundary use camelCase. Rust may use snake_case internally with explicit Serde renaming. Timestamps cross the boundary as UTC values in one documented representation.

### Tauri commands

| Area | Commands | Planned phase |
|---|---|---|
| Discovery | `search_media`, `get_media_details`, `list_trending` | P2, surfaced in P4 |
| Library | `list_library`, `upsert_library_entry`, `advance_library_progress`, `remove_library_entry` | P1, extended and surfaced in P4-P5 |
| Metadata | `refresh_library_metadata` | P2, surfaced in P4-P5 |
| Recommendations | `get_recommendations` | P5 |
| Dashboard | `get_dashboard_insights` | P4 base aggregates/events; P5 monthly/yearly series |
| Backup | `export_backup`, `preview_import`, `import_backup` | P6 |

Commands remain thin adapters. Validation, database behavior, AniList behavior, recommendation logic, and backup logic live in testable Rust modules.

For P4, `list_library` returns authoritative local entry fields enriched with optional cached media presentation fields and freshness needed by Dashboard and library cards. Missing cached metadata must not hide a local entry, and metadata refreshes never overwrite local status, progress, score, or notes.

### Backup envelope

Backups use versioned JSON containing `schemaVersion`, `exportedAt`, settings, local library entries, and user-owned watch activity. Backup-only library records preserve the activity high-water mark, and watch activity carries stable event IDs so repeated imports can be idempotent. An existing event ID with an identical payload is a no-op; a reused event ID with a different payload is a validation error before writes. Cache records and downloaded images are excluded. Import validates the complete file before opening one write transaction. Library and setting conflicts use the greatest `updatedAt`, with equal timestamps keeping local data. Same-`createdAt` library records retain the greatest activity high-water mark; different `createdAt` values are separate incarnations and the winning incarnation retains its own high-water mark. Import never deletes unrelated local entries.

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

## Quality Gates

Run focused tests throughout a phase. Run the complete gate before v1 acceptance and after changes to shared contracts, migrations, packaging, or release configuration:

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
| Missing Windows native prerequisites | Verify them before scaffolding and again on the packaging machine | P0, P7 |
| AniList outage, malformed data, or throttling | Typed adapters, bounded caching, stale fallback, `Retry-After`, and deterministic mocks | P2, P7 |
| Local data loss | Transactions, foreign keys, append-only migrations, explicit confirmation, backups, and rollback tests | P1, P6, P7 |
| Recommendation behavior becomes opaque or unstable | Pure deterministic scoring, fixed fixtures, and user-visible explanation labels | P5 |
| Responsive or accessible behavior regresses | Desktop/narrow tests, keyboard checks, both themes, and reduced-motion review | P3, P7 |
| Manga, mobile, sync, or notification scope enters v1 | Keep future work non-actionable until `PROJECT.md` is approved and revised | All phases |
| Public distribution conflicts with AniList terms | Keep v1 private unless permission or a compatible product change is documented | P7 |

## Future Roadmap - Not Approved for Version 1

Future phases have no schedule, implementation status, or permission to begin. Promote one into the active roadmap only after its product and architecture decisions are added to `PROJECT.md`.

### F1 - Manga workflows

- Expose manga in search, details, library filtering, and recommendations using the existing `MediaKind` boundary.
- Add chapter and volume progress semantics without weakening anime progress validation.
- Extend backups and append-only migrations while preserving all v1 data.
- Reassess manga-specific metadata, airing-equivalent UX, and test fixtures before implementation.

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
