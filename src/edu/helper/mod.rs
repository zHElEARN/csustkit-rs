use std::sync::Arc;

use scraper::ElementRef;

use crate::{connection::ConnectionMode, session::CsustSession};

use super::types::EduError;

mod exams;
mod grades;
mod profile;
mod schedule;

pub struct EduHelper {
    mode: ConnectionMode,
    session: Arc<CsustSession>,
}

impl EduHelper {
    pub fn new(mode: ConnectionMode) -> Result<Arc<Self>, EduError> {
        let session =
            CsustSession::new().map_err(|error| EduError::ClientBuildFailed(error.to_string()))?;
        Ok(Self::with_session(mode, session))
    }

    pub fn with_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self { mode, session })
    }

    pub async fn is_logged_in(&self) -> bool {
        self.get_profile().await.is_ok()
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
