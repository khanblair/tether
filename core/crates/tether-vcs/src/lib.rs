//! Git and GitHub support.

mod oauth;

pub use oauth::{DeviceGrant, GithubDeviceAuth, OAuthError, PollState};
