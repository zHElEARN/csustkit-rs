use std::borrow::Cow;

use encoding_rs::{Encoding, UTF_8};
use reqwest::{
    RequestBuilder,
    header::{CONTENT_TYPE, HeaderMap},
};
use serde::de::DeserializeOwned;
use url::Url;

use crate::ConnectionMode;

const WEBVPN_LOGIN_MARKER: &str = "window.location.href = './start.html";

pub(crate) struct LoginCheck<E> {
    pub(crate) mode: ConnectionMode,
    pub(crate) fallback_encoding: &'static Encoding,
    pub(crate) subsystem_login: fn(&str, &Url) -> bool,
    pub(crate) not_logged_in: E,
}

pub(crate) struct RequestResponse {
    pub(crate) final_url: Url,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Vec<u8>,
}

impl RequestResponse {
    pub(crate) fn json<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_slice(&self.body)
    }

    pub(crate) fn text(&self) -> Cow<'_, str> {
        self.text_with_fallback(UTF_8)
    }

    fn text_with_fallback(&self, fallback: &'static Encoding) -> Cow<'_, str> {
        let encoding = self
            .headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|content_type| {
                content_type
                    .split(';')
                    .skip(1)
                    .filter_map(|parameter| parameter.split_once('='))
                    .find(|(name, _)| name.trim().eq_ignore_ascii_case("charset"))
            })
            .and_then(|(_, label)| Encoding::for_label(label.trim().trim_matches('"').as_bytes()))
            .unwrap_or(fallback);

        encoding.decode(&self.body).0
    }
}

/// Reads a response once and checks for login pages before callers decode it.
pub(crate) async fn perform_request<E>(
    request: RequestBuilder,
    login_check: Option<LoginCheck<E>>,
) -> Result<RequestResponse, E>
where
    E: From<reqwest::Error>,
{
    let response = request.send().await?;
    let response = RequestResponse {
        final_url: response.url().clone(),
        headers: response.headers().clone(),
        body: response.bytes().await?.to_vec(),
    };

    if let Some(check) = login_check {
        let text = response.text_with_fallback(check.fallback_encoding);
        if (check.mode == ConnectionMode::WebVpn && text.contains(WEBVPN_LOGIN_MARKER))
            || (check.subsystem_login)(&text, &response.final_url)
        {
            return Err(check.not_logged_in);
        }
    }

    Ok(response)
}
