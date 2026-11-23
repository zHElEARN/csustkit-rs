use std::{sync::Arc, time::Duration};

use reqwest::{Certificate, Client, redirect::Policy};
use url::Url;

mod cookies;

use cookies::SessionCookieJar;
pub use cookies::{SessionCookie, SessionCookieError};

const USER_AGENT: &str = concat!("csustkit-rs/", env!("CARGO_PKG_VERSION"));
const CONNECTION_AND_READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
pub enum SessionRequestError {
    #[error("请求连续 {attempts} 次返回空响应: {url}")]
    EmptyResponse { url: Url, attempts: u8 },
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}

/// Shared HTTP state for CSUST services.
///
/// Clones share the HTTP client and cookie jar, including the login state.
/// The client times out after 60 seconds of connecting or waiting for
/// response data. Each successful read resets the read timeout.
#[derive(Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct CsustSession {
    pub(crate) client: Client,
    cookie_jar: Arc<SessionCookieJar>,
}

impl CsustSession {
    pub fn new() -> Result<Self, reqwest::Error> {
        Self::builder().build()
    }

    pub fn builder() -> CsustSessionBuilder {
        CsustSessionBuilder::default()
    }

    pub fn export_cookies(&self) -> Vec<SessionCookie> {
        self.cookie_jar.export()
    }

    pub fn replace_cookies(&self, cookies: &[SessionCookie]) -> Result<(), SessionCookieError> {
        self.cookie_jar.replace(cookies)
    }

    pub fn clear_cookies(&self) {
        self.cookie_jar.clear();
    }
}

#[derive(Default)]
pub struct CsustSessionBuilder {
    user_agent: Option<String>,
    root_certificates: Vec<Certificate>,
}

impl CsustSessionBuilder {
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn add_root_certificate(mut self, certificate: Certificate) -> Self {
        self.root_certificates.push(certificate);
        self
    }

    pub fn build(self) -> Result<CsustSession, reqwest::Error> {
        let cookie_jar = Arc::new(SessionCookieJar::default());
        let client = Client::builder()
            .connect_timeout(CONNECTION_AND_READ_TIMEOUT)
            .read_timeout(CONNECTION_AND_READ_TIMEOUT)
            .cookie_provider(Arc::clone(&cookie_jar))
            .redirect(Policy::limited(10))
            .user_agent(self.user_agent.as_deref().unwrap_or(USER_AGENT))
            .tls_certs_merge(self.root_certificates)
            .build()?;

        Ok(CsustSession { client, cookie_jar })
    }
}

#[cfg(test)]
mod tests {
    use reqwest::cookie::CookieStore;

    use super::{CsustSession, SessionCookie, SessionCookieError, Url};

    #[test]
    fn cookie_export_and_restore_preserve_scope_and_expiration() {
        let session = CsustSession::new().unwrap();
        session
            .replace_cookies(&[
                SessionCookie {
                    origin: "https://auth.example.test/sso;route/login".into(),
                    set_cookie: "session=sample; HttpOnly; Secure; SameSite=Lax".into(),
                },
                SessionCookie {
                    origin: "https://auth.example.test/".into(),
                    set_cookie: "persistent=sample; Domain=.example.test; Path=/; Max-Age=3600; SameSite=None".into(),
                },
                SessionCookie {
                    origin: "https://auth.example.test/".into(),
                    set_cookie: "expired=sample; Max-Age=0".into(),
                },
            ])
            .unwrap();

        let cookies = session.export_cookies();
        assert_eq!(cookies.len(), 2);
        let session_cookie = cookies
            .iter()
            .find(|cookie| cookie.set_cookie.starts_with("session="))
            .unwrap();
        assert_eq!(
            session_cookie.origin,
            "https://auth.example.test/sso;route/"
        );
        assert!(session_cookie.set_cookie.contains("; HttpOnly"));
        assert!(session_cookie.set_cookie.contains("; SameSite=Lax"));
        let persistent = cookies
            .iter()
            .find(|cookie| cookie.set_cookie.starts_with("persistent="))
            .unwrap();
        assert!(persistent.set_cookie.contains("; Expires="));
        assert!(!persistent.set_cookie.contains("; Max-Age="));

        let restored = CsustSession::new().unwrap();
        let data = serde_json::to_vec(&cookies).unwrap();
        let imported: Vec<SessionCookie> = serde_json::from_slice(&data).unwrap();
        restored.replace_cookies(&imported).unwrap();
        let exported_again = restored.export_cookies();
        assert_eq!(exported_again.len(), cookies.len());
        assert!(cookies.iter().all(|cookie| exported_again.contains(cookie)));

        let scoped = Url::parse("https://auth.example.test/sso;route/home").unwrap();
        let header = restored.cookie_jar.cookies(&scoped).unwrap();
        assert!(header.to_str().unwrap().contains("session=sample"));
        for url in [
            "https://auth.example.test/sso/home",
            "http://auth.example.test/sso;route/home",
            "https://sub.auth.example.test/sso;route/home",
        ] {
            let header = restored
                .cookie_jar
                .cookies(&Url::parse(url).unwrap())
                .unwrap();
            assert_eq!(header.to_str().unwrap(), "persistent=sample");
        }
    }

    #[test]
    fn invalid_replacement_preserves_cookies_and_clear_is_shared() {
        let session = CsustSession::new().unwrap();
        let clone = session.clone();
        let origin = "https://auth.example.test/";
        session
            .replace_cookies(&[SessionCookie {
                origin: origin.into(),
                set_cookie: "session=old".into(),
            }])
            .unwrap();

        let result = clone.replace_cookies(&[
            SessionCookie {
                origin: origin.into(),
                set_cookie: "session=new".into(),
            },
            SessionCookie {
                origin: origin.into(),
                set_cookie: "other=sample; Domain=unrelated.test".into(),
            },
        ]);
        assert!(matches!(
            result,
            Err(SessionCookieError::InvalidCookie { index: 1, .. })
        ));
        assert_eq!(session.export_cookies()[0].set_cookie, "session=old");

        clone.clear_cookies();
        assert!(session.export_cookies().is_empty());
    }
}
