use csustkit::physics_experiment::{PhysicsExperimentError, PhysicsExperimentHelper};

use super::{Console, display_date};

#[derive(Clone, Copy)]
enum Action {
    Courses,
    Grades,
    Logout,
}

pub async fn run_menu(console: &mut Console, helper: &PhysicsExperimentHelper) {
    let Some(username) =
        console.username("CSUST_PHYSICS_EXPERIMENT_USERNAME", "请输入大物实验用户名")
    else {
        return;
    };
    let Some(password) =
        console.password("CSUST_PHYSICS_EXPERIMENT_PASSWORD", "请输入大物实验密码")
    else {
        return;
    };
    if let Err(error) = helper.login(&username, &password).await {
        eprintln!("登录大物实验失败：{error}");
        return;
    }

    while let Some(action) = console.menu(
        "大物实验",
        &[
            (Action::Courses, "查看实验课表"),
            (Action::Grades, "查看实验成绩"),
            (Action::Logout, "登出大物实验"),
        ],
        "返回",
    ) {
        let result = match action {
            Action::Courses => courses(helper).await,
            Action::Grades => grades(helper).await,
            Action::Logout => match helper.logout().await {
                Ok(()) => {
                    println!("已登出大物实验。");
                    return;
                }
                Err(error) => {
                    eprintln!("登出大物实验失败：{error}");
                    continue;
                }
            },
        };
        if let Err(error) = result {
            eprintln!("操作失败：{error}");
        }
    }
}

async fn courses(helper: &PhysicsExperimentHelper) -> Result<(), PhysicsExperimentError> {
    let courses = helper.get_courses().await?;
    if courses.is_empty() {
        println!("暂无实验排课。");
    }
    for course in courses {
        println!(
            "{} | {} | {} | {} 至 {}",
            course.name,
            course.teacher,
            course.location,
            display_date(course.start_time),
            display_date(course.end_time),
        );
    }
    Ok(())
}

async fn grades(helper: &PhysicsExperimentHelper) -> Result<(), PhysicsExperimentError> {
    let grades = helper.get_course_grades().await?;
    if grades.is_empty() {
        println!("暂无实验成绩。");
    }
    for grade in grades {
        println!(
            "{} | {} | 总成绩 {}",
            grade.course_name, grade.item_name, grade.total_grade,
        );
    }
    Ok(())
}
