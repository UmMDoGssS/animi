use std::path::PathBuf;

use animi_lib::database::{Database, DatabasePaths};
use serde_json::json;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use tempfile::tempdir;

#[tokio::test]
async fn every_released_profile_fixture_reaches_the_latest_schema_and_reopens() {
    for fixture_name in [
        "v0001.sqlite3",
        "v0002.sqlite3",
        "v0003.sqlite3",
        "v0007.sqlite3",
    ] {
        let root = tempdir().expect("temporary upgrade profile");
        let paths = DatabasePaths::from_root(root.path()).expect("valid app-data paths");
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/database")
            .join(fixture_name);
        std::fs::copy(fixture, paths.database()).expect("copy immutable profile fixture");

        let database = Database::connect(&paths).await.expect("upgrade profile");
        let item = database
            .library()
            .list_items()
            .await
            .expect("preserved library")
            .pop()
            .expect("fixture library entry");
        assert_eq!(item.entry.anilist_id, 101);
        assert_eq!(item.entry.progress, 3);
        assert_eq!(
            database
                .settings()
                .get("theme")
                .await
                .expect("preserved setting")
                .expect("fixture theme")
                .value,
            json!("SYSTEM")
        );
        database.close().await;

        let options = SqliteConnectOptions::new()
            .filename(paths.database())
            .read_only(true);
        let mut inspection = SqliteConnection::connect_with(&options)
            .await
            .expect("inspect upgraded schema");
        let migrations = sqlx::query_as::<_, (i64, bool)>(
            "SELECT version, success FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&mut inspection)
        .await
        .expect("migration ledger");
        assert_eq!(migrations.len(), 8);
        assert_eq!(migrations.last(), Some(&(8, true)));
        inspection.close().await.expect("close inspection");

        let reopened = Database::connect(&paths)
            .await
            .expect("reopen upgraded profile");
        let reopened_entries = reopened.library().list().await.expect("reopened library");
        assert_eq!(
            reopened_entries
                .iter()
                .find(|entry| entry.anilist_id == 101)
                .expect("reopened anime entry")
                .progress,
            3
        );
    }
}

#[test]
fn release_configuration_keeps_the_private_windows_identity_stable() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tauri.conf.json")).expect("Tauri configuration"),
    )
    .expect("valid Tauri configuration JSON");
    assert_eq!(config["identifier"], "app.animi.desktop");
    assert_eq!(config["version"], "1.1.0");
    assert_eq!(config["bundle"]["targets"], json!(["nsis"]));
    assert_eq!(config["bundle"]["createUpdaterArtifacts"], false);
    assert_eq!(config["bundle"]["windows"]["allowDowngrades"], false);
    assert_eq!(
        config["bundle"]["windows"]["nsis"]["installMode"],
        "currentUser"
    );
    assert!(
        config["app"]["security"]["csp"]
            .as_str()
            .is_some_and(|csp| csp.contains("object-src 'none'"))
    );
}
