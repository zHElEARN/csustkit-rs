#[path = "csustkit/campus_card.rs"]
mod campus_card;
#[path = "csustkit/console.rs"]
mod console;
#[path = "csustkit/edu.rs"]
mod edu;
#[path = "csustkit/formatting.rs"]
mod formatting;
#[path = "csustkit/login.rs"]
mod login;
#[path = "csustkit/mooc.rs"]
mod mooc;
#[path = "csustkit/webvpn.rs"]
mod webvpn;

use std::sync::Arc;

use csustkit::CsustSession;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let session = match CsustSession::new() {
        Ok(session) => session,
        Err(error) => {
            eprintln!("无法创建共享会话: {error}");
            std::process::exit(1);
        }
    };

    loop {
        println!("\n=== 入口菜单 ===");
        println!("1. 登录演示");
        println!("2. WebVPN 工具");
        println!("0. 退出");

        match console::prompt("请选择") {
            Some(choice) => match choice.as_str() {
                "1" => {
                    if let Some(login) = login::run_login_demo(Arc::clone(&session)).await {
                        login::run_main_menu(login).await;
                    }
                }
                "2" => webvpn::run_menu(),
                "0" => break,
                _ => println!("输入无效，请重新选择。"),
            },
            None => break,
        }
    }

    println!("程序已退出。");
}
