pub mod anilist;
pub mod backups;
pub mod commands;
pub mod database;
pub mod domain;
pub mod recommendations;

use database::{Database, DatabasePaths};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let paths = DatabasePaths::from_root(app.path().app_data_dir()?)?;
            let database = tauri::async_runtime::block_on(Database::connect(&paths))?;
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::smoke::smoke_check,
            commands::library::list_library,
            commands::library::upsert_library_entry,
            commands::library::remove_library_entry
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("Animi failed to start: {error}");
        std::process::exit(1);
    }
}
