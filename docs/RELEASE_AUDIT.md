# Version 1 release audit

This record maps P7 verification to repeatable evidence. It does not authorize public distribution.

## Regression and failure matrix

| Profile  | Theme               | Connectivity                    | Width                                    | Evidence                                                                                                                                              |
| -------- | ------------------- | ------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fresh    | Latte and Macchiato | Online-shaped fixtures          | 1280 × 800 and 390 × 844                 | Frontend component suites cover all destinations, primary flows, responsive layouts, theme persistence, keyboard controls, and terminal async states. |
| Fresh    | Latte and Macchiato | Offline/no cache                | 1280 × 800 and 390 × 844                 | Discover, Home, Dashboard, details, and shared async-state tests cover actionable offline/error states without indefinite spinners.                   |
| Existing | Preserved setting   | Cached offline fallback         | Backend fixtures v0001, v0002, and v0003 | Migration and cache tests upgrade to schema 7, preserve user-owned state, and expose stale refresh failures.                                          |
| Existing | Preserved setting   | Recovery interrupted or invalid | Isolated temporary storage               | Backup tests cover malformed/unsupported input, stale preview tokens, forced mid-apply failure, rollback, and idempotent replay.                      |

Critical flow coverage includes Home airing and bounded quick progress, Discover search/trending/recommendations, details-to-library editing, Dashboard filtering/activity/metadata refresh, theme settings, export, preview, import, clear confirmation, restart persistence, and removal confirmation.

## Accessibility

- Primary destinations use an ARIA tab list with roving Left/Right/Home/End operation; every critical action is a native button, link, input, radio, or select-equivalent control with an accessible name.
- The shell provides a skip link, one page heading, logical DOM focus order, modal focus containment/restoration, and visible `:focus-visible` outlines.
- Cover images have title-specific alternative text; missing covers expose an equivalent labeled fallback. Decorative cat marks are hidden from assistive technology.
- Loading regions expose `aria-busy`; errors use alerts; offline, stale, success, export, restore, and clear results use status announcements or polite live regions.
- Both Catppuccin palettes use semantic tokens. Release QA checks normal/muted/status text contrast against their intended opaque fallback surfaces.
- A global reduced-motion query collapses transitions and animations and disables smooth scrolling.

## Privacy and scope

- There is no application server, HTTP listener, account, analytics SDK, telemetry, OAuth, cloud storage, sync engine, background service, updater, or notification plugin.
- The Svelte frontend uses typed Tauri IPC and contains no direct network or durable browser-storage boundary. Rust owns SQLite, files, and the sole external endpoint: `https://graphql.anilist.co`.
- AniList requests are user/on-launch driven and bounded; library metadata refresh batches only tracked IDs. The application does not bulk-collect AniList data or use it as backup storage.
- Tests inject temporary database roots and disabled/scripted transports. Committed database files are synthetic immutable migration fixtures; no packaged cache, database, backup, secret, credential, or environment file is permitted.
- `package.json` is private and unlicensed, Cargo publishing is disabled, updater artifacts are disabled, and the repository license forbids public redistribution. AniList permission remains unresolved, so the installer is private only.

## Dependencies and production configuration

- Direct JavaScript dependencies use MIT or Apache-2.0-compatible licenses. Direct Rust dependencies use MIT, Apache-2.0, or dual MIT/Apache-2.0 terms. The application remains proprietary; dependency licenses are not replaced by the application license.
- `bun.lock` and `src-tauri/Cargo.lock` are the only root dependency lockfiles. Bun remains the exclusive frontend package manager.
- Application identity is stable at `app.animi.desktop`; product/binary name is Animi; package, Cargo, and Tauri versions are aligned at 1.0.0.
- The main window capability grants only `core:default`. Backup dialogs are invoked by Rust through the registered dialog plugin; no frontend filesystem permission is exposed.
- Production CSP denies objects and frames, limits scripts/fonts/styles to packaged content, permits IPC only for connections, and permits HTTPS only for cover images.
- The installer is current-user NSIS, blocks downgrades, embeds no updater, and skips bundling WebView2 because WebView2 is an explicit prerequisite.
- The source-controlled icon is an Animi-specific cat mark with restrained rainbow whiskers, generated into every Windows icon size and visually checked at 32 px and 512 px.

## Windows package expectations

- Build with `bun run release:audit` followed by `bun run release:build`.
- Smoke installation uses a temporary current-user directory and an isolated temporary `APPDATA` root so validation cannot open or change a real Animi profile.
- Launch must remain running without a startup panic. A second launch against the same isolated app-data root must preserve the database and settings paths.
- Installing the same stable identity over an earlier build preserves `%APPDATA%\app.animi.desktop`; migrations run on first launch. Downgrades are refused.
- NSIS uninstall removes installed program files and shortcuts. User-created application data under `%APPDATA%\app.animi.desktop` is intentionally retained for recovery or reinstallation and can be removed only through Animi's confirmed clear-data flow or an explicit manual action.

Final command counts, artifact hashes, installer/install/launch/uninstall results, and any exceptions are recorded in `PLAN.md` after the gate runs.
