use csustkit::{chaoxing::ChaoxingHelper, sso::SsoHelper};

use super::{console, formatting};

const SCHOOL_OFFSET: chrono::FixedOffset = chrono::FixedOffset::east_opt(8 * 60 * 60).unwrap();

pub async fn run_menu(helper: &ChaoxingHelper, sso: &SsoHelper) {
    if let Err(error) = sso.login_to_chaoxing().await {
        println!("进入学习通失败: {error}");
        return;
    }
    loop {
        println!("\n=== 超星学习通 ===");
        println!("1. 查看个人信息");
        println!("2. 查看全部作业");
        println!("0. 返回上一级");
        match console::prompt("请选择").as_deref() {
            Some("1") => match helper.get_profile().await {
                Ok(profile) => println!("姓名: {}", profile.name),
                Err(error) => println!("操作失败: {error}"),
            },
            Some("2") => match helper.get_assignments().await {
                Ok(assignments) if assignments.is_empty() => println!("暂无作业。"),
                Ok(assignments) => {
                    for assignment in assignments {
                        println!(
                            "{} | {} | {}",
                            assignment.course_name,
                            assignment.title,
                            assignment.status.display_name(),
                        );
                        if let Some(deadline) = assignment.deadline {
                            println!(
                                "  截止时间: {}",
                                formatting::display_date(deadline.with_timezone(&SCHOOL_OFFSET))
                            );
                        }
                        println!("  详情: {}", assignment.detail_url);
                    }
                }
                Err(error) => println!("操作失败: {error}"),
            },
            Some("0") | None => return,
            _ => println!("输入无效，请重新选择。"),
        }
    }
}
