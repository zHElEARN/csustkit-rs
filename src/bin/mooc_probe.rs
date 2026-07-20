use std::{env, sync::Arc};

use csustkit::{
    ConnectionMode, CsustSession,
    mooc::{MoocError, MoocHelper},
    sso::{SsoError, SsoHelper},
};

#[derive(Debug)]
enum ProbeError {
    MissingCredentials,
    CaptchaRequired(&'static str),
    Session(reqwest::Error),
    Sso(SsoError),
    Mooc(MoocError),
}

impl From<SsoError> for ProbeError {
    fn from(error: SsoError) -> Self {
        Self::Sso(error)
    }
}

impl From<MoocError> for ProbeError {
    fn from(error: MoocError) -> Self {
        Self::Mooc(error)
    }
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials => write!(formatter, "未从环境变量读取到必要凭据"),
            Self::CaptchaRequired(mode) => {
                write!(formatter, "{mode} 登录需要验证码，按要求停止测试")
            }
            Self::Session(error) => write!(formatter, "共享会话创建失败: {error}"),
            Self::Sso(error) => write!(formatter, "SSO 操作失败: {error}"),
            Self::Mooc(error) => write!(formatter, "MOOC 操作失败: {error}"),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Session(error) => Some(error),
            Self::Sso(error) => Some(error),
            Self::Mooc(error) => Some(error),
            _ => None,
        }
    }
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => println!("Direct 和 WebVPN 的 MOOC 只读链路均验证成功。"),
        Err(error) => {
            println!("测试停止: {error}");
            std::process::exit(1);
        }
    }
}

async fn run() -> Result<(), ProbeError> {
    let username = required_environment("CSUST_AUTHSERVER_USERNAME")?;
    let password = required_environment("CSUST_AUTHSERVER_PASSWORD")?;

    test_mode(ConnectionMode::Direct, &username, &password).await?;
    test_mode(ConnectionMode::WebVpn, &username, &password).await?;
    Ok(())
}

fn required_environment(name: &str) -> Result<String, ProbeError> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(ProbeError::MissingCredentials)
}

async fn test_mode(mode: ConnectionMode, username: &str, password: &str) -> Result<(), ProbeError> {
    let mode_name = match mode {
        ConnectionMode::Direct => "Direct",
        ConnectionMode::WebVpn => "WebVPN",
    };
    println!("开始测试 {mode_name} MOOC 链路 ...");

    let session = CsustSession::new().map_err(ProbeError::Session)?;
    let sso = SsoHelper::with_session(mode, Arc::clone(&session));
    let mooc = MoocHelper::with_session(mode, session);

    let login_form = sso.get_login_form().await?;
    if sso.check_need_captcha(username.to_owned()).await? {
        return Err(ProbeError::CaptchaRequired(mode_name));
    }
    sso.login(login_form, username.to_owned(), password.to_owned(), None)
        .await?;
    sso.login_to_mooc().await?;

    let operation_result = async {
        let profile = mooc.get_profile().await?;
        let courses = mooc.get_courses().await?;
        let pending = mooc.get_courses_with_pending_assignments().await?;
        println!(
            "{mode_name} MOOC 成功: 姓名={}，课程={}，待提交作业课程={}",
            profile.name,
            courses.len(),
            pending.len()
        );
        if let Some(course) = courses.first() {
            let assignments = mooc.get_course_assignments(course).await?;
            let exams = mooc.get_course_exams(course).await?;
            println!(
                "{mode_name} 首门课程={}：作业={}，测验={}",
                course.name,
                assignments.len(),
                exams.len()
            );
        }
        Ok::<(), ProbeError>(())
    }
    .await;

    let mooc_logout = mooc.logout().await.map_err(ProbeError::from);
    let sso_logout = sso.logout().await.map_err(ProbeError::from);
    operation_result?;
    mooc_logout?;
    sso_logout?;
    Ok(())
}
