use std::sync::Arc;

use csustkit::{
    mooc::{MoocError, MoocHelper},
    sso::SsoHelper,
};

use super::{console, formatting};

pub async fn run_menu(mooc: Arc<MoocHelper>, sso: Arc<SsoHelper>) {
    if let Err(error) = sso.login_to_mooc().await {
        println!("进入网络课程中心失败: {error}");
        return;
    }
    loop {
        println!("\n=== 网络课程中心 ===");
        println!("1. 查看个人信息");
        println!("2. 查看课程列表");
        println!("3. 查看待完成作业课程");
        println!("4. 查看某门课程的作业");
        println!("5. 查看某门课程的测验");
        println!("0. 返回上一级");
        let Some(choice) = console::prompt("请选择") else {
            return;
        };
        let result = match choice.as_str() {
            "1" => profile(&mooc).await,
            "2" => courses(&mooc).await,
            "3" => pending(&mooc).await,
            "4" => course_detail(&mooc, true).await,
            "5" => course_detail(&mooc, false).await,
            "0" => return,
            _ => {
                println!("输入无效，请重新选择。");
                continue;
            }
        };
        if let Err(error) = result {
            println!("操作失败: {error}");
        }
    }
}

async fn profile(helper: &MoocHelper) -> Result<(), MoocError> {
    let profile = helper.get_profile().await?;
    println!("\n姓名: {}", profile.name);
    println!("上次登录: {}", profile.last_login_time);
    println!("在线总时长: {}", profile.total_online_time);
    println!("登录次数: {}", profile.login_count);
    Ok(())
}

async fn courses(helper: &MoocHelper) -> Result<(), MoocError> {
    let mut courses = helper.get_courses().await?;
    courses.sort_by(|left, right| left.name.cmp(&right.name));
    formatting::print_mooc_courses(&courses);
    Ok(())
}

async fn pending(helper: &MoocHelper) -> Result<(), MoocError> {
    let mut courses = helper.get_courses_with_pending_assignments().await?;
    courses.sort_by(|left, right| left.name.cmp(&right.name));
    if courses.is_empty() {
        println!("暂无待完成作业课程。");
    } else {
        println!("待提交作业课程:");
        formatting::print_mooc_courses(&courses);
    }
    Ok(())
}

async fn course_detail(helper: &MoocHelper, assignments: bool) -> Result<(), MoocError> {
    let mut courses = helper.get_courses().await?;
    courses.sort_by(|left, right| left.name.cmp(&right.name));
    let Some(course) = console::select("请选择课程", &courses, |course| {
        let number = course
            .number
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("无编号");
        let teacher = course
            .teacher
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("未知教师");
        format!("{} [{number}] - {teacher}", course.name)
    }) else {
        return Ok(());
    };
    if assignments {
        formatting::print_assignments(&helper.get_course_assignments(course).await?, course);
    } else {
        formatting::print_mooc_exams(&helper.get_course_exams(course).await?, course);
    }
    Ok(())
}
