# Animi 1.1.0

Private, local-first anime and manga tracking, with Linux as the primary desktop platform.

## What's included

- Anime and manga discovery, details, library editing, and separate Dashboard insights. Manga includes novels and one-shots with explicit format labels.
- Continue Reading on Home, ordered by local activity, with a keyboard-accessible one-chapter action. Unknown chapter totals remain usable; known totals stop quick progress.
- Independent chapters and volumes. Corrections never count previously recorded progress twice. Manga monthly and yearly charts keep chapter and volume totals separate; anime keeps episode and watch-time reporting.
- Same-kind local recommendations, including deterministic cached offline fallback.
- A native Debian package, desktop launcher and icon, system fonts, GTK file dialogs, XDG storage, and Latte/Macchiato themes. Primary acceptance targets Ubuntu 26.04 x86_64 on Wayland with WebKitGTK 2.52.6.

## Upgrade and recovery

The stable application identity remains `app.animi.desktop`. Launching 1.1 migrates the local database forward to schema 8. Existing anime entries, settings, activity identities, and correction high-water marks are preserved. Keep a backup before moving a profile between machines or upgrading.

Exports now use backup schema 2. Imports accept schemas 1 and 2; there is no schema-1 export downgrade. Schema-1 manga starts with zero volume progress. Backups include user-owned settings, entries, activity, and high-water marks; cached metadata and images are excluded. Imported entries may need metadata fetched again while online.

Linux stores data in `$XDG_DATA_HOME/app.animi.desktop`, normally `~/.local/share/app.animi.desktop`. Windows keeps its existing per-user app-data directory. Transfer a Windows profile to Linux through a backup, rather than expecting automatic cross-platform discovery or synchronization.

Removing the Linux package retains the user profile. Reinstalling reuses it. Settings → Clear local data requires `CLEAR MY LOCAL DATA`; export to a location outside the live storage directory first. Import previews validate the complete file before transactional application and preserve unrelated entries.

Linux package managers can install an older binary, but the application's migration ledger rejects an unsupported newer database before opening it for normal use. Do not force a downgrade or edit the migration ledger. Recover using the current application and a compatible backup.

## Known limits

- This is a private release. Public redistribution remains outside the approved AniList terms scope.
- Windows configuration remains supported, but Windows 1.1 NSIS execution is deferred to a Windows host. Historical 1.0.1 acceptance remains recorded separately.
- Linux acceptance is specific to Ubuntu 26.04 x86_64 and its system libraries. This build does not claim compatibility with older distributions or provide an AppImage.
- On the acceptance machine, WebKitGTK 2.52.6 intermittently reports a helper-process abort during Mesa 26.0.8 shutdown cleanup. Active workflows and data-preservation checks passed; the shutdown warning remains a known system-runtime limitation.
- No reader, chapter release predictions, reading-time estimates, OAuth, cloud sync, analytics, updater, background service, mobile client, or closed-app notifications.
- Offline use requires cached metadata. Missing metadata remains explicit; local progress and backups remain available.

See [release audit](RELEASE_AUDIT.md) for verification commands and the distinction between packaged UI tests, isolated package-manager tests, and native unit/integration coverage.
