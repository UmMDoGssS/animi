pub mod anilist;
pub mod backups;
pub mod commands;
pub mod database;
pub mod domain;
pub mod recommendations;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::smoke::smoke_check])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("Animi failed to start: {error}");
        std::process::exit(1);
    }
}
