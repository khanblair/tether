//! SQLite storage: schema, migrations, queries, blobs.
//!
//! P1-02 holds the connection and schema 0001. Queries and the blob store arrive with the
//! tasks that need them.

mod db;
mod error;
mod migrations;
mod models;
#[cfg(test)]
mod models_tests;
mod provider_configs;
#[cfg(test)]
mod schema_tests;

pub use db::Store;
pub use error::StoreError;
pub use models::{CATALOG_TTL_MS, catalog_is_fresh};
