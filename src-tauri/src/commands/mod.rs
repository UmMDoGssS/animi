pub mod backups;
pub mod dashboard;
pub mod discovery;
pub mod library;
pub mod settings;
pub mod smoke;

pub use discovery::{
    get_home_airing, get_media_details, get_recommendations, list_trending,
    refresh_library_metadata, search_media,
};
pub use library::{
    advance_library_progress, list_library, remove_library_entry, upsert_library_entry,
};
pub use settings::{get_theme_preference, set_theme_preference};
pub use smoke::smoke_check;
