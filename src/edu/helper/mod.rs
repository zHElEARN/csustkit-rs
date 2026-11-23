use std::time::{SystemTime, UNIX_EPOCH};

use encoding_rs::UTF_8;
use reqwest::RequestBuilder;
use scraper::ElementRef;

use crate::{
    connection::ConnectionMode,
    request::{LoginCheck, RequestResponse, send_with_retry},
    session::CsustSession,
    url_factory::{ServiceDomain, make_url},
};

use super::types::EduError;

mod classrooms;
mod exams;
mod grades;
mod profile;
mod schedule;
mod semester;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct EduHelper {
    mode: ConnectionMode,
    session: CsustSession,
}

impl EduHelper {
    pub fn new(mode: ConnectionMode, session: CsustSession) -> Self {
        Self { mode, session }
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export(async_runtime = "tokio"))]
impl EduHelper {
    pub async fn is_logged_in(&self) -> bool {
        self.get_profile().await.is_ok()
    }

    pub async fn logout(&self) -> Result<(), EduError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(EduError::Time)?
            .as_millis();
        let url = make_url(
            self.mode,
            ServiceDomain::Education,
            &format!("/jsxsd/xk/LoginToXk?method=exit&tktime={timestamp}"),
        );
        self.session
            .client
            .get(url)
            .send()
            .await
            .map_err(EduError::LogoutFailed)?
            .bytes()
            .await
            .map_err(EduError::LogoutFailed)?;
        Ok(())
    }
}

impl EduHelper {
    async fn request(
        &self,
        build_request: impl Fn() -> RequestBuilder,
    ) -> Result<RequestResponse, EduError> {
        send_with_retry(build_request, || LoginCheck {
            mode: self.mode,
            fallback_encoding: UTF_8,
            subsystem_login: |body, _| body.contains("请输入账号"),
            not_logged_in: EduError::NotLoggedIn,
        })
        .await
    }
}

fn element_text(element: ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_owned()
}
