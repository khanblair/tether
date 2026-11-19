//! Errors the store reports. The engine converts them to `CoreError` at its boundary.

use thiserror::Error;

/// A failure while opening, migrating, or using the database.
#[derive(Debug, Error)]
pub enum StoreError {
    /// SQLite reported an error.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The folder for the database file could not be created.
    #[error("could not create the database folder: {0}")]
    Io(#[from] std::io::Error),
    /// The database was written by a newer version of the app.
    #[error("the database is at schema version {found}, but this app supports up to {supported}")]
    TooNew {
        /// The version stored in the database.
        found: u32,
        /// The newest version this app knows.
        supported: u32,
    },
    /// SQLite could not switch the database to write-ahead logging.
    #[error("the database stayed in {mode} mode instead of write-ahead logging")]
    NoWriteAheadLog {
        /// The journal mode SQLite reported.
        mode: String,
    },
}
