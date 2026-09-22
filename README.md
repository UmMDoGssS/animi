# Animi

Animi is a private, local-first Linux-first anime and manga tracker and recommendation application. The Tauri 2 backend owns durable state and external metadata access; the Svelte 5 frontend communicates with it only through typed commands.

Product and architecture decisions live in [`PROJECT.md`](PROJECT.md). Delivery status and milestone evidence live in [`PLAN.md`](PLAN.md).

## Prerequisites

- Bun 1.3.14 or newer compatible release
- Node.js 24 or newer compatible release
- Rust 1.96 or newer compatible release
- Linux: a C compiler, pkg-config, GTK 3, WebKitGTK 4.1, librsvg, and patchelf (Ubuntu 26.04 x86_64 is the primary target)
- Windows: Microsoft C++ Build Tools, the `x86_64-pc-windows-msvc` target, and Microsoft Edge WebView2 Runtime

## Development

```powershell
bun install
bun run tauri dev
```

The frontend alone can be served with `bun run dev`. That mode cannot complete Tauri IPC calls and is intended only for isolated UI work.

## Private desktop release

Public distribution is disabled until the AniList permission prerequisite in `PROJECT.md` is resolved. For an authorized private build:

```powershell
bun run release:audit
bun run release:build
```

On Linux this produces a Debian package in `src-tauri/target/release/bundle/deb`. Install it with your distribution's package manager. The app uses the system WebKitGTK runtime and native desktop file dialogs. Its profile is stored under `$XDG_DATA_HOME/app.animi.desktop` (normally `~/.local/share/app.animi.desktop`). Package removal retains this profile; use Settings → Clear local data for deliberate deletion.

On Windows this produces a current-user NSIS installer. WebView2 must already be installed. Animi's durable profile lives in the Windows per-user app-data directory for the stable `app.animi.desktop` identity; upgrades preserve that directory, and uninstall intentionally leaves it available for recovery. Use Settings → Clear local data when user-owned data should be deleted deliberately.

See [`docs/RELEASE_NOTES_1.0.0.md`](docs/RELEASE_NOTES_1.0.0.md) and [`docs/RELEASE_AUDIT.md`](docs/RELEASE_AUDIT.md) for compatibility, limitations, and P7 verification scope.

## Quality gates

```powershell
bun run format:check
bun run lint
bun run check
bun run test
bun run release:audit
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

Frontend dependencies and scripts use Bun exclusively. Automated backend tests inject temporary storage roots and a network-disabled AniList transport, so they never touch a user database or the live AniList API.

## Source layout

- `src/lib/components`: focused Svelte components
- `src/lib/ipc`: typed frontend command adapters and DTOs
- `src-tauri/src/commands`: thin Tauri command adapters
- `src-tauri/src/domain`: serializable domain contracts and errors
- `src-tauri/src/database`: injected database-path boundary
- `src-tauri/src/anilist`: replaceable HTTP transport boundary
- `src-tauri/src/backups`: backup format ownership
- `src-tauri/src/recommendations`: deterministic recommendation configuration
- `src-tauri/tests/fixtures`: deterministic backend fixtures

## Packaged Linux acceptance

With a desktop session and Ubuntu's `webkitgtk-webdriver` package (called `webkit2gtk-driver` on older releases), run:

```sh
python3 scripts/acceptance/linux_packaged.py src-tauri/target/release/bundle/deb/Animi_1.0.1_amd64.deb /usr/bin/WebKitWebDriver
```

The script extracts the package, creates temporary storage and deterministic metadata, exercises the real WebKitGTK UI/IPC, restarts, and verifies offline data. It never opens the normal profile. Fixture mode requires `ANIMI_RELEASE_SMOKE=1`, nonempty `ANIMI_RELEASE_SMOKE_DATA_DIR`, and `ANIMI_RELEASE_SMOKE_FIXTURES`; it cannot fall back to live network access. These values are for isolated QA only.
