use serde::{Deserialize, Serialize};
use std::time::SystemTimeError;

#[derive(Debug, thiserror::Error)]
pub enum SsoError {
    #[error("SSO 客户端创建失败: {0}")]
    ClientBuildFailed(String),
    #[error("获取登录表单失败: {0}")]
    GetLoginFormFailed(String),
    #[error("验证码获取失败")]
    CaptchaRetrievalFailed,
    #[error("登录失败: {0}")]
    LoginFailed(String),
    #[error("校园卡系统登录失败: {0}")]
    LoginToCampusCardFailed(String),
    #[error("统一身份认证未登录")]
    NotLoggedIn,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error("获取当前时间失败: {0}")]
    Time(#[source] SystemTimeError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SsoLoginForm {
    pub pwd_encrypt_salt: String,
    pub execution: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoProfile {
    pub category_name: String,
    pub user_account: String,
    pub user_name: String,
    pub cert_code: String,
    pub phone: String,
    pub email: Option<String>,
    pub dept_name: String,
    pub default_user_avatar: String,
    pub head_image_icon: Option<String>,
}

impl SsoProfile {
    pub fn avatar_url(&self) -> &str {
        self.head_image_icon
            .as_deref()
            .unwrap_or(&self.default_user_avatar)
    }
}
