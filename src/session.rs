use std::{
    sync::Arc,
    time::Duration,
};

use reqwest::{Client, RequestBuilder, cookie::Jar, redirect::Policy};
use tokio::time::sleep;
use url::Url;

const USER_AGENT: &str = concat!("csustkit-rs/", env!("CARGO_PKG_VERSION"));
const MAX_RETRIES: u8 = 5;
const RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, thiserror::Error)]
pub enum SessionRequestError {
    #[error("请求连续 {attempts} 次返回空响应: {url}")]
    EmptyResponse { url: Url, attempts: u8 },
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}

pub(crate) struct SessionResponse {
    pub(crate) body: Vec<u8>,
}

/// Shared HTTP state for CSUST services.
///
/// The two clients use the same cookie jar: one follows redirects for normal
/// requests, while the other lets SSO inspect each redirect explicitly. Clones
/// share the same cookie jar and therefore the same login state.
#[derive(Clone)]
pub struct CsustSession {
    pub(crate) client: Client,
    pub(crate) no_redirect_client: Client,
    pub(crate) cookie_jar: Arc<Jar>,
}

impl CsustSession {
    pub fn new() -> Result<Self, reqwest::Error> {
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

        Ok(Self {
            client,
            no_redirect_client,
            cookie_jar,
        })
    }

    pub(crate) async fn send_with_retry<F>(
        &self,
        build_request: F,
    ) -> Result<SessionResponse, SessionRequestError>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut retries = 0_u8;
        loop {
            let response = build_request().send().await?;
            let url = response.url().clone();
            let body = response.bytes().await?.to_vec();

            if !body.is_empty() {
                return Ok(SessionResponse { body });
            }

            if retries >= MAX_RETRIES {
                return Err(SessionRequestError::EmptyResponse {
                    url,
                    attempts: retries + 1,
                });
            }

            retries += 1;
            sleep(RETRY_DELAY).await;
        }
    }
}
