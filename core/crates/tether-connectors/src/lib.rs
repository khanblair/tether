//! Network connectors used by the Tether core.

mod backend;
mod content;
mod fetch;
mod html;
mod ip;

pub use backend::{ReqwestWebBackend, WebBackend, WebBody, WebFuture, WebReply};
pub use fetch::{FetchError, FetchOptions, FetchedPage, WebFetcher};
pub use ip::is_public_ip;
