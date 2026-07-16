pub mod dashboard;
pub mod discovery;
pub mod library;
pub mod settings;
pub mod smoke;

pub use discovery::{get_media_details, list_trending, refresh_library_metadata, search_media};
pub use library::{list_library, remove_library_entry, upsert_library_entry};
pub use settings::{get_theme_preference, set_theme_preference};
pub use smoke::smoke_check;
