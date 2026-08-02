# Backup compatibility fixtures

These files are immutable synthetic profiles used only by temporary-database tests. They contain
no real user data.

- `v1-mixed-profile.json` freezes the Animi 1.0.x backup contract, including a representative
  pre-existing manga row. Schema 1 imports must interpret primary progress by `mediaKind`, start
  manga volume progress at zero, and preserve stable activity event IDs.
- `v2-mixed-profile.json` freezes the version 1.1 mixed-media contract with independent chapter
  and volume progress, both activity high-water marks, and generalized activity deltas.

Export code must emit only the latest schema. Import tests may read these fixtures, but must not
rewrite them or apply them to a real application database.
