"""Create the immutable Animi 1.0.1 (migration 7) compatibility fixture."""

from __future__ import annotations

import hashlib
import shutil
import sqlite3
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MIGRATIONS_ROOT = REPOSITORY_ROOT / "src-tauri" / "migrations"
FIXTURES_ROOT = REPOSITORY_ROOT / "src-tauri" / "tests" / "fixtures" / "database"
SOURCE = FIXTURES_ROOT / "v0003.sqlite3"
DESTINATION = FIXTURES_ROOT / "v0007.sqlite3"


def description(path: Path) -> str:
    return path.stem.split("_", 1)[1].replace("_", " ")


def main() -> None:
    if DESTINATION.exists():
        raise SystemExit(f"Refusing to overwrite immutable fixture: {DESTINATION}")

    shutil.copyfile(SOURCE, DESTINATION)
    try:
        connection = sqlite3.connect(DESTINATION)
        for version in range(4, 8):
            migration = next(MIGRATIONS_ROOT.glob(f"{version:04}_*.sql"))
            contents = migration.read_bytes()
            connection.executescript(contents.decode("utf-8"))
            connection.execute(
                """
                INSERT INTO _sqlx_migrations (
                    version, description, success, checksum, execution_time
                ) VALUES (?, ?, TRUE, ?, 0)
                """,
                (version, description(migration), hashlib.sha384(contents).digest()),
            )
        connection.execute(
            """
            INSERT INTO library_entries (
                anilist_id, media_kind, status, progress, activity_progress,
                score, notes, created_at, updated_at
            ) VALUES (
                202, 'MANGA', 'WATCHING', 7, 9,
                8.0, 'pre-existing manga',
                '2026-07-16T10:00:00.000Z', '2026-07-16T11:00:00.000Z'
            )
            """
        )
        connection.execute(
            """
            INSERT INTO media_snapshots (
                anilist_id, media_kind, title, cover_url, media_format,
                genres_json, tags_json, media_status, total_units,
                duration_minutes, release_date, cached_at
            ) VALUES (
                202, 'MANGA', 'Pre-existing Manga', NULL, 'MANGA',
                '["Drama"]', '[]', 'RELEASING', 20,
                NULL, '2026-01', '2026-07-16T10:00:00.000Z'
            )
            """
        )
        connection.execute(
            """
            INSERT INTO watch_activity (
                event_id, anilist_id, media_kind, progress_delta,
                episode_duration_minutes, occurred_at
            ) VALUES (
                '20000000-0000-4000-a000-000000000202',
                202, 'MANGA', 9, NULL, '2026-07-16T10:00:00.000Z'
            )
            """
        )
        connection.commit()
        connection.execute("VACUUM")
        connection.close()
    except Exception:
        DESTINATION.unlink(missing_ok=True)
        raise

    checksum = hashlib.sha256(DESTINATION.read_bytes()).hexdigest()
    print(f"created {DESTINATION.relative_to(REPOSITORY_ROOT)}")
    print(f"sha256 {checksum}")


if __name__ == "__main__":
    main()
