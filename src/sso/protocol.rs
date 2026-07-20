use percent_encoding::percent_decode_str;
use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

use super::types::{SsoError, SsoLoginForm, SsoProfile};

#[derive(Deserialize)]
pub(super) struct LoginUserResponse {
    pub(super) data: Option<SsoProfile>,
}

#[derive(Deserialize)]
pub(super) struct CheckNeedCaptchaResponse {
    #[serde(rename = "isNeed")]
    pub(super) is_need: bool,
}

pub(super) fn parse_login_form(body: &str) -> Result<SsoLoginForm, SsoError> {
    let document = Html::parse_document(body);
    let salt_selector = Selector::parse("input#pwdEncryptSalt")
        .map_err(|_| SsoError::GetLoginFormFailed("未找到pwdEncryptSalt输入框".to_owned()))?;
    let execution_selector = Selector::parse("input#execution")
        .map_err(|_| SsoError::GetLoginFormFailed("未找到execution输入框".to_owned()))?;

    let pwd_encrypt_salt = document
        .select(&salt_selector)
        .next()
        .and_then(|element| element.value().attr("value"))
        .ok_or_else(|| SsoError::GetLoginFormFailed("未找到pwdEncryptSalt输入框".to_owned()))?
        .to_owned();
    let execution = document
        .select(&execution_selector)
        .next()
        .and_then(|element| element.value().attr("value"))
        .ok_or_else(|| SsoError::GetLoginFormFailed("未找到execution输入框".to_owned()))?
        .to_owned();

    Ok(SsoLoginForm {
        pwd_encrypt_salt,
        execution,
    })
}

pub(super) fn login_failure_from_body(body: &str, final_url: Url) -> SsoError {
    if let Ok(selector) = Selector::parse("#showErrorTip") {
        let document = Html::parse_document(body);
        if let Some(error_element) = document.select(&selector).next() {
            let message = error_element.text().collect::<String>();
            if !message.is_empty() {
                return SsoError::LoginFailed(format!("登录失败: {message}"));
            }
        }
    }
    SsoError::LoginFailed(format!("登录失败: {final_url}"))
}

pub(super) fn extract_campus_card_ticket(
    final_url: &Url,
    expected_prefix: &str,
) -> Result<String, SsoError> {
    if !final_url.as_str().starts_with(expected_prefix) {
        return Err(SsoError::LoginToCampusCardFailed(format!(
            "重定向URL异常: {final_url}"
        )));
    }

    let encoded_ticket = final_url
        .query_pairs()
        .find_map(|(name, value)| (name == "ticket").then_some(value.into_owned()))
        .ok_or_else(|| SsoError::LoginToCampusCardFailed("无法获取登录凭据".to_owned()))?;

    percent_decode_str(&encoded_ticket)
        .decode_utf8()
        .map(|value| value.into_owned())
        .map_err(|_| SsoError::LoginToCampusCardFailed("无法获取登录凭据".to_owned()))
}
