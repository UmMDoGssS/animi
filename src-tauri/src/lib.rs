pub mod anilist;
pub mod backups;
pub mod commands;
pub mod database;
pub mod domain;
pub mod recommendations;

use std::sync::Arc;

use anilist::{AniListClient, AniListService, ReqwestTransport};
use database::{Database, DatabasePaths};
use tauri::Manager;

// Cargo's test-only linker argument covers integration tests. The library test harness is linked
// separately, so it declares the same generated resource explicitly on Windows.
#[cfg(all(test, target_os = "windows"))]
#[link(name = "resource", kind = "static")]
unsafe extern "C" {}

#[doc(hidden)]
pub fn register_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        commands::smoke::smoke_check,
        commands::library::list_library,
        commands::library::upsert_library_entry,
        commands::library::remove_library_entry,
        commands::settings::get_theme_preference,
        commands::settings::set_theme_preference,
        commands::dashboard::get_dashboard_insights,
        commands::discovery::search_media,
        commands::discovery::get_media_details,
        commands::discovery::list_trending,
        commands::discovery::refresh_library_metadata
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = register_commands(tauri::Builder::default().setup(|app| {
        let paths = DatabasePaths::from_root(app.path().app_data_dir()?)?;
        let database = tauri::async_runtime::block_on(Database::connect(&paths))?;
        let transport = ReqwestTransport::new()?;
        let anilist =
            AniListService::new(database.clone(), AniListClient::new(Arc::new(transport)));
        app.manage(database);
        app.manage(anilist);
        Ok(())
    }))
    .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("Animi failed to start: {error}");
        std::process::exit(1);
    }
}
