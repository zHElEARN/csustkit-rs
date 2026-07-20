use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use percent_encoding::percent_decode_str;
use reqwest::{
    Response,
    header::{LOCATION, SET_COOKIE},
};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::SystemTimeError};
use url::Url;

use crate::{
    connection::ConnectionMode,
    session::CsustSession,
    url_factory::{ServiceDomain, make_url},
};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

const LOGIN_PATH: &str = "/authserver/login?service=https%3A%2F%2Fehall.csust.edu.cn%2Flogin";
const CAMPUS_CARD_LOGIN_PATH: &str = "/berserker-auth/cas/login/wisedu?targetUrl=https://hxyxh5.csust.edu.cn/plat/?name=loginTransit";
const WEBVPN_ENCLIENT_URL: &str = "https://vpn.csust.edu.cn/enclient/";
const WEBVPN_CAS_CHECK_URL: &str =
    "https://vpn.csust.edu.cn/enclient/api/users/admin/custom/page/login/sso/cas";
const RANDOM_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz2345678";

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

#[derive(Deserialize)]
struct LoginUserResponse {
    data: Option<SsoProfile>,
}

#[derive(Deserialize)]
struct CheckNeedCaptchaResponse {
    #[serde(rename = "isNeed")]
    is_need: bool,
}

pub struct SsoHelper {
    mode: ConnectionMode,
    session: Arc<CsustSession>,
}

impl SsoHelper {
    pub fn new(mode: ConnectionMode) -> Result<Arc<Self>, SsoError> {
        let session =
            CsustSession::new().map_err(|error| SsoError::ClientBuildFailed(error.to_string()))?;
        Ok(Self::with_session(mode, session))
    }

    pub fn with_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self { mode, session })
    }

    pub async fn get_login_form(&self) -> Result<SsoLoginForm, SsoError> {
        let response = self
            .get_following_redirects(make_url(self.mode, ServiceDomain::AuthServer, LOGIN_PATH))
            .await?;

        let final_url = response.url().clone();
        if urls_equal(
            &final_url,
            &make_url(self.mode, ServiceDomain::Ehall, "/index.html"),
        ) {
            return Err(SsoError::GetLoginFormFailed("账号已登录".to_owned()));
        }

        let body = response
            .text()
            .await
            .map_err(|_| SsoError::GetLoginFormFailed("无响应数据".to_owned()))?;
        parse_login_form(&body)
    }

    pub async fn check_need_captcha(&self, username: String) -> Result<bool, SsoError> {
        let timestamp = current_timestamp_millis()?;
        let response = self
            .session
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::AuthServer,
                &format!("/authserver/checkNeedCaptcha.htl?username={username}&_={timestamp}"),
            ))
            .send()
            .await?
            .json::<CheckNeedCaptchaResponse>()
            .await?;

        Ok(response.is_need)
    }

    pub async fn get_captcha(&self) -> Result<Vec<u8>, SsoError> {
        let response = self
            .session
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::AuthServer,
                "/authserver/getCaptcha.htl",
            ))
            .send()
            .await?;
        let bytes = response.bytes().await?;

        if bytes.is_empty() {
            Err(SsoError::CaptchaRetrievalFailed)
        } else {
            Ok(bytes.to_vec())
        }
    }

    pub async fn login(
        &self,
        login_form: SsoLoginForm,
        username: String,
        password: String,
        captcha: Option<String>,
    ) -> Result<(), SsoError> {
        let encrypted_password =
            encrypt_password(&password, &login_form.pwd_encrypt_salt).unwrap_or(password);
        let captcha = captcha.unwrap_or_default();
        let params = [
            ("username", username.as_str()),
            ("password", encrypted_password.as_str()),
            ("captcha", captcha.as_str()),
            ("_eventId", "submit"),
            ("cllt", "userNameLogin"),
            ("dllt", "generalLogin"),
            ("lt", ""),
            ("execution", login_form.execution.as_str()),
        ];

        let response = self
            .session
            .client
            .post(make_url(self.mode, ServiceDomain::AuthServer, LOGIN_PATH))
            .form(&params)
            .send()
            .await?;

        let final_url = response.url().clone();
        let body = response.text().await.unwrap_or_default();

        let mut check_url = final_url.clone();
        if self.mode == ConnectionMode::WebVpn {
            if !urls_equal(&final_url, WEBVPN_ENCLIENT_URL) {
                return Err(login_failure_from_body(&body, final_url));
            }

            let check_response = self.session.client.get(WEBVPN_CAS_CHECK_URL).send().await?;
            check_url = check_response.url().clone();
        }

        let ehall_index = make_url(self.mode, ServiceDomain::Ehall, "/index.html");
        let ehall_default_index = make_url(self.mode, ServiceDomain::Ehall, "/default/index.html");
        if urls_equal(&check_url, &ehall_index) || urls_equal(&final_url, &ehall_default_index) {
            Ok(())
        } else {
            Err(login_failure_from_body(&body, final_url))
        }
    }

    pub async fn get_login_user(&self) -> Result<SsoProfile, SsoError> {
        let response = self
            .session
            .client
            .get(make_url(self.mode, ServiceDomain::Ehall, "/getLoginUser"))
            .send()
            .await?;

        let response = response.json::<LoginUserResponse>().await?;
        response.data.ok_or(SsoError::NotLoggedIn)
    }

    /// 从已登录的统一身份认证会话获取校园卡登录凭据。
    pub async fn login_to_campus_card(&self) -> Result<String, SsoError> {
        let response = self
            .session
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::CampusCard,
                CAMPUS_CARD_LOGIN_PATH,
            ))
            .send()
            .await?;
        let final_url = response.url();
        let expected_prefix = make_url(self.mode, ServiceDomain::CampusCard, "/plat");

        extract_campus_card_ticket(final_url, &expected_prefix)
    }

    pub async fn is_logged_in(&self) -> bool {
        self.get_login_user().await.is_ok()
    }

    pub async fn logout(&self) -> Result<(), SsoError> {
        self.session
            .client
            .get(make_url(self.mode, ServiceDomain::Ehall, "/logout"))
            .send()
            .await?
            .bytes()
            .await?;
        self.session
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::AuthServer,
                "/authserver/logout",
            ))
            .send()
            .await?
            .bytes()
            .await?;
        Ok(())
    }
}

