pub mod campus_card;
pub mod chaoxing;
pub mod connection;
pub mod edu;
mod enums;
pub mod mooc;
pub mod physics_experiment;
mod request;
pub mod session;
pub mod sso;
mod url_factory;
pub mod utils;
pub mod webvpn;

#[cfg(feature = "uniffi")]
mod ffi;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub use connection::ConnectionMode;
pub use enums::{Campus, DayOfWeek};
pub use session::{
    CsustSession, CsustSessionBuilder, SessionCookie, SessionCookieError, SessionRequestError,
};
pub use url_factory::{ServiceDomain, make_url};
pub use webvpn::{WebVpnError, webvpn_decrypt_url, webvpn_encrypt_url};
