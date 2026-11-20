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
pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../migrations/0002_message_fts.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("../migrations/0003_checkpoints.sql"),
    },
];

/// The schema version kept in `PRAGMA user_version`. A new database reports 0.
pub(crate) fn current_version(conn: &Connection) -> Result<u32, StoreError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// The newest version in `migrations`, or 0 when the list is empty.
pub(crate) fn latest(migrations: &[Migration]) -> u32 {
    migrations.last().map_or(0, |m| m.version)
}

/// Applies every migration newer than the database, one transaction each, and returns
/// the new version. The version is written in the same transaction as the schema
/// change, so a failed migration leaves the database at the version before it.
pub(crate) fn apply(conn: &mut Connection, migrations: &[Migration]) -> Result<u32, StoreError> {
    let supported = latest(migrations);
    let current = current_version(conn)?;
    if current > supported {
        return Err(StoreError::TooNew {
            found: current,
            supported,
        });
    }
    for migration in migrations.iter().filter(|m| m.version > current) {
        apply_one(conn, migration, supported)?;
    }
    current_version(conn)
}

// Foreign keys cannot be switched inside a transaction, and with them on, dropping a
// parent table deletes its children. Rebuilding a table is the only way to change a
// CHECK list, so they go off for the migration and are checked before it commits.
fn apply_one(
    conn: &mut Connection,
    migration: &Migration,
    supported: u32,
) -> Result<(), StoreError> {
    conn.pragma_update(None, "foreign_keys", false)?;
    let result = run_in_transaction(conn, migration, supported);
    let restored = conn.pragma_update(None, "foreign_keys", true);
    result?;
    Ok(restored?)
}

fn run_in_transaction(
    conn: &mut Connection,
    migration: &Migration,
    supported: u32,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    // Another opener may have migrated while this one waited for the write lock.
    let found = current_version(&tx)?;
    if found > supported {
        return Err(StoreError::TooNew { found, supported });
    }
    if found >= migration.version {
        return Ok(());
    }
    tx.execute_batch(migration.sql)
        .map_err(|source| StoreError::Migration {
            version: migration.version,
            source,
        })?;
    if tx.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(StoreError::ForeignKeyViolation {
            version: migration.version,
        });
    }
    tx.pragma_update(None, "user_version", migration.version)?;
    Ok(tx.commit()?)
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
        let err = apply(&mut conn, &migrations).unwrap_err();
        assert!(matches!(err, StoreError::Migration { version: 2, .. }));
        assert_eq!(current_version(&conn).unwrap(), 1);
        assert!(table_exists(&conn, "first"));
        assert!(!table_exists(&conn, "second"));
    }

    fn parent_child_migrations() -> [Migration; 2] {
        [
            Migration {
                version: 1,
                sql: "CREATE TABLE parent (id INTEGER PRIMARY KEY, kind TEXT CHECK (kind IN ('a')));
                      CREATE TABLE child (id INTEGER PRIMARY KEY,
                          parent_id INTEGER NOT NULL REFERENCES parent (id) ON DELETE CASCADE);",
            },
            // The standard table rebuild that widens a CHECK list.
            Migration {
                version: 2,
                sql: "CREATE TABLE parent_new (id INTEGER PRIMARY KEY, kind TEXT CHECK (kind IN ('a', 'b')));
                      INSERT INTO parent_new SELECT id, kind FROM parent;
                      DROP TABLE parent;
                      ALTER TABLE parent_new RENAME TO parent;",
            },
        ]
    }

    #[test]
    fn rebuilding_a_parent_table_keeps_its_children() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        let [first, second] = parent_child_migrations();
        apply(&mut conn, &[first]).unwrap();
        conn.execute_batch("INSERT INTO parent VALUES (1, 'a'); INSERT INTO child VALUES (1, 1);")
            .unwrap();
        let [first, _] = parent_child_migrations();
        apply(&mut conn, &[first, second]).unwrap();
        let children: i64 = conn
            .query_row("SELECT COUNT(*) FROM child", [], |row| row.get(0))
            .unwrap();
        assert_eq!(children, 1);
    }

    #[test]
    fn a_migration_that_orphans_rows_is_rolled_back() {
        let mut conn = Connection::open_in_memory().unwrap();
        let [first, _] = parent_child_migrations();
        apply(&mut conn, &[first]).unwrap();
        conn.execute_batch("INSERT INTO parent VALUES (1, 'a'); INSERT INTO child VALUES (1, 1);")
            .unwrap();
        let [first, _] = parent_child_migrations();
        let orphaning = Migration {
            version: 2,
            sql: "DELETE FROM parent;",
        };
        let err = apply(&mut conn, &[first, orphaning]).unwrap_err();
        assert!(matches!(
            err,
            StoreError::ForeignKeyViolation { version: 2 }
        ));
        assert_eq!(current_version(&conn).unwrap(), 1);
        let parents: i64 = conn
            .query_row("SELECT COUNT(*) FROM parent", [], |row| row.get(0))
            .unwrap();
        assert_eq!(parents, 1);
    }

    #[test]
    fn every_migration_file_is_listed_in_order_and_matches_its_sql() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        files.sort();
        assert_eq!(
            files.len(),
            MIGRATIONS.len(),
            "a migration file is missing from MIGRATIONS"
        );
        for (file, migration) in files.iter().zip(MIGRATIONS) {
            let name = file.file_name().unwrap().to_str().unwrap();
            assert!(
                name.starts_with(&format!("{:04}_", migration.version)),
                "{name}"
            );
            assert_eq!(std::fs::read_to_string(file).unwrap(), migration.sql);
        }
    }

    #[test]
    fn applying_twice_changes_nothing() {
        let mut conn = Connection::open_in_memory().unwrap();
        let latest_version = latest(MIGRATIONS);
        assert_eq!(apply(&mut conn, MIGRATIONS).unwrap(), latest_version);
        assert_eq!(apply(&mut conn, MIGRATIONS).unwrap(), latest_version);
    }

    #[test]
    fn a_database_from_a_newer_app_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        let err = apply(&mut conn, MIGRATIONS).unwrap_err();
        assert!(matches!(
            err,
            StoreError::TooNew { found: 99, supported } if supported == latest(MIGRATIONS)
        ));
    }
}