impl SsoHelper {
    async fn get_following_redirects(&self, initial_url: String) -> Result<Response, SsoError> {
        let mut url = initial_url;
        for _ in 0..10 {
            let response = self
                .session
                .no_redirect_client
                .get(&url)
                .send()
                .await
                .map_err(|_| SsoError::GetLoginFormFailed("无响应数据".to_owned()))?;

            if !response.status().is_redirection() {
                return Ok(response);
            }

            for cookie in response.headers().get_all(SET_COOKIE) {
                if let Ok(cookie) = cookie.to_str() {
                    self.session
                        .cookie_jar
                        .add_cookie_str(cookie, response.url());
                }
            }

            let next_url = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|location| response.url().join(location).ok())
                .ok_or_else(|| SsoError::GetLoginFormFailed("无响应数据".to_owned()))?;
            url = next_url.to_string();
        }

        Err(SsoError::GetLoginFormFailed("无响应数据".to_owned()))
    }
}

fn parse_login_form(body: &str) -> Result<SsoLoginForm, SsoError> {
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

fn encrypt_password(password: &str, salt: &str) -> Result<String, ()> {
    if salt.is_empty() {
        return Ok(password.to_owned());
    }

    let prefix = random_string(64)?;
    let iv = random_string(16)?;
    encrypt_password_with_parts(password, salt, &prefix, &iv)
}

fn encrypt_password_with_parts(
    password: &str,
    salt: &str,
    prefix: &str,
    iv: &str,
) -> Result<String, ()> {
    let mut key = [0u8; 16];
    let salt_bytes = salt.as_bytes();
    if salt_bytes.len() < key.len() {
        return Err(());
    }
    key.copy_from_slice(&salt_bytes[..16]);

    let mut plain_text = prefix.to_owned();
    plain_text.push_str(password);

    let encrypted = Aes128CbcEnc::new_from_slices(&key, iv.as_bytes())
        .map_err(|_| ())?
        .encrypt_padded_vec::<Pkcs7>(plain_text.as_bytes());
    Ok(BASE64_STANDARD.encode(encrypted))
}

fn random_string(length: usize) -> Result<String, ()> {
    let mut bytes = vec![0u8; length];
    getrandom::fill(&mut bytes).map_err(|_| ())?;

    Ok(bytes
        .into_iter()
        .map(|byte| RANDOM_CHARS[usize::from(byte) % RANDOM_CHARS.len()] as char)
        .collect())
}

fn urls_equal(actual: &Url, expected: &str) -> bool {
    Url::parse(expected)
        .map(|expected_url| actual == &expected_url)
        .unwrap_or(false)
}

fn current_timestamp_millis() -> Result<u128, SsoError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(SsoError::Time)
}

fn login_failure_from_body(body: &str, final_url: Url) -> SsoError {
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

fn extract_campus_card_ticket(final_url: &Url, expected_prefix: &str) -> Result<String, SsoError> {
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
