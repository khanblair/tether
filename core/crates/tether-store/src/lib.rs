//! SQLite storage: schema, migrations, queries, blobs.
//!
//! P1-02 holds the connection and schema 0001. Queries and the blob store arrive with the
//! tasks that need them.

mod db;
mod error;
mod migrations;
#[cfg(test)]
mod schema_tests;

pub use db::Store;
pub use error::StoreError;
