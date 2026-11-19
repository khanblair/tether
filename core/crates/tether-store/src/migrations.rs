//! Numbered schema migrations and the runner that applies them.

use rusqlite::{Connection, TransactionBehavior};

use crate::StoreError;

/// One numbered schema change. The SQL runs inside a transaction, so it must not
/// contain `BEGIN` or `COMMIT`.
pub(crate) struct Migration {
    pub(crate) version: u32,
    pub(crate) sql: &'static str,
}

/// Every migration, in order. Versions start at 1 and have no gaps.
pub(crate) const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("../migrations/0001_init.sql"),
}];

/// The schema version kept in `PRAGMA user_version`. A new database reports 0.
pub(crate) fn current_version(conn: &Connection) -> Result<u32, StoreError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// Applies every migration newer than the database, one transaction each, and returns
/// the new version. The version is written in the same transaction as the schema
/// change, so a failed migration leaves the database at the version before it.
pub(crate) fn apply(conn: &mut Connection, migrations: &[Migration]) -> Result<u32, StoreError> {
    let current = current_version(conn)?;
    let supported = migrations.last().map_or(0, |m| m.version);
    if current > supported {
        return Err(StoreError::TooNew {
            found: current,
            supported,
        });
    }
    for migration in migrations.iter().filter(|m| m.version > current) {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(migration.sql)?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
    }
    current_version(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get::<_, u32>(0),
        )
        .unwrap()
            == 1
    }

    #[test]
    fn versions_start_at_one_and_have_no_gaps() {
        for (index, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(migration.version as usize, index + 1);
        }
    }

    #[test]
    fn a_failed_migration_rolls_back_its_schema_and_its_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        let migrations = [
            Migration {
                version: 1,
                sql: "CREATE TABLE first (id INTEGER);",
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE second (id INTEGER); INSERT INTO missing VALUES (1);",
            },
        ];
        assert!(apply(&mut conn, &migrations).is_err());
        assert_eq!(current_version(&conn).unwrap(), 1);
        assert!(table_exists(&conn, "first"));
        assert!(!table_exists(&conn, "second"));
    }

    #[test]
    fn applying_twice_changes_nothing() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(apply(&mut conn, MIGRATIONS).unwrap(), 1);
        assert_eq!(apply(&mut conn, MIGRATIONS).unwrap(), 1);
    }

    #[test]
    fn a_database_from_a_newer_app_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        let err = apply(&mut conn, MIGRATIONS).unwrap_err();
        assert!(matches!(
            err,
            StoreError::TooNew {
                found: 99,
                supported: 1
            }
        ));
    }
}
