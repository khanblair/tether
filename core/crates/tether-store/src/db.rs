//! The database connection: open, set up, and migrate.

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::StoreError;
use crate::migrations::{self, MIGRATIONS};

/// How long a statement waits for a lock before it fails.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// How many times `Store::open` tries when another connection holds the database.
const OPEN_ATTEMPTS: u32 = 20;

/// The pause between those tries.
const RETRY_DELAY: Duration = Duration::from_millis(50);

/// Runs `attempt`, repeating it when another connection holds the database. Several
/// connections switching to WAL at once can get "database is locked" even with a busy
/// timeout, and every step of opening is safe to repeat.
fn retry_when_busy<T>(mut attempt: impl FnMut() -> Result<T, StoreError>) -> Result<T, StoreError> {
    for _ in 1..OPEN_ATTEMPTS {
        match attempt() {
            Err(error) if is_busy(&error) => std::thread::sleep(RETRY_DELAY),
            result => return result,
        }
    }
    attempt()
}

fn is_busy(error: &StoreError) -> bool {
    matches!(
        error,
        StoreError::Sqlite(rusqlite::Error::SqliteFailure(failure, _))
            if failure.code == rusqlite::ErrorCode::DatabaseBusy
    )
}

/// The app's SQLite database. One `Store` owns the write connection.
#[derive(Debug)]
pub struct Store {
    pub(crate) conn: Connection,
}

impl Store {
    /// Opens the database file, creating it and its folder on first use, and brings
    /// the schema up to date.
    ///
    /// # Errors
    /// Returns an error when the folder or file cannot be created, when SQLite cannot
    /// use write-ahead logging, when a migration fails, or when the database was
    /// written by a newer app.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|source| StoreError::CreateFolder {
                path: folder.to_owned(),
                source,
            })?;
        }
        retry_when_busy(|| Self::open_once(path))
    }

    fn open_once(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        let mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(StoreError::NoWriteAheadLog { mode });
        }
        Self::prepare(conn)
    }

    /// Opens a private in-memory database with the same schema. Tests use it.
    ///
    /// # Errors
    /// Returns an error when a migration fails.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::prepare(Connection::open_in_memory()?)
    }

    /// The schema version of the open database.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the version.
    pub fn schema_version(&self) -> Result<u32, StoreError> {
        migrations::current_version(&self.conn)
    }

    // Foreign keys and the journal mode cannot change inside a transaction, so both are
    // set before the migrations open theirs.
    fn prepare(mut conn: Connection) -> Result<Self, StoreError> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        migrations::apply(&mut conn, MIGRATIONS)?;
        Ok(Self { conn })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pragma_text(store: &Store, name: &str) -> String {
        store
            .conn
            .pragma_query_value(None, name, |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn a_fresh_install_creates_the_folder_and_the_latest_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("files").join("db").join("tether.db");
        let store = Store::open(&path).unwrap();
        assert!(path.exists());
        assert_eq!(
            store.schema_version().unwrap(),
            migrations::latest(MIGRATIONS)
        );
        assert_eq!(pragma_text(&store, "journal_mode"), "wal");
    }

    #[test]
    fn reopening_keeps_the_data_and_the_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tether.db");
        {
            let store = Store::open(&path).unwrap();
            store
                .conn
                .execute(
                    "INSERT INTO projects (id, name, path, created_at) VALUES ('p1', 'demo', 'demo', 1)",
                    [],
                )
                .unwrap();
        }
        let store = Store::open(&path).unwrap();
        let name: String = store
            .conn
            .query_row("SELECT name FROM projects WHERE id = 'p1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "demo");
        assert_eq!(
            store.schema_version().unwrap(),
            migrations::latest(MIGRATIONS)
        );
    }

    fn open_after(
        start: std::sync::Arc<std::sync::Barrier>,
        path: std::path::PathBuf,
    ) -> std::thread::JoinHandle<Result<Store, StoreError>> {
        std::thread::spawn(move || {
            start.wait();
            Store::open(&path)
        })
    }

    fn open_from_threads(path: &Path, threads: usize) -> Vec<Result<Store, StoreError>> {
        let start = std::sync::Arc::new(std::sync::Barrier::new(threads));
        let handles: Vec<_> = (0..threads)
            .map(|_| open_after(std::sync::Arc::clone(&start), path.to_owned()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    }

    #[test]
    fn several_threads_opening_a_fresh_database_all_succeed() {
        let latest = migrations::latest(MIGRATIONS);
        let dir = tempfile::tempdir().unwrap();
        for round in 0..5 {
            let path = dir.path().join(format!("round{round}.db"));
            let versions: Vec<u32> = open_from_threads(&path, 4)
                .into_iter()
                .map(|result| result.unwrap().schema_version().unwrap())
                .collect();
            assert_eq!(versions, vec![latest; 4]);
        }
    }

    #[test]
    fn foreign_keys_are_on_after_open() {
        let store = Store::open_in_memory().unwrap();
        let on: i64 = store
            .conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert_eq!(on, 1);
    }

    #[test]
    fn opening_a_database_from_a_newer_app_fails_and_keeps_its_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tether.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", 42).unwrap();
        }
        let err = Store::open(&path).unwrap_err();
        assert!(matches!(err, StoreError::TooNew { found: 42, .. }));
        let conn = Connection::open(&path).unwrap();
        assert_eq!(migrations::current_version(&conn).unwrap(), 42);
    }
}
