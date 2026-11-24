mod auth;
mod console;
mod services;
mod webvpn;

use std::process::ExitCode;

use csustkit::{
    ConnectionMode, CsustSession, campus_card::CampusCardHelper, chaoxing::ChaoxingHelper,
    edu::EduHelper, mooc::MoocHelper, physics_experiment::PhysicsExperimentHelper, sso::SsoHelper,
};

use console::Console;
use services::{campus_card, chaoxing, edu, mooc, physics_experiment};

#[derive(Clone, Copy)]
enum Entry {
    CampusServices,
    PhysicsExperiment,
    WebVpnLinks,
}

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();

    let mut console = Console::default();
    while let Some(entry) = console.menu(
        "CSUSTKit",
        &[
            (Entry::CampusServices, "校园服务（统一认证）"),
            (Entry::PhysicsExperiment, "大物实验（校园网直连）"),
            (Entry::WebVpnLinks, "WebVPN 链接转换"),
        ],
        "退出",
    ) {
        match entry {
            Entry::CampusServices => run_campus_services(&mut console).await,
            Entry::PhysicsExperiment => {
                if let Some(session) = new_session() {
                    let helper = PhysicsExperimentHelper::new(ConnectionMode::Direct, session);
                    physics_experiment::run_menu(&mut console, &helper).await;
                }
            }
            Entry::WebVpnLinks => webvpn::run_menu(&mut console),
        }
    }

    println!("程序已退出。");
    console.exit_code()
}

#[derive(Clone, Copy)]
enum Service {
    Mooc,
    Education,
    CampusCard,
    Chaoxing,
    PhysicsExperiment,
    Profile,
}

async fn run_campus_services(console: &mut Console) {
    let Some(mode) = console.menu(
        "选择网络模式",
        &[
            (ConnectionMode::Direct, "校园网直连"),
            (ConnectionMode::WebVpn, "WebVPN"),
        ],
        "返回",
    ) else {
        return;
    };
    let Some(session) = new_session() else {
        return;
    };

    let sso = SsoHelper::new(mode, session.clone());
    if !auth::login(console, &sso, &session).await {
        return;
    }

    let title = match mode {
        ConnectionMode::Direct => "校园服务（校园网直连）",
        ConnectionMode::WebVpn => "校园服务（WebVPN）",
    };
    while let Some(service) = console.menu(
        title,
        &[
            (Service::Mooc, "网络课程中心"),
            (Service::Education, "教务系统"),
            (Service::CampusCard, "校园卡"),
            (Service::Chaoxing, "超星学习通"),
            (Service::PhysicsExperiment, "大物实验（独立账号）"),
            (Service::Profile, "统一认证个人资料"),
        ],
        "结束本次会话",
    ) {
        // Clones share the cookie jar for this login.
        match service {
            Service::Mooc => {
                let helper = MoocHelper::new(mode, session.clone());
                mooc::run_menu(console, &helper, &sso).await;
            }
            Service::Education => {
                let helper = EduHelper::new(mode, session.clone());
                edu::run_menu(console, &helper, &sso).await;
            }
            Service::CampusCard => {
                let helper = CampusCardHelper::new(mode, session.clone());
                campus_card::run_menu(console, &helper, &sso).await;
            }
            Service::Chaoxing => {
                let helper = ChaoxingHelper::new(mode, session.clone());
                chaoxing::run_menu(console, &helper, &sso).await;
            }
            Service::PhysicsExperiment => {
                let helper = PhysicsExperimentHelper::new(mode, session.clone());
                physics_experiment::run_menu(console, &helper).await;
            }
            Service::Profile => auth::profile(&sso).await,
        }
    }
}

fn new_session() -> Option<CsustSession> {
    match CsustSession::new() {
        Ok(session) => Some(session),
        Err(error) => {
            eprintln!("无法创建会话：{error}");
            None
        }
    }
}
