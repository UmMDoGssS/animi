use chrono::Utc;
use tauri::State;

use crate::database::{Database, SettingRecord};
use crate::domain::{CommandError, ThemePreference};

const THEME_PREFERENCE_KEY: &str = "themePreference";

#[tauri::command(rename_all = "camelCase")]
pub async fn get_theme_preference(
    database: State<'_, Database>,
) -> Result<ThemePreference, CommandError> {
    get_theme_preference_with_database(&database).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn set_theme_preference(
    database: State<'_, Database>,
    preference: ThemePreference,
) -> Result<ThemePreference, CommandError> {
    set_theme_preference_with_database(&database, preference).await
}

pub(crate) async fn get_theme_preference_with_database(
    database: &Database,
) -> Result<ThemePreference, CommandError> {
    let Some(setting) = database
        .settings()
        .get(THEME_PREFERENCE_KEY)
        .await
        .map_err(|error| error.into_command_error())?
    else {
        return Ok(ThemePreference::System);
    };

    serde_json::from_value(setting.value).map_err(|_| {
        CommandError::new(
            "SETTING_VALUE_INVALID",
            "The saved theme preference is invalid.",
            false,
        )
    })
}

pub(crate) async fn set_theme_preference_with_database(
    database: &Database,
    preference: ThemePreference,
) -> Result<ThemePreference, CommandError> {
    database
        .settings()
        .upsert_many(&[SettingRecord {
            key: THEME_PREFERENCE_KEY.to_owned(),
            value: serde_json::json!(preference),
            updated_at: Utc::now(),
        }])
        .await
        .map_err(|error| error.into_command_error())?;

    Ok(preference)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use tempfile::tempdir;

    use crate::database::{Database, DatabasePaths, SettingRecord};
    use crate::domain::ThemePreference;

    use super::{
        THEME_PREFERENCE_KEY, get_theme_preference_with_database,
        set_theme_preference_with_database,
    };

    async fn test_database() -> (tempfile::TempDir, Database) {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid database path");
        let database = Database::connect(&paths).await.expect("test database");
        (root, database)
    }

    #[tokio::test]
    async fn defaults_to_system_and_persists_manual_preferences() {
        let (root, database) = test_database().await;

        assert_eq!(
            get_theme_preference_with_database(&database)
                .await
                .expect("default preference"),
            ThemePreference::System
        );

        assert_eq!(
            set_theme_preference_with_database(&database, ThemePreference::Macchiato)
                .await
                .expect("saved preference"),
            ThemePreference::Macchiato
        );
        database.close().await;
        let paths = DatabasePaths::from_root(root.path()).expect("valid reopened database path");
        let reopened = Database::connect(&paths).await.expect("reopened database");
        assert_eq!(
            get_theme_preference_with_database(&reopened)
                .await
                .expect("persisted preference"),
            ThemePreference::Macchiato
        );
    }

    #[tokio::test]
    async fn rejects_an_invalid_stored_preference() {
        let (_root, database) = test_database().await;
        database
            .settings()
            .upsert_many(&[SettingRecord {
                key: THEME_PREFERENCE_KEY.to_owned(),
                value: json!("MIDNIGHT"),
                updated_at: Utc::now(),
            }])
            .await
            .expect("invalid fixture is still valid JSON");

        let error = get_theme_preference_with_database(&database)
            .await
            .expect_err("unknown preference should be rejected");
        assert_eq!(error.code, "SETTING_VALUE_INVALID");
        assert!(!error.retryable);
    }
}
