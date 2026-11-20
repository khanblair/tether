//! Git and GitHub support.

mod oauth;
mod pat;

pub use oauth::{DeviceGrant, GithubDeviceAuth, OAuthError, PollState};
pub use pat::{GithubPatAuth, PatError, PatInfo};
