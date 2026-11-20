use std::env;

use csustkit::physics_experiment::PhysicsExperimentHelper;

use super::{console, formatting};

pub async fn run_menu(helper: &PhysicsExperimentHelper) {
    let username = env::var("CSUST_PHYSICS_EXPERIMENT_USERNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| console::prompt_non_empty("请输入大物实验用户名"));
    let Some(username) = username else { return };
    let password = env::var("CSUST_PHYSICS_EXPERIMENT_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| console::prompt_password("请输入大物实验密码"));
    let Some(password) = password else { return };
    if let Err(error) = helper.login(&username, &password).await {
        println!("登录大物实验失败: {error}");
        return;
    }

    loop {
        println!("\n=== 大物实验 ===");
        println!("1. 查看实验课表");
        println!("2. 查看实验成绩");
        println!("3. 登出大物实验");
        println!("0. 返回上一级");
        match console::prompt("请选择").as_deref() {
            Some("1") => match helper.get_courses().await {
                Ok(courses) if courses.is_empty() => println!("暂无实验排课。"),
                Ok(courses) => {
                    for course in courses {
                        println!(
                            "{} | {} | {} | {} 至 {}",
                            course.name,
                            course.teacher,
                            course.location,
                            formatting::display_date(course.start_time),
                            formatting::display_date(course.end_time),
                        );
                    }
                }
                Err(error) => println!("操作失败: {error}"),
            },
            Some("2") => match helper.get_course_grades().await {
                Ok(grades) if grades.is_empty() => println!("暂无实验成绩。"),
                Ok(grades) => {
                    for grade in grades {
                        println!(
                            "{} | {} | 总成绩 {}",
                            grade.course_name, grade.item_name, grade.total_grade,
                        );
                    }
                }
                Err(error) => println!("操作失败: {error}"),
            },
            Some("3") => match helper.logout().await {
                Ok(()) => return,
                Err(error) => println!("登出失败: {error}"),
            },
            Some("0") | None => return,
            _ => println!("输入无效，请重新选择。"),
        }
    }
}
