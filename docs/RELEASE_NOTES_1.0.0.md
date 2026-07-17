# Animi 1.0.0

Animi 1.0.0 is the first private Windows release of the local-first anime tracker. Public distribution remains disabled: AniList permission or an approved product change is still required before this installer may be shared publicly.

## Delivered

- A four-destination desktop experience for Home, Discover, Dashboard, and Settings with Catppuccin Latte and Macchiato themes, responsive layouts, keyboard operation, visible focus, and reduced-motion support.
- A local SQLite library for watching, completed, and planning anime, including safe progress correction, durable watch activity, personal preference insights, and monthly/yearly activity.
- On-demand AniList search, details, airing schedules, trending titles, and explainable local recommendations with bounded caches, rate-limit handling, offline fallback, and truthful stale indicators.
- Versioned JSON backup export, fully validated preview, transactional restore, idempotent activity merging, and exact-phrase local-data deletion.
- A current-user Windows NSIS installer with a stable `app.animi.desktop` identity, downgrade protection, a production content-security policy, and no updater or public publishing channel.

## Backup and migration compatibility

- Backup schema version 1 is the only supported import and export format in 1.0.0.
- Database migrations are append-only. Profiles created by every earlier repository schema fixture are migrated automatically to schema 7 while preserving settings, library entries, cached tracked metadata, progress high-water marks, and watch activity.
- Application data remains under the Windows per-user app-data directory associated with `app.animi.desktop`. Installing a newer version must not remove it; uninstall behavior is documented in the release audit.
- Import validates the complete backup before writing. Unrelated local library entries remain in place, and a failed or stale recovery applies no partial changes.

## Known limitations

- This is a private, unsigned build. Windows may display an unrecognized-publisher warning.
- WebView2 is a prerequisite and is not embedded in the installer. Supported Windows installations normally provide it; the README lists it explicitly for offline installation planning.
- Anime metadata requires AniList. Cached tracked titles remain usable offline, but uncached search, details, airing, and recommendation candidates cannot be fetched without a connection.
- Animi has no AniList account connection, manga UI, phone client, synchronization, analytics, cloud storage, background service, or closed-app notifications.
- Closed-app airing refresh and notifications are intentionally outside version 1.

## Dependency additions

No runtime or frontend dependency was added during release hardening. The existing direct dependencies were retained because each owns a distinct boundary: Tauri desktop/IPC/dialogs, Svelte/SvelteKit UI, Tailwind styling, SQLx persistence, Reqwest HTTP, Tokio async work, Serde JSON contracts, Chrono timestamps, UUID activity identity, and typed error handling.
