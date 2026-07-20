use std::sync::Arc;

use reqwest::{Client, cookie::Jar, redirect::Policy};

const USER_AGENT: &str = concat!("csustkit-rs/", env!("CARGO_PKG_VERSION"));

/// Shared HTTP state for CSUST services.
///
/// The two clients use the same cookie jar: one follows redirects for normal
/// requests, while the other lets SSO inspect each redirect explicitly.
pub struct CsustSession {
    pub(crate) client: Client,
    pub(crate) no_redirect_client: Client,
    pub(crate) cookie_jar: Arc<Jar>,
}

impl CsustSession {
    pub fn new() -> Result<Arc<Self>, reqwest::Error> {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_jar))
            .redirect(Policy::limited(10))
            .user_agent(USER_AGENT)
            .build()?;
        let no_redirect_client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_jar))
            .redirect(Policy::none())
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Arc::new(Self {
            client,
            no_redirect_client,
            cookie_jar,
        }))
    }
}
