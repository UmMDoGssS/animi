# Backend fixtures

Place deterministic AniList and migration fixtures beneath this directory. Tests must load these local files through injected storage and transport boundaries; they must never contact AniList or open the production app-data directory.

Fixture groups use stable, behavior-oriented names such as `anilist/search-success.json`, `anilist/rate-limited.json`, and `database/v0001.sqlite3` as those test suites are introduced.

The AniList fixtures cover normal, incomplete, partial-error, malformed, forbidden,
rate-limited, and offline behavior. Manga fixtures additionally cover manga, novel, and
one-shot formats; known and missing chapter or volume totals; fuzzy dates; same-kind
continuations; kind mismatches; and invalid negative totals. Tests load every response only
through an injected transport.
