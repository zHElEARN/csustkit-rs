use csustkit::{
    chaoxing::{ChaoxingError, ChaoxingHelper},
    sso::SsoHelper,
};

use super::{Console, display_date};

const SCHOOL_OFFSET: chrono::FixedOffset = chrono::FixedOffset::east_opt(8 * 60 * 60).unwrap();

#[derive(Clone, Copy)]
enum Action {
    Profile,
    Assignments,
}

pub async fn run_menu(console: &mut Console, helper: &ChaoxingHelper, sso: &SsoHelper) {
    if let Err(error) = sso.login_to_chaoxing().await {
        eprintln!("进入学习通失败：{error}");
        return;
    }

    while let Some(action) = console.menu(
        "超星学习通",
        &[
            (Action::Profile, "查看个人信息"),
            (Action::Assignments, "查看全部作业"),
        ],
        "返回",
    ) {
        let result = match action {
            Action::Profile => profile(helper).await,
            Action::Assignments => assignments(helper).await,
        };
        if let Err(error) = result {
            eprintln!("操作失败：{error}");
        }
    }
}

async fn profile(helper: &ChaoxingHelper) -> Result<(), ChaoxingError> {
    let profile = helper.get_profile().await?;
    println!("姓名：{}", profile.name);
    Ok(())
}

async fn assignments(helper: &ChaoxingHelper) -> Result<(), ChaoxingError> {
    let assignments = helper.get_assignments().await?;
    if assignments.is_empty() {
        println!("暂无作业。");
    }
    for assignment in assignments {
        println!(
            "{} | {} | {}",
            assignment.course_name,
            assignment.title,
            assignment.status.display_name(),
        );
        if let Some(deadline) = assignment.deadline {
            println!(
                "  截止时间：{}",
                display_date(deadline.with_timezone(&SCHOOL_OFFSET))
            );
        }
        println!("  详情：{}", assignment.detail_url);
    }
    Ok(())
}
