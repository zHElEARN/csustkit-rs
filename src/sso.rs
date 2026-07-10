use std::sync::Arc;

use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use reqwest::{
    Client, Response,
    cookie::Jar,
    header::{LOCATION, SET_COOKIE},
    redirect::Policy,
};
use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

use crate::{
    connection::ConnectionMode,
    url_factory::{ServiceDomain, make_url},
};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

const LOGIN_PATH: &str = "/authserver/login?service=https%3A%2F%2Fehall.csust.edu.cn%2Flogin";
const WEBVPN_ENCLIENT_URL: &str = "https://vpn.csust.edu.cn/enclient/";
const WEBVPN_CAS_CHECK_URL: &str =
    "https://vpn.csust.edu.cn/enclient/api/users/admin/custom/page/login/sso/cas";
const RANDOM_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz2345678";
// The WebVPN gateway returns HTTP 500 when the User-Agent header is absent.
const SSO_USER_AGENT: &str = concat!("csustkit-rs/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum SsoError {
    #[error("SSO client build failed")]
    ClientBuildFailed,
    #[error("login form retrieval failed")]
    GetLoginFormFailed,
    #[error("captcha check failed")]
    CaptchaCheckFailed,
    #[error("captcha retrieval failed")]
    CaptchaRetrievalFailed,
    #[error("password encryption failed")]
    PasswordEncryptionFailed,
    #[error("login failed")]
    LoginFailed,
    #[error("profile retrieval failed")]
    ProfileRetrievalFailed,
    #[error("SSO session is not logged in")]
    NotLoggedIn,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SsoLoginForm {
    pub pwd_encrypt_salt: String,
    pub execution: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, uniffi::Record)]
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

#[derive(Deserialize)]
struct LoginUserResponse {
    data: Option<SsoProfile>,
}

#[derive(Deserialize)]
struct CheckNeedCaptchaResponse {
    #[serde(rename = "isNeed")]
    is_need: bool,
}

#[derive(uniffi::Object)]
pub struct SsoHelper {
    mode: ConnectionMode,
    client: Client,
    no_redirect_client: Client,
    cookie_jar: Arc<Jar>,
}

#[uniffi::export]
impl SsoHelper {
    #[uniffi::constructor]
    pub fn new(mode: ConnectionMode) -> Result<Arc<Self>, SsoError> {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_jar))
            .redirect(Policy::limited(10))
            .user_agent(SSO_USER_AGENT)
            .build()
            .map_err(|_| SsoError::ClientBuildFailed)?;
        let no_redirect_client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_jar))
            .redirect(Policy::none())
            .user_agent(SSO_USER_AGENT)
            .build()
            .map_err(|_| SsoError::ClientBuildFailed)?;

        Ok(Arc::new(Self {
            mode,
            client,
            no_redirect_client,
            cookie_jar,
        }))
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
            return Err(SsoError::GetLoginFormFailed);
        }

        let body = response
            .text()
            .await
            .map_err(|_| SsoError::GetLoginFormFailed)?;
        parse_login_form(&body)
    }

    pub async fn check_need_captcha(&self, username: String) -> Result<bool, SsoError> {
        let timestamp = current_timestamp_millis()?;
        let response = self
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::AuthServer,
                &format!("/authserver/checkNeedCaptcha.htl?username={username}&_={timestamp}"),
            ))
            .send()
            .await
            .map_err(|_| SsoError::CaptchaCheckFailed)?
            .json::<CheckNeedCaptchaResponse>()
            .await
            .map_err(|_| SsoError::CaptchaCheckFailed)?;

        Ok(response.is_need)
    }

    pub async fn get_captcha(&self) -> Result<Vec<u8>, SsoError> {
        let response = self
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::AuthServer,
                "/authserver/getCaptcha.htl",
            ))
            .send()
            .await
            .map_err(|_| SsoError::CaptchaRetrievalFailed)?;
        let bytes = response
            .bytes()
            .await
            .map_err(|_| SsoError::CaptchaRetrievalFailed)?;

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
            .client
            .post(make_url(self.mode, ServiceDomain::AuthServer, LOGIN_PATH))
            .form(&params)
            .send()
            .await
            .map_err(|_| SsoError::LoginFailed)?;

        let final_url = response.url().clone();

        let mut check_url = final_url.clone();
        if self.mode == ConnectionMode::WebVpn {
            if !urls_equal(&final_url, WEBVPN_ENCLIENT_URL) {
                return Err(SsoError::LoginFailed);
            }

            let check_response = self
                .client
                .get(WEBVPN_CAS_CHECK_URL)
                .send()
                .await
                .map_err(|_| SsoError::LoginFailed)?;
            check_url = check_response.url().clone();
        }

        let ehall_index = make_url(self.mode, ServiceDomain::Ehall, "/index.html");
        let ehall_default_index = make_url(self.mode, ServiceDomain::Ehall, "/default/index.html");
        if urls_equal(&check_url, &ehall_index) || urls_equal(&final_url, &ehall_default_index) {
            Ok(())
        } else {
            Err(SsoError::LoginFailed)
        }
    }

    pub async fn get_login_user(&self) -> Result<SsoProfile, SsoError> {
        let response = self
            .client
            .get(make_url(self.mode, ServiceDomain::Ehall, "/getLoginUser"))
            .send()
            .await
            .map_err(|_| SsoError::ProfileRetrievalFailed)?;

        let response = response
            .json::<LoginUserResponse>()
            .await
            .map_err(|_| SsoError::ProfileRetrievalFailed)?;
        response.data.ok_or(SsoError::NotLoggedIn)
    }

    pub async fn is_logged_in(&self) -> bool {
        self.get_login_user().await.is_ok()
    }
}

