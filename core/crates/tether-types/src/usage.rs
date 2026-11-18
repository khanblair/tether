//! Token and cost totals for a session.

/// Running totals for one session. Cost is USD times 1,000,000.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UsageSnapshot {
    /// Input tokens sent to the provider.
    pub tokens_in: u64,
    /// Output tokens received from the provider.
    pub tokens_out: u64,
    /// Cost in millionths of a US dollar.
    pub cost_micros: u64,
}
