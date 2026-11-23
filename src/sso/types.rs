use serde::{Deserialize, Serialize};
use std::time::SystemTimeError;

use crate::SessionRequestError;

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error), uniffi(flat_error))]
pub enum SsoError {
    #[error("获取登录表单失败: {0}")]
    GetLoginFormFailed(String),
    #[error("验证码获取失败")]
    CaptchaRetrievalFailed,
    #[error("登录密码加密失败")]
    PasswordEncryptionFailed,
    #[error("登录失败: {0}")]
    LoginFailed(String),
    #[error("账号信息待完善，请前往网页端完善信息")]
    NeedImproveUserInfo,
    #[error("校园卡系统登录失败: {0}")]
    LoginToCampusCardFailed(String),
    #[error("网络课程中心登录失败: {0}")]
    LoginToMoocFailed(String),
    #[error("超星学习通登录失败: {0}")]
    LoginToChaoxingFailed(String),
    #[error("教务 SSO 请求失败: {0}")]
    SessionRequest(#[from] SessionRequestError),
    #[error("统一身份认证未登录")]
    NotLoggedIn,
    #[error("统一身份认证用户信息解析失败: {0}")]
    ProfileDecodeFailed(#[source] serde_json::Error),
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error("获取当前时间失败: {0}")]
    Time(#[source] SystemTimeError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct SsoLoginForm {
    pub pwd_encrypt_salt: String,
    pub execution: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