impl SsoHelper {
    async fn get_following_redirects(&self, initial_url: String) -> Result<Response, SsoError> {
        let mut url = initial_url;
        for _ in 0..10 {
            let response = self
                .no_redirect_client
                .get(&url)
                .send()
                .await
                .map_err(|_| SsoError::GetLoginFormFailed)?;

            if !response.status().is_redirection() {
                return Ok(response);
            }

            for cookie in response.headers().get_all(SET_COOKIE) {
                if let Ok(cookie) = cookie.to_str() {
                    self.cookie_jar.add_cookie_str(cookie, response.url());
                }
            }

            let next_url = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|location| response.url().join(location).ok())
                .ok_or(SsoError::GetLoginFormFailed)?;
            url = next_url.to_string();
        }

        Err(SsoError::GetLoginFormFailed)
    }
}

fn parse_login_form(body: &str) -> Result<SsoLoginForm, SsoError> {
    let document = Html::parse_document(body);
    let salt_selector =
        Selector::parse("input#pwdEncryptSalt").map_err(|_| SsoError::GetLoginFormFailed)?;
    let execution_selector =
        Selector::parse("input#execution").map_err(|_| SsoError::GetLoginFormFailed)?;

    let pwd_encrypt_salt = document
        .select(&salt_selector)
        .next()
        .and_then(|element| element.value().attr("value"))
        .ok_or(SsoError::GetLoginFormFailed)?
        .to_owned();
    let execution = document
        .select(&execution_selector)
        .next()
        .and_then(|element| element.value().attr("value"))
        .ok_or(SsoError::GetLoginFormFailed)?
        .to_owned();

    Ok(SsoLoginForm {
        pwd_encrypt_salt,
        execution,
    })
}

fn encrypt_password(password: &str, salt: &str) -> Result<String, SsoError> {
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
) -> Result<String, SsoError> {
    let mut key = [0u8; 16];
    let salt_bytes = salt.as_bytes();
    if salt_bytes.len() < key.len() {
        return Err(SsoError::PasswordEncryptionFailed);
    }
    key.copy_from_slice(&salt_bytes[..16]);

    let mut plain_text = prefix.to_owned();
    plain_text.push_str(password);

    let encrypted = Aes128CbcEnc::new_from_slices(&key, iv.as_bytes())
        .map_err(|_| SsoError::PasswordEncryptionFailed)?
        .encrypt_padded_vec::<Pkcs7>(plain_text.as_bytes());
    Ok(BASE64_STANDARD.encode(encrypted))
}

fn random_string(length: usize) -> Result<String, SsoError> {
    let mut bytes = vec![0u8; length];
    getrandom::fill(&mut bytes).map_err(|_| SsoError::PasswordEncryptionFailed)?;

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
        .map_err(|_| SsoError::CaptchaCheckFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockModeDecrypt;

    type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

    #[test]
    fn parses_login_form_html() {
        let form = parse_login_form(
            r#"
            <html>
              <input id="pwdEncryptSalt" value="1234567890abcdef" />
              <input id="execution" value="e1s1" />
            </html>
            "#,
        )
        .unwrap();

        assert_eq!(
            form,
            SsoLoginForm {
                pwd_encrypt_salt: "1234567890abcdef".to_owned(),
                execution: "e1s1".to_owned(),
            }
        );
    }

    #[test]
    fn login_form_parse_requires_fields() {
        assert_eq!(
            parse_login_form("<html></html>").unwrap_err(),
            SsoError::GetLoginFormFailed
        );
    }

    #[test]
    fn encrypt_password_uses_aes_cbc_pkcs7_and_random_prefix() {
        use base64::Engine as _;

        let prefix = "A".repeat(64);
        let iv = "B".repeat(16);
        let encrypted =
            encrypt_password_with_parts("password", "1234567890abcdef", &prefix, &iv).unwrap();
        let encrypted_bytes = BASE64_STANDARD.decode(encrypted).unwrap();
        let decrypted = Aes128CbcDec::new_from_slices(b"1234567890abcdef", iv.as_bytes())
            .unwrap()
            .decrypt_padded_vec::<Pkcs7>(&encrypted_bytes)
            .unwrap();
        let decrypted = String::from_utf8(decrypted).unwrap();

        assert_eq!(decrypted, format!("{prefix}password"));
    }

    #[test]
    fn empty_salt_leaves_password_unchanged() {
        assert_eq!(encrypt_password("password", "").unwrap(), "password");
    }
}
