use std::env;

use csustkit::{ConnectionMode, SsoError, SsoHelper};

#[derive(Debug)]
enum ProbeError {
    MissingCredentials,
    CaptchaRequired(&'static str),
    Sso(SsoError),
}

impl From<SsoError> for ProbeError {
    fn from(error: SsoError) -> Self {
        Self::Sso(error)
    }
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials => write!(formatter, "未从环境变量读取到必要凭据"),
            Self::CaptchaRequired(mode) => {
                write!(formatter, "{mode} 登录需要验证码，按要求停止测试")
            }
            Self::Sso(error) => write!(formatter, "SSO 操作失败: {error}"),
        }
    }
}

impl std::error::Error for ProbeError {}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => println!("Direct 和 WebVPN 均登录并获取用户信息成功。"),
        Err(error) => {
            println!("测试停止: {error}");
            std::process::exit(1);
        }
    }
}

async fn run() -> Result<(), ProbeError> {
    let username = env::var("CSUST_AUTHSERVER_USERNAME")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(ProbeError::MissingCredentials)?;
    let password = env::var("CSUST_AUTHSERVER_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(ProbeError::MissingCredentials)?;

    test_mode(ConnectionMode::Direct, &username, &password).await?;
    test_mode(ConnectionMode::WebVpn, &username, &password).await?;
    Ok(())
}

async fn test_mode(mode: ConnectionMode, username: &str, password: &str) -> Result<(), ProbeError> {
    let mode_name = match mode {
        ConnectionMode::Direct => "Direct",
        ConnectionMode::WebVpn => "WebVPN",
    };
    println!("开始测试 {mode_name} ...");

    let helper = SsoHelper::new(mode)?;
    let login_form = helper.get_login_form().await?;
    if helper.check_need_captcha(username.to_owned()).await? {
        return Err(ProbeError::CaptchaRequired(mode_name));
    }

    helper
        .login(login_form, username.to_owned(), password.to_owned(), None)
        .await?;
    let profile = helper.get_login_user().await?;
    println!(
        "{mode_name} 登录成功: 姓名={}, 学号={}, 学院={}",
        profile.user_name, profile.user_account, profile.dept_name
    );
    Ok(())
}
