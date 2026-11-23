use std::sync::{PoisonError, RwLock};

use cookie_store::{Cookie, CookieDomain, CookieError, CookieExpiration, CookieStore};
use reqwest::{cookie::CookieStore as ReqwestCookieStore, header::HeaderValue};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCookie {
    pub origin: String,
    pub set_cookie: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionCookieError {
    #[error("Cookie {index} 的来源 URL 无效")]
    InvalidOrigin { index: usize },
    #[error("Cookie {index} 的 Set-Cookie 值无效")]
    InvalidHeader { index: usize },
    #[error("Cookie {index} 无法导入: {source}")]
    InvalidCookie {
        index: usize,
        #[source]
        source: CookieError,
    },
}

#[derive(Default)]
pub(super) struct SessionCookieJar(RwLock<CookieStore>);

impl SessionCookieJar {
    pub(super) fn export(&self) -> Vec<SessionCookie> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter_unexpired()
            .filter_map(export_cookie)
            .collect()
    }

    pub(super) fn replace(&self, cookies: &[SessionCookie]) -> Result<(), SessionCookieError> {
        let mut store = CookieStore::default();
        for (index, cookie) in cookies.iter().enumerate() {
            let origin = Url::parse(&cookie.origin)
                .ok()
                .filter(|url| matches!(url.scheme(), "http" | "https") && url.has_host())
                .ok_or(SessionCookieError::InvalidOrigin { index })?;
            HeaderValue::from_str(&cookie.set_cookie)
                .map_err(|_| SessionCookieError::InvalidHeader { index })?;
            match store.parse(&cookie.set_cookie, &origin) {
                Ok(_) | Err(CookieError::Expired) => {}
                Err(source) => return Err(SessionCookieError::InvalidCookie { index, source }),
            }
        }
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = store;
        Ok(())
    }

    pub(super) fn clear(&self) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

impl ReqwestCookieStore for SessionCookieJar {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let mut store = self.0.write().unwrap_or_else(PoisonError::into_inner);
        for cookie in headers.filter_map(|header| std::str::from_utf8(header.as_bytes()).ok()) {
            let _ = store.parse(cookie, url);
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let store = self.0.read().unwrap_or_else(PoisonError::into_inner);
        let header = store
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        if header.is_empty() {
            return None;
        }

        let mut header = HeaderValue::from_str(&header).ok()?;
        header.set_sensitive(true);
        Some(header)
    }
}

fn export_cookie(cookie: &Cookie<'static>) -> Option<SessionCookie> {
    let mut raw = (**cookie).clone();
    let host = match &cookie.domain {
        CookieDomain::HostOnly(host) => {
            raw.unset_domain();
            host
        }
        CookieDomain::Suffix(host) => {
            raw.set_domain(host.clone());
            host
        }
        _ => return None,
    };
    let path = if cookie.path.is_from_path_attr() {
        "/"
    } else {
        raw.unset_path();
        cookie.path.as_ref()
    };
    let origin_path = if path == "/" { "" } else { path };

    raw.set_max_age(None);
    match cookie.expires {
        CookieExpiration::AtUtc(expires) => raw.set_expires(expires),
        CookieExpiration::SessionEnd => raw.unset_expires(),
    }

    raw.set_secure(Some(cookie.secure().unwrap_or(false)));
    let partitioned_without_secure = raw.partitioned() == Some(true) && raw.secure() == Some(false);
    if partitioned_without_secure {
        raw.set_partitioned(None);
    }
    let mut set_cookie = raw.to_string();
    if partitioned_without_secure {
        set_cookie.push_str("; Partitioned");
    }

    Some(SessionCookie {
        origin: format!("https://{host}{origin_path}/"),
        set_cookie,
    })
}
