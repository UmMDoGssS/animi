# Animi

Animi is a private, local-first Windows anime tracker and recommendation application. The Tauri 2 backend owns durable state and external metadata access; the Svelte 5 frontend communicates with it only through typed commands.

Product and architecture decisions live in [`PROJECT.md`](PROJECT.md). Delivery status and milestone evidence live in [`PLAN.md`](PLAN.md).

## Prerequisites

- Bun 1.3.14 or newer compatible release
- Node.js 24 or newer compatible release
- Rust 1.96 or newer compatible release with the `x86_64-pc-windows-msvc` target
- Microsoft C++ Build Tools
- Microsoft Edge WebView2 Runtime

## Development

```powershell
bun install
bun run tauri dev
```

The frontend alone can be served with `bun run dev`. That mode cannot complete Tauri IPC calls and is intended only for isolated UI work.

## Private Windows release

Public distribution is disabled until the AniList permission prerequisite in `PROJECT.md` is resolved. For an authorized private build:

```powershell
bun run release:audit
bun run release:build
```

This produces a current-user NSIS installer. WebView2 must already be installed. Animi's durable profile lives in the Windows per-user app-data directory for the stable `app.animi.desktop` identity; upgrades preserve that directory, and uninstall intentionally leaves it available for recovery. Use Settings → Clear local data when user-owned data should be deleted deliberately.

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
