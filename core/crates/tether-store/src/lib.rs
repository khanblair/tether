//! SQLite storage: schema, migrations, queries, blobs.
//!
//! P1-02 holds the connection and schema 0001. Queries and the blob store arrive with the
//! tasks that need them.

mod approvals;
mod checkpoints;
mod db;
mod error;
mod migrations;
mod models;
#[cfg(test)]
mod models_tests;
mod permission_rules;
#[cfg(test)]
mod permission_rules_tests;
mod projects;
#[cfg(test)]
mod projects_tests;
mod provider_configs;
#[cfg(test)]
mod schema_tests;
mod session_fts;
mod session_load;
mod session_search;
mod sessions;
#[cfg(test)]
mod sessions_rules_tests;
#[cfg(test)]
mod sessions_tests;

pub use approvals::ApprovalRecord;
pub use checkpoints::{BlobStore, Checkpoint, CheckpointFile, SnapshotInput};
pub use db::Store;
pub use error::StoreError;
pub use models::{CATALOG_TTL_MS, catalog_is_fresh};
pub use projects::{ProjectDefaults, ProjectRecord};
pub use session_fts::SessionSearchHit;
pub use sessions::SessionRecord;
