use std::path::{Path, PathBuf};

use crate::domain::CommandError;

pub const DATABASE_FILE_NAME: &str = "animi.sqlite3";

/// Resolves database storage from an injected root. Production will inject
/// Tauri's app-data directory; tests inject a temporary directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabasePaths {
    root: PathBuf,
    database: PathBuf,
}

impl DatabasePaths {
    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self, CommandError> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(CommandError::new(
                "INVALID_STORAGE_PATH",
                "The application storage path is empty.",
                false,
            ));
        }

        let database = root.join(DATABASE_FILE_NAME);
        Ok(Self { root, database })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> &Path {
        &self.database
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{DATABASE_FILE_NAME, DatabasePaths};

    #[test]
    fn test_database_is_rooted_only_in_a_temporary_directory() {
        let temporary_root = tempdir().expect("a temporary test directory should be available");
        let paths = DatabasePaths::from_root(temporary_root.path())
            .expect("the temporary root should be accepted");

        assert!(paths.database().starts_with(temporary_root.path()));
        assert_eq!(
            paths.database().file_name(),
            Some(DATABASE_FILE_NAME.as_ref())
        );
    }

    #[test]
    fn empty_storage_root_is_rejected() {
        let error = DatabasePaths::from_root("")
            .expect_err("an empty root must never resolve to the current directory");

        assert_eq!(error.code, "INVALID_STORAGE_PATH");
    }
}
