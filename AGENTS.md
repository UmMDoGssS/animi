# Animi Agent Guide

## Purpose and Scope

This file applies to the entire repository. Read `PLAN.md` before making non-trivial changes. `PLAN.md` is the architectural source of truth; update it alongside an implementation only when the approved product scope or architecture changes.

Animi is a private, local-first anime tracker and recommendation application. Version 1 is a Windows desktop application built with Tauri 2, Svelte 5, TypeScript, Rust, SQLite, Tailwind CSS, AniList GraphQL, and Bun.

Version 1 is anime-focused and has no server, cloud database, analytics, AniList OAuth, live cross-device synchronization, or closed-app notifications. The local SQLite database is authoritative. Manga support and mobile clients are later additions, although shared types and responsive layouts must leave room for them.

## Working Agreement

- Inspect `PROJECT.md`, relevant source files, tests, and current repository changes before editing.
- Implement clear, scoped tasks independently and verify the result without asking for routine approval.
- State consequential assumptions and preserve unrelated user changes.
- Avoid speculative abstractions, broad refactors, or features outside the requested scope.
- Prefer small, reviewable changes. Update nearby tests and documentation when behavior changes.
- Use Bun exclusively for frontend dependencies, lockfiles, and scripts. Do not introduce npm, pnpm, or Yarn lockfiles.
- Maintained dependencies may be added when they materially reduce risk or complexity. Avoid overlapping libraries and explain additions in the handoff.
- Work on `main` unless the task requests a branch. Do not create commits unless requested.
- When commits are requested, keep them focused and use Conventional Commit prefixes such as `feat`, `fix`, `test`, `docs`, `refactor`, and `chore`.

## Architecture Boundaries

- Rust owns SQLite access, migrations, AniList requests, caching, rate-limit handling, imports, exports, and all durable state.
- The Svelte frontend communicates through typed Tauri commands. It must not execute SQL or call AniList directly.
- Use camelCase JSON fields at the IPC boundary. Rust may retain idiomatic snake_case internally with Serde renaming.
- Return typed success values and structured command errors containing `code`, `message`, and `retryable`.
- Treat AniList as an on-demand metadata provider, not as application storage. Do not bulk-collect data.
- Honor cache TTLs, rate-limit response headers, `Retry-After`, offline operation, and stale-data indicators defined in `PLAN.md`.
- Store timestamps consistently in UTC and perform multi-record writes in transactions.
- Use versioned, forward-migratable backup formats. Validate a complete import before opening its write transaction.
- Released database migrations are append-only. Never rewrite migration history that may already exist on a user device.
- Destructive data operations require explicit confirmation and must not silently discard library entries.

## Frontend Style Guide

- Use TypeScript strict mode. Avoid `any`; if an unsafe boundary is unavoidable, validate it and isolate it behind a typed adapter.
- Use Svelte 5 conventions and built-in state primitives. Do not add a global state library without a concrete need.
- Keep components focused. Extract repeated behavior and presentation, but do not split components solely to reduce line counts.
- Use PascalCase for Svelte components, camelCase for TypeScript values and functions, and descriptive domain-oriented names.
- Model loading, empty, offline, stale, and error states explicitly. Do not leave failed asynchronous work as an indefinite spinner.
- Keep user-facing strings clear and concise. Comments should explain why a constraint or workaround exists, not narrate obvious code.
- Format with Prettier, lint with ESLint, and validate Svelte and TypeScript with `svelte-check`.

## Visual Design and Accessibility

- Use Catppuccin Macchiato for dark mode and Catppuccin Latte for light mode.
- Default to the operating-system color preference and provide a persisted manual theme toggle.
- Define colors through semantic CSS custom properties such as surface, overlay, text, muted text, accent, success, warning, and danger. Do not scatter raw palette values through components.
- Use frosted-glass panels with translucent surfaces, subtle borders, restrained backdrop blur, and opaque fallbacks when blur is unsupported.
- Make the application feel cat-themed through tasteful icons, microcopy, and small decorative details. Cat motifs must remain secondary to content and usability.
- Favor cover art, rounded panels, clear information hierarchy, and restrained animation. Avoid visual noise and excessive gradients.
- Maintain readable contrast in both palettes, full keyboard operation, visible focus styles, meaningful alternative text, and reduced-motion behavior.
- Test important layouts at desktop widths and a narrow responsive width so future mobile reuse remains practical.

## Rust Style Guide

- Follow `rustfmt` and keep Clippy warning-free with all targets and features enabled.
- Use typed domain errors and propagate failures with `Result`. Do not use `unwrap` or `expect` on production paths.
- Keep Tauri commands thin. Put database, AniList, backup, and recommendation behavior in testable modules.
- Prefer explicit domain types and validated constructors over loosely related primitive parameters.
- Keep blocking work off the async runtime and avoid holding locks or database transactions across network requests.
- Use parameterized SQL only. Enable and test SQLite foreign-key enforcement.

## Testing Guide

- Run focused tests while developing, then run the balanced quality gate before handoff.
- Frontend unit and component tests use Vitest and Svelte Testing Library.
- Rust unit tests cover domain behavior; integration tests cover migrations, repositories, cache behavior, imports, exports, and command boundaries.
- Automated tests must never contact the live AniList API. Use deterministic fixtures and mocked responses for success, offline, malformed GraphQL data, `403`, and `429 Retry-After` cases.
- Database tests use temporary databases and must never open or modify a real user database.
- Cover library status transitions, progress bounds, missing episode totals, unknown sequels, stale cache behavior, recommendation fallback, backup conflicts, and transaction rollback.
- Add or update end-to-end coverage for affected critical user flows. Run a production Tauri build for packaging, migration, native-configuration, or release changes.
- Documentation-only changes may skip runtime suites when the handoff explicitly says so.

Expected full quality gate after the initial scaffold provides these scripts:

```powershell
bun run format:check
bun run lint
bun run check
bun run test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

## Completion Checklist

- Confirm the requested behavior and relevant edge cases are covered.
- Run targeted tests and the applicable quality-gate commands.
- Ensure formatting and generated lockfiles are current without introducing a non-Bun frontend lockfile.
- Review the final diff for accidental scope expansion, secrets, user-data risks, and unrelated changes.
- Report what changed, what was verified, any tests that could not run, and any justified dependency additions.
