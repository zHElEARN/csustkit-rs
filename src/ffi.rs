use std::sync::Arc;

use url::Url;

use crate::{ConnectionMode, CsustSession, edu::EduHelper, sso::SsoHelper};

uniffi::custom_type!(Url, String, {
    remote,
    lower: |url| url.into(),
    try_lift: |value| Ok(Url::parse(&value)?),
});

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum SessionInitError {
    #[error("无法创建校园服务会话")]
    ClientBuild(#[from] reqwest::Error),
}

#[uniffi::export]
impl CsustSession {
    #[uniffi::constructor(name = "new")]
    pub fn new_shared() -> Result<Arc<Self>, SessionInitError> {
        Ok(Arc::new(Self::new()?))
    }
}

#[uniffi::export]
impl SsoHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}

#[uniffi::export]
impl EduHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}
