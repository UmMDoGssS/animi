use crate::domain::CommandError;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("database operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("could not prepare the database directory: {0}")]
    Storage(#[from] std::io::Error),
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored data violates the domain contract: {0}")]
    CorruptData(String),
}

impl DatabaseError {
    pub fn into_command_error(self) -> CommandError {
        match self {
            Self::CorruptData(_) | Self::Json(_) => CommandError::new(
                "DATABASE_DATA_INVALID",
                "Animi found invalid data in its local database.",
                false,
            ),
            Self::Sqlx(_) | Self::Migration(_) | Self::Storage(_) => CommandError::new(
                "DATABASE_UNAVAILABLE",
                "Animi could not access its local database.",
                true,
            ),
        }
    }
}
