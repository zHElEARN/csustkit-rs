use std::sync::Arc;

use reqwest::{
    Response,
    header::{LOCATION, SET_COOKIE},
};
use url::Url;

use crate::{
    connection::ConnectionMode,
    session::CsustSession,
    url_factory::{ServiceDomain, make_url},
};

use super::{
    crypto::encrypt_password,
    protocol::{
        CheckNeedCaptchaResponse, LoginUserResponse, extract_campus_card_ticket,
        login_failure_from_body, parse_login_form,
    },
    types::{SsoError, SsoLoginForm, SsoProfile},
};

const LOGIN_PATH: &str = "/authserver/login?service=https%3A%2F%2Fehall.csust.edu.cn%2Flogin";
const CAMPUS_CARD_LOGIN_PATH: &str = "/berserker-auth/cas/login/wisedu?targetUrl=https://hxyxh5.csust.edu.cn/plat/?name=loginTransit";
const WEBVPN_ENCLIENT_URL: &str = "https://vpn.csust.edu.cn/enclient/";
const WEBVPN_CAS_CHECK_URL: &str =
    "https://vpn.csust.edu.cn/enclient/api/users/admin/custom/page/login/sso/cas";

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
