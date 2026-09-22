pub mod anilist;
pub mod backups;
pub mod commands;
pub mod database;
pub mod domain;
pub mod recommendations;
mod release_smoke;

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::Arc;

use anilist::{AniListClient, AniListService, ReqwestTransport};
use database::{Database, DatabasePaths};
use tauri::Manager;

const RELEASE_SMOKE_MODE_ENV: &str = "ANIMI_RELEASE_SMOKE";
const RELEASE_SMOKE_DATA_DIR_ENV: &str = "ANIMI_RELEASE_SMOKE_DATA_DIR";

// Cargo's test-only linker argument covers integration tests. The library test harness is linked
// separately, so it declares the same generated resource explicitly on Windows.
#[cfg(all(test, target_os = "windows"))]
#[link(name = "resource", kind = "static")]
unsafe extern "C" {}

#[doc(hidden)]
pub fn register_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        commands::smoke::smoke_check,
        commands::backups::export_backup,
        commands::backups::preview_import,
        commands::backups::import_backup,
        commands::backups::get_settings_overview,
        commands::backups::clear_local_data,
        commands::library::list_library,
        commands::library::upsert_library_entry,
        commands::library::advance_library_progress,
        commands::library::remove_library_entry,
        commands::settings::get_theme_preference,
        commands::settings::set_theme_preference,
        commands::dashboard::get_dashboard_insights,
        commands::discovery::search_media,
        commands::discovery::get_media_details,
        commands::discovery::list_trending,
        commands::discovery::get_recommendations,
        commands::discovery::get_home_airing,
        commands::discovery::refresh_library_metadata
    ])
}

fn select_data_root(
    default_root: PathBuf,
    release_smoke_mode: Option<OsString>,
    release_smoke_root: Option<OsString>,
) -> PathBuf {
    if release_smoke_mode.as_deref() == Some(OsStr::new("1"))
        && let Some(root) = release_smoke_root.filter(|root| !root.is_empty())
    {
        return root.into();
    }

    default_root
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = register_commands(
        tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .setup(|app| {
                let data_root = select_data_root(
                    app.path().app_data_dir()?,
                    std::env::var_os(RELEASE_SMOKE_MODE_ENV),
                    std::env::var_os(RELEASE_SMOKE_DATA_DIR_ENV),
                );
                let paths = DatabasePaths::from_root(data_root)?;
                let database = tauri::async_runtime::block_on(Database::connect(&paths))?;
                // Mock transport requires the same explicit isolated-profile guard as storage.
                let fixture_root = if std::env::var(RELEASE_SMOKE_MODE_ENV).as_deref() == Ok("1")
                    && std::env::var_os(RELEASE_SMOKE_DATA_DIR_ENV)
                        .is_some_and(|root| !root.is_empty())
                {
                    std::env::var_os("ANIMI_RELEASE_SMOKE_FIXTURES")
                        .filter(|root| !root.is_empty())
                        .map(PathBuf::from)
                } else {
                    None
                };
                let transport: Arc<dyn anilist::AniListTransport> = match fixture_root {
                    Some(root) => Arc::new(release_smoke::FixtureTransport { root }),
                    None => Arc::new(ReqwestTransport::new()?),
                };
                let anilist = AniListService::new(database.clone(), AniListClient::new(transport));
                let backup_destination_picker =
                    commands::backups::BackupDestinationPicker::for_app(app.handle().clone());
                let backup_source_picker =
                    commands::backups::BackupSourcePicker::for_app(app.handle().clone());
                app.manage(database);
                app.manage(anilist);
                app.manage(backup_destination_picker);
                app.manage(backup_source_picker);
                app.manage(backups::ImportSessionStore::default());
                Ok(())
            }),
    )
    .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("Animi failed to start: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod release_smoke_tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::select_data_root;

    #[test]
    fn release_smoke_root_requires_the_exact_guard_and_a_nonempty_path() {
        let default = PathBuf::from("normal-profile");
        let isolated = OsString::from("isolated-profile");

        assert_eq!(
            select_data_root(
                default.clone(),
                Some(OsString::from("1")),
                Some(isolated.clone()),
            ),
            PathBuf::from(isolated)
        );
        assert_eq!(
            select_data_root(
                default.clone(),
                Some(OsString::from("true")),
                Some(OsString::from("isolated-profile")),
            ),
            default
        );
        assert_eq!(
            select_data_root(
                default.clone(),
                Some(OsString::from("1")),
                Some(OsString::new()),
            ),
            default
        );
    }
}
