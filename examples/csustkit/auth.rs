use std::{error::Error, fs};

use csustkit::{CsustSession, sso::SsoHelper};

use super::console::Console;

pub async fn login(console: &mut Console, sso: &SsoHelper, session: &CsustSession) -> bool {
    println!("\n=== 统一认证登录 ===");
    loop {
        match login_once(console, sso).await {
            Ok(true) => {
                println!("统一认证登录成功。");
                return true;
            }
            Ok(false) => return false,
            Err(error) => eprintln!("统一认证登录失败：{error}"),
        }

        if console
            .menu("统一认证登录", &[((), "重试登录")], "返回")
            .is_none()
        {
            return false;
        }
        session.clear_cookies();
    }
}

async fn login_once(console: &mut Console, sso: &SsoHelper) -> Result<bool, Box<dyn Error>> {
    let Some(username) = console.username("CSUST_AUTHSERVER_USERNAME", "请输入统一认证用户名")
    else {
        return Ok(false);
    };
    let Some(password) = console.password("CSUST_AUTHSERVER_PASSWORD", "请输入统一认证密码")
    else {
        return Ok(false);
    };

    let login_form = sso.get_login_form().await?;
    let captcha = if sso.check_need_captcha(username.clone()).await? {
        fs::write("captcha.jpg", sso.get_captcha().await?)?;
        println!("验证码已保存到当前目录的 captcha.jpg。");
        let Some(captcha) = console.text("请输入验证码") else {
            return Ok(false);
        };
        Some(captcha)
    } else {
        None
    };
    sso.login(login_form, username, password, captcha).await?;
    Ok(true)
}

pub async fn profile(sso: &SsoHelper) {
    match sso.get_profile().await {
        Ok(user) => {
            println!("姓名：{}", user.user_name);
            println!("账号：{}", user.user_account);
            println!("院系：{}", user.dept_name);
        }
        Err(error) => eprintln!("获取统一认证个人资料失败：{error}"),
    }
}
