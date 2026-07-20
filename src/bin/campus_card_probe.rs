use std::{env, sync::Arc};

use csustkit::{
    ConnectionMode, CsustSession,
    campus_card::{Campus, CampusCardError, CampusCardHelper},
    sso::{SsoError, SsoHelper},
};

#[derive(Debug)]
enum ProbeError {
    MissingCredentials,
    CaptchaRequired(&'static str),
    Session(reqwest::Error),
    Sso(SsoError),
    CampusCard(CampusCardError),
    EmptyBuildings {
        mode: &'static str,
        campus: &'static str,
    },
    EmptyRooms {
        mode: &'static str,
        campus: &'static str,
    },
    ElectricityOutOfRange {
        mode: &'static str,
        campus: &'static str,
        value: f64,
    },
}

impl From<SsoError> for ProbeError {
    fn from(error: SsoError) -> Self {
        Self::Sso(error)
    }
}

impl From<CampusCardError> for ProbeError {
    fn from(error: CampusCardError) -> Self {
        Self::CampusCard(error)
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
            Self::CampusCard(error) => write!(formatter, "校园卡操作失败: {error}"),
            Self::EmptyBuildings { mode, campus } => {
                write!(formatter, "{mode} {campus}楼栋列表为空")
            }
            Self::EmptyRooms { mode, campus } => {
                write!(formatter, "{mode} {campus}宿舍列表为空")
            }
            Self::ElectricityOutOfRange {
                mode,
                campus,
                value,
            } => write!(formatter, "{mode} {campus}电量数值异常: {value}"),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Session(error) => Some(error),
            Self::Sso(error) => Some(error),
            Self::CampusCard(error) => Some(error),
            _ => None,
        }
    }
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => println!("Direct 和 WebVPN 校园卡完整只读链路均验证成功。"),
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
    println!("开始测试 {mode_name} ...");

    let session = CsustSession::new().map_err(ProbeError::Session)?;
    let sso = SsoHelper::with_session(mode, Arc::clone(&session));
    let campus_card = CampusCardHelper::with_session(mode, session);

    let login_form = sso.get_login_form().await?;
    if sso.check_need_captcha(username.to_owned()).await? {
        return Err(ProbeError::CaptchaRequired(mode_name));
    }
    sso.login(login_form, username.to_owned(), password.to_owned(), None)
        .await?;

    let operation_result = async {
        let ticket = sso.login_to_campus_card().await?;
        campus_card.sync_token(ticket).await?;
        campus_card.get_profile().await?;

        for campus in Campus::ALL {
            let buildings = campus_card.get_buildings(campus).await?;
            let building = buildings.first().ok_or(ProbeError::EmptyBuildings {
                mode: mode_name,
                campus: campus.display_name(),
            })?;
            let rooms = campus_card.get_rooms(building).await?;
            let room = rooms.first().ok_or(ProbeError::EmptyRooms {
                mode: mode_name,
                campus: campus.display_name(),
            })?;
            let electricity = campus_card.get_electricity(room).await?;
            if !electricity.is_finite() || !(-1.0..10_000.0).contains(&electricity) {
                return Err(ProbeError::ElectricityOutOfRange {
                    mode: mode_name,
                    campus: campus.display_name(),
                    value: electricity,
                });
            }

            println!(
                "{mode_name} {}: 楼栋={}，首栋宿舍={}，电量={electricity}",
                campus.display_name(),
                buildings.len(),
                rooms.len(),
            );
        }

        Ok::<(), ProbeError>(())
    }
    .await;

    let campus_logout_result = if campus_card.token().is_some() {
        campus_card.logout().await.map_err(ProbeError::from)
    } else {
        Ok(())
    };
    let sso_logout_result = sso.logout().await.map_err(ProbeError::from);

    operation_result?;
    campus_logout_result?;
    sso_logout_result?;
    Ok(())
}
