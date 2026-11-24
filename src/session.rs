use std::{sync::Arc, time::Duration};

use reqwest::{Client, redirect::Policy};
use url::Url;

mod cookies;

use cookies::SessionCookieJar;
pub use cookies::{SessionCookie, SessionCookieError};

const USER_AGENT: &str = concat!("csustkit-rs/", env!("CARGO_PKG_VERSION"));
const CONNECTION_AND_READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error), uniffi(flat_error))]
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
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
impl CsustSession {
    pub fn export_cookies(&self) -> Vec<SessionCookie> {
        self.cookie_jar.export()
    }

    /// 按名称、域和路径更新 Cookie，保留本批未涉及的条目。
    /// 任一条目校验失败时，整批更新不会生效。
    pub fn update_cookies(&self, cookies: &[SessionCookie]) -> Result<(), SessionCookieError> {
        self.cookie_jar.update(cookies)
    }

    /// 使用完整快照替换当前会话的 Cookie。
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
}

impl CsustSessionBuilder {
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
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
    fn cookie_updates_preserve_other_entries_and_share_session_state() {
        let session = CsustSession::new().unwrap();
        let clone = session.clone();
        let origin = "https://auth.example.test/";
        let other_path = SessionCookie {
            origin: origin.into(),
            set_cookie: "session=other-path; Path=/sso".into(),
        };
        let other_host = SessionCookie {
            origin: "https://edu.example.test/".into(),
            set_cookie: "session=other-host".into(),
        };
        session
            .replace_cookies(&[
                SessionCookie {
                    origin: origin.into(),
                    set_cookie: "session=old".into(),
                },
                other_path.clone(),
                other_host.clone(),
            ])
            .unwrap();
        let original = session.export_cookies();

        let replacement = SessionCookie {
            origin: origin.into(),
            set_cookie: "session=new".into(),
        };
        let invalid = [
            replacement.clone(),
            SessionCookie {
                origin: origin.into(),
                set_cookie: "other=sample; Domain=unrelated.test".into(),
            },
        ];
        for import in [CsustSession::replace_cookies, CsustSession::update_cookies] {
            assert!(matches!(
                import(&clone, &invalid),
                Err(SessionCookieError::InvalidCookie { index: 1, .. })
            ));
            let exported = session.export_cookies();
            assert_eq!(exported.len(), original.len());
            assert!(original.iter().all(|cookie| exported.contains(cookie)));
        }

        let added = SessionCookie {
            origin: origin.into(),
            set_cookie: "added=from-webview".into(),
        };
        clone
            .update_cookies(&[replacement.clone(), added.clone()])
            .unwrap();
        let updated = session.export_cookies();
        assert_eq!(updated.len(), original.len() + 1);
        assert!(updated.contains(&replacement));
        let retained = [added, other_path, other_host];
        assert!(retained.iter().all(|cookie| updated.contains(cookie)));

        clone
            .update_cookies(&[SessionCookie {
                origin: origin.into(),
                set_cookie: "session=; Max-Age=0".into(),
            }])
            .unwrap();
        let expired = session.export_cookies();
        assert_eq!(expired.len(), retained.len());
        assert!(retained.iter().all(|cookie| expired.contains(cookie)));

        clone.clear_cookies();
        assert!(session.export_cookies().is_empty());
    }
}
