//! Errors the store reports. The engine converts them to `CoreError` at its boundary.

use std::path::PathBuf;

use thiserror::Error;

/// A failure while opening, migrating, or using the database.
#[derive(Debug, Error)]
pub enum StoreError {
    /// A project name is blank, too long, or contains control characters.
    #[error("invalid project name")]
    InvalidProjectName,
    /// A project id did not name an existing row.
    #[error("project not found")]
    MissingProject,
    /// A checkpoint blob could not be read or written.
    #[error("checkpoint file error: {0}")]
    CheckpointIo(#[from] std::io::Error),
    /// A requested snapshot path is invalid or duplicated.
    #[error("invalid checkpoint path")]
    InvalidCheckpointPath,
    /// A checkpoint id is absent or belongs to another session.
    #[error("checkpoint not found in session")]
    MissingCheckpoint,
    /// Conversation rewind needs an assistant message tied to the checkpoint.
    #[error("checkpoint has no message in its session")]
    MissingCheckpointMessage,
    /// SQLite reported an error.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The folder for the database file could not be created.
    #[error("could not create the database folder {}: {source}", path.display())]
    CreateFolder {
        /// The folder that could not be created.
        path: PathBuf,
        /// The operating system error.
        source: std::io::Error,
    },
    /// A migration's SQL failed. The database stays at the version before it.
    #[error("migration {version} failed: {source}")]
    Migration {
        /// The version of the migration that failed.
        version: u32,
        /// What SQLite reported.
        source: rusqlite::Error,
    },
    /// A migration left rows that point at missing parents. It was rolled back.
    #[error("migration {version} left rows that break a foreign key")]
    ForeignKeyViolation {
        /// The version of the migration that was rolled back.
        version: u32,
    },
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
    /// A model price cannot be represented by SQLite's signed integer type.
    #[error("model {field} price {value} does not fit in SQLite INTEGER")]
    ModelPriceOverflow {
        /// The price field that overflowed.
        field: &'static str,
        /// The value that was rejected.
        value: u64,
    },
    /// A cached row contains an enum value unknown to this app.
    #[error("cached model row has invalid {field} value: {value}")]
    InvalidModelEnum {
        /// The enum column with the invalid value.
        field: &'static str,
        /// The stored text.
        value: String,
    },
    /// A cached model row contains a value outside the model type's range.
    #[error("cached model row has invalid {field} value: {value}")]
    InvalidModelValue {
        /// The model field with the invalid value.
        field: &'static str,
        /// The stored integer.
        value: i64,
    },
    /// Capability JSON could not be encoded for storage.
    #[error("could not encode model capabilities: {0}")]
    ModelCapsJson(#[from] serde_json::Error),
    /// A model in a replacement list names another provider.
    #[error("model belongs to {found}, but the replacement is for {expected}")]
    ModelProviderMismatch {
        /// The provider being replaced.
        expected: &'static str,
        /// The provider named by the model.
        found: &'static str,
    },
    /// The input list is too long to assign SQLite positions.
    #[error("model list is too long to store")]
    PositionOverflow,
    /// A session or message id did not name an existing row.
    #[error("session or message not found")]
    MissingSessionRow,
    /// A session title has no visible text.
    #[error("session title is empty")]
    EmptySessionTitle,
    /// A session is in the middle of a turn or approval.
    #[error("session is busy")]
    SessionBusy,
    /// A model id has no visible text.
    #[error("session model is empty")]
    EmptySessionModel,
    /// A session row has an enum value this version does not know.
    #[error("session row has invalid {field} value: {value}")]
    InvalidSessionEnum {
        /// The column.
        field: &'static str,
        /// The saved value.
        value: String,
    },
    /// A message part could not be decoded.
    #[error("message part JSON is invalid: {0}")]
    MessagePartJson(serde_json::Error),
    /// A stored sequence or usage counter does not fit the public type.
    #[error("session counter is out of range")]
    SessionCounterOverflow,
    /// A transient screen state was offered for storage. Only a finished test result is saved.
    #[error("provider status {status} is screen state and is not stored")]
    TransientStatus {
        /// The status text that was refused.
        status: &'static str,
    },
}
