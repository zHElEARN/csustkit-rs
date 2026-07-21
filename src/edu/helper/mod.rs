use std::time::{SystemTime, UNIX_EPOCH};

use scraper::ElementRef;

use crate::{
    connection::ConnectionMode,
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

pub struct EduHelper {
    mode: ConnectionMode,
    session: CsustSession,
}

impl EduHelper {
    pub fn new(mode: ConnectionMode) -> Result<Self, EduError> {
        let session =
            CsustSession::new().map_err(|error| EduError::ClientBuildFailed(error.to_string()))?;
        Ok(Self::with_session(mode, session))
    }

    pub fn with_session(mode: ConnectionMode, session: CsustSession) -> Self {
        Self { mode, session }
    }

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

fn ensure_logged_in(body: &str) -> Result<(), EduError> {
    if body.contains("请输入账号") {
        Err(EduError::NotLoggedIn)
    } else {
        Ok(())
    }
}

fn element_text(element: ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_owned()
}
