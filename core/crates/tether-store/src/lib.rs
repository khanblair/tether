//! SQLite storage: schema, migrations, queries, blobs.
//!
//! P1-02 holds the connection and schema 0001. Queries and the blob store arrive with the
//! tasks that need them.

mod approvals;
mod budgets;
mod checkpoints;
mod context;
#[cfg(test)]
mod context_tests;
mod db;
mod error;
mod mcp_servers;
mod migrations;
mod models;
#[cfg(test)]
mod models_tests;
mod permission_rules;
#[cfg(test)]
mod permission_rules_tests;
mod project_network;
#[cfg(test)]
mod project_network_tests;
mod project_trust;
mod projects;
#[cfg(test)]
mod projects_tests;
mod prompt_snapshots;
mod provider_configs;
#[cfg(test)]
mod schema_tests;
mod session_context;
mod session_fts;
mod session_load;
mod session_plans;
#[cfg(test)]
mod session_plans_tests;
#[cfg(test)]
mod session_reasoning_tests;
mod session_rows;
mod session_search;
mod session_title;
mod sessions;
#[cfg(test)]
mod sessions_rules_tests;
#[cfg(test)]
mod sessions_tests;
mod skill_settings;
mod todos;
#[cfg(test)]
mod todos_tests;
mod usage;
#[cfg(test)]
mod usage_tests;

pub use approvals::ApprovalRecord;
pub use budgets::BudgetSettings;
pub use checkpoints::{BlobStore, Checkpoint, CheckpointFile, SnapshotInput};
pub use context::{CompactionRecord, new_compaction_id};
pub use db::Store;
pub use error::StoreError;
pub use mcp_servers::{McpServerHeaderRecord, McpServerRecord, McpToolSettingRecord};
pub use models::{CATALOG_TTL_MS, catalog_is_fresh};
pub use projects::{GitIdentity, ProjectDefaults, ProjectRecord, ProjectRemote};
pub use prompt_snapshots::PromptSnapshot;
pub use session_fts::SessionSearchHit;
pub use sessions::SessionRecord;
pub use skill_settings::SkillChange;
pub use usage::{
    DayUsage, ModelUsage, ProjectUsage, ResponseUsageRecord, UsageTotals, UsageWindows,
    usage_day_offset,
};
