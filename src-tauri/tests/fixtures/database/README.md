# Database migration fixtures

`src-tauri/migrations` is append-only after release. When a new migration is added, place an
immutable pre-upgrade SQLite fixture in this directory named `vNNNN.sqlite3`, together with a
small manifest describing the migration version and representative rows.

Migration tests must copy the fixture into a `tempfile` directory before opening it, run the
current migrator against the copy, and assert both schema and preserved records. Tests must never
open a fixture in place or resolve the production app-data directory. The initial schema is tested
from an empty temporary directory, so the first binary fixture is intentionally deferred until a
second migration creates a real upgrade boundary.

`v0002.sqlite3` pins the first-applied migration-2 checksum and the schema immediately before
`0003_activity_progress.sql`. Keep both the database and its manifest immutable so checksum drift
and upgrade regressions remain reproducible.
