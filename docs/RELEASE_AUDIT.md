# Animi release audits

## Version 1.1 Linux acceptance (2026-09-23)

Version 1.1 targets Ubuntu 26.04 x86_64 on this Wayland desktop with GTK 3.24.52 and WebKitGTK 2.52.6. The stable identity remains `app.animi.desktop`; versions are aligned at 1.1.0. Windows 1.1 installer execution is deferred to a Windows host; the historical P7 record below is unchanged.

- `scripts/acceptance/linux_packaged.py` runs the Debian-installed executable through the real WebKitGTK webview. It covers anime search/details/add/Home quick progress/watch time; manga search/details/add, independent chapter/volume correction, Continue Reading, Dashboard, stale offline caches, restart, and confirmed removal. Sixteen screenshots cover Home, Discover, Dashboard, and Settings at 1180 and 390 pixels in Latte and Macchiato with document-overflow assertions. Focus, keyboard actions, status announcements, contrast, and reduced motion also have component coverage.
- Fresh and upgraded package runs use temporary database, XDG config/cache/data, and fixture directories. Explicit smoke mode replaces only transport and backup picker destinations; shipped commands, migrations, database transactions, frontend assets, and IPC registration remain in use. Backup destinations are outside live storage, schema-2 exports round-trip settings/entries/activity exactly, schema-1 imports remain accepted, and invalid previews preserve local records. Native file-dialog clicks are not automated by WebKitWebDriver; normal launches retain Rust-owned GTK dialogs.
- A Debian package rebuilt from the actual `v1.0.1` tag creates a representative old profile. Installing 1.1 over it preserves the anime entry, settings, and activity event IDs. An isolated `fakeroot`/`dpkg` root verifies install, same-identity upgrade, removal, and reinstall. The root intentionally omits dependency registration and uses host GTK/WebKit libraries; this is not a real-system package installation. Removing the package retains the profile byte-for-byte. Reinstall restores access. The old executable rejects the migrated database before normal startup and leaves it unchanged.
- `scripts/acceptance/linux_xdg.py` verifies a normal, non-smoke startup resolves `$XDG_DATA_HOME/app.animi.desktop/animi.sqlite3` in a temporary XDG environment.
- All four immutable database fixtures (schemas 1, 2, 3, and 7) upgrade and reopen through native integration tests. Existing P8 and backup suites cover mixed profiles, schema compatibility, independent high-water marks, foreign keys, import conflicts, stale previews, and forced transaction rollback. A new release test rejects a future migration without changing user records.
- `scripts/acceptance/audit_deb.py` allows only the executable, desktop entry, icons, application license, and third-party notices. There are no maintainer scripts, bundled user databases, caches, backups, environment files, or extra executables. `bun run release:audit` verifies private publishing policy, versions, identity, CSP, unchanged capabilities, Linux Debian/Windows NSIS policy, approved API boundary, lockfiles, and repository artifact hygiene.

Reproduce the Linux checks after `bun run release:build`:

```sh
python3 scripts/acceptance/audit_deb.py src-tauri/target/release/bundle/deb/Animi_1.1.0_amd64.deb
python3 scripts/acceptance/linux_xdg.py src-tauri/target/release/animi /usr/bin/WebKitWebDriver
python3 scripts/acceptance/linux_packaged.py src-tauri/target/release/bundle/deb/Animi_1.1.0_amd64.deb /usr/bin/WebKitWebDriver
ANIMI_PREVIOUS_PACKAGE=/absolute/path/to/Animi_1.0.1_amd64.deb python3 scripts/acceptance/linux_packaged.py src-tauri/target/release/bundle/deb/Animi_1.1.0_amd64.deb /usr/bin/WebKitWebDriver
```

`ANIMI_PREVIOUS_PACKAGE` must identify a private Linux build from the actual old tag, not an intermediate manga-development build still numbered 1.0.1. Backup fixture mode does not contact AniList. The old release, which predates fixture transport, uses a deliberately unavailable proxy for non-loopback requests. The driver and application communicate locally. Screenshots are written outside the repository to `/tmp/animi-acceptance-evidence` by default.

An intermittent WebKit helper shutdown abort was observed on this host. System core backtraces identify Mesa 26.0.8 Gallium/GBM cleanup through `__run_exit_handlers` in WebKitWebProcess (WebKitGTK 2.52.6). The active flows, persistence checks, and final harness exits passed; this runtime shutdown limitation is not claimed fixed. The harness separately stops its isolated driver process group before deleting temporary XDG storage to avoid late helper writes racing cleanup.

The final clean-tree command results and package hash are recorded in `PLAN.md`. No runtime dependency was added for P10 acceptance or P11/P12 completion.

## Historical version 1 Windows release audit

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
- The source-controlled icon is an Animi-specific neutral cat mark framed by a circular rainbow, generated into every Windows icon size and visually checked at 32 px and 512 px.

## Windows package expectations

- Build with `bun run release:audit` followed by `bun run release:build`.
- Smoke installation uses a temporary current-user directory and the guarded `ANIMI_RELEASE_SMOKE=1` plus `ANIMI_RELEASE_SMOKE_DATA_DIR` startup override. Supplying the directory without the exact guard has no effect, while the paired values let packaged QA avoid opening or changing the normal Animi profile.
- Launch must remain running without a startup panic. A second launch against the same isolated app-data root must preserve the database and settings paths.
- Installing the same stable identity over an earlier build preserves `%APPDATA%\app.animi.desktop`; migrations run on first launch. Downgrades are refused.
- NSIS uninstall removes installed program files and shortcuts. User-created application data under `%APPDATA%\app.animi.desktop` is intentionally retained for recovery or reinstallation and can be removed only through Animi's confirmed clear-data flow or an explicit manual action.

Final command counts, artifact hashes, installer/install/launch/uninstall results, and any exceptions are recorded in `PLAN.md` after the gate runs.
