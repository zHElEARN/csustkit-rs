use std::{env, fs, sync::Arc};

use csustkit::{
    ConnectionMode, CsustSession, campus_card::CampusCardHelper, edu::EduHelper, mooc::MoocHelper,
    sso::SsoHelper,
};

use super::{campus_card, console, edu, mooc};

pub struct LoginContext {
    pub mode: ConnectionMode,
    pub session: Arc<CsustSession>,
    pub sso: Arc<SsoHelper>,
}

pub async fn run_login_demo(session: Arc<CsustSession>) -> Option<LoginContext> {
    let mode = select_connection_mode()?;
    let sso = SsoHelper::with_session(mode, Arc::clone(&session));

    loop {
        println!("\n=== 统一认证登录 ===");
        let result = login_once(&sso).await;
        match result {
            Ok(()) => {
                match sso.get_login_user().await {
                    Ok(user) => {
                        println!("\n登录成功");
                        println!("姓名: {}", user.user_name);
                        println!("学号: {}", user.user_account);
                        println!("学院: {}", user.dept_name);
                    }
                    Err(error) => println!("登录成功，但获取用户信息失败: {error}"),
                }
                return Some(LoginContext { mode, session, sso });
            }
            Err(error) => {
                println!("\n登录失败: {error}");
                println!("1. 重试登录");
                println!("0. 返回入口菜单");
                if console::prompt("请选择").as_deref() != Some("1") {
                    println!("已返回入口菜单。");
                    return None;
                }
            }
        }
    }
}

fn select_connection_mode() -> Option<ConnectionMode> {
    loop {
        println!("\n=== 网络模式 ===");
        println!("1. 直接连接");
        println!("2. WebVPN");
        match console::prompt("请选择网络模式")?.as_str() {
            "1" => return Some(ConnectionMode::Direct),
            "2" => return Some(ConnectionMode::WebVpn),
            _ => println!("输入无效，请输入 1 或 2。"),
        }
    }
}

async fn login_once(sso: &SsoHelper) -> Result<(), Box<dyn std::error::Error>> {
    let login_form = sso.get_login_form().await?;
    let username = match env::var("CSUST_AUTHSERVER_USERNAME") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => console::prompt_non_empty("请输入用户名").ok_or("输入已结束")?,
    };
    let captcha = if sso.check_need_captcha(username.clone()).await? {
        let bytes = sso.get_captcha().await?;
        fs::write("captcha.jpg", bytes)?;
        println!("验证码已保存到当前目录的 captcha.jpg");
        Some(console::prompt_non_empty("请输入验证码").ok_or("输入已结束")?)
    } else {
        None
    };
    let password = match env::var("CSUST_AUTHSERVER_PASSWORD") {
        Ok(value) if !value.is_empty() => value,
        _ => console::prompt_password("请输入密码").ok_or("输入已结束")?,
    };
    sso.login(login_form, username, password, captcha).await?;
    Ok(())
}

pub async fn run_main_menu(login: LoginContext) {
    let mooc_helper = MoocHelper::with_session(login.mode, Arc::clone(&login.session));
    let edu_helper = EduHelper::with_session(login.mode, Arc::clone(&login.session));
    let campus_card_helper = CampusCardHelper::with_session(login.mode, Arc::clone(&login.session));
    loop {
        println!("\n=== 主菜单 ===");
        println!("1. 网络课程中心");
        println!("2. 教务系统");
        println!("3. 校园卡系统");
        println!("0. 返回入口菜单");
        let Some(choice) = console::prompt("请选择") else {
            return;
        };
        match choice.as_str() {
            "1" => mooc::run_menu(Arc::clone(&mooc_helper), Arc::clone(&login.sso)).await,
            "2" => edu::run_menu(Arc::clone(&edu_helper), Arc::clone(&login.sso)).await,
            "3" => {
                campus_card::run_menu(Arc::clone(&campus_card_helper), Arc::clone(&login.sso)).await
            }
            "0" => return,
            _ => println!("输入无效，请重新选择。"),
        }
    }
}
