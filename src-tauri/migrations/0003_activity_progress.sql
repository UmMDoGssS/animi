ALTER TABLE library_entries
    ADD COLUMN activity_progress INTEGER NOT NULL DEFAULT 0
        CHECK (activity_progress BETWEEN 0 AND 2147483647);

UPDATE library_entries SET activity_progress = progress;
