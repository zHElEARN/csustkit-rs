use csustkit::{
    mooc::{Assignment, Course, Exam, MoocError, MoocHelper},
    sso::SsoHelper,
};

use super::{Console, display_date};

#[derive(Clone, Copy)]
enum Action {
    Profile,
    Courses,
    Pending,
    CourseDetail(CourseDetail),
}

#[derive(Clone, Copy)]
enum CourseDetail {
    Assignments,
    Exams,
}

pub async fn run_menu(console: &mut Console, helper: &MoocHelper, sso: &SsoHelper) {
    if let Err(error) = sso.login_to_mooc().await {
        eprintln!("进入网络课程中心失败：{error}");
        return;
    }

    while let Some(action) = console.menu(
        "网络课程中心",
        &[
            (Action::Profile, "查看个人信息"),
            (Action::Courses, "查看课程列表"),
            (Action::Pending, "查看有待交作业的课程"),
            (
                Action::CourseDetail(CourseDetail::Assignments),
                "查看课程作业",
            ),
            (Action::CourseDetail(CourseDetail::Exams), "查看课程测验"),
        ],
        "返回",
    ) {
        let result = match action {
            Action::Profile => profile(helper).await,
            Action::Courses => courses(helper).await,
            Action::Pending => pending(helper).await,
            Action::CourseDetail(detail) => course_detail(console, helper, detail).await,
        };
        if let Err(error) = result {
            eprintln!("操作失败：{error}");
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
    print_mooc_courses(&courses);
    Ok(())
}

async fn pending(helper: &MoocHelper) -> Result<(), MoocError> {
    let mut courses = helper.get_courses_with_pending_assignments().await?;
    courses.sort_by(|left, right| left.name.cmp(&right.name));
    if courses.is_empty() {
        println!("暂无待完成作业课程。");
    } else {
        println!("待提交作业课程:");
        print_mooc_courses(&courses);
    }
    Ok(())
}

async fn course_detail(
    console: &mut Console,
    helper: &MoocHelper,
    detail: CourseDetail,
) -> Result<(), MoocError> {
    let mut courses = helper.get_courses().await?;
    courses.sort_by(|left, right| left.name.cmp(&right.name));
    if courses.is_empty() {
        println!("暂无课程数据。");
        return Ok(());
    }
    let Some(course) = console.select("选择课程", &courses, |course| {
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
    match detail {
        CourseDetail::Assignments => {
            let assignments = helper.get_course_assignments(course).await?;
            print_assignments(&assignments, course);
        }
        CourseDetail::Exams => {
            let exams = helper.get_course_exams(course).await?;
            print_mooc_exams(&exams, course);
        }
    }
    Ok(())
}

fn print_mooc_courses(courses: &[Course]) {
    if courses.is_empty() {
        println!("暂无课程数据。");
        return;
    }
    println!("\n课程列表:");
    for (index, course) in courses.iter().enumerate() {
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
        let department = course
            .department
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("未知院系");
        println!("{}. {}", index + 1, course.name);
        println!("   编号: {number} | 教师: {teacher} | 院系: {department}");
    }
}

fn print_assignments(assignments: &[Assignment], course: &Course) {
    if assignments.is_empty() {
        println!("{} 暂无作业数据。", course.name);
        return;
    }
    println!("\n{} 的作业:", course.name);
    for (index, assignment) in assignments.iter().enumerate() {
        println!("{}. {}", index + 1, assignment.title);
        println!("   发布人: {}", assignment.publisher);
        println!("   开始时间: {}", display_date(assignment.start_time));
        println!("   截止时间: {}", display_date(assignment.deadline));
        println!(
            "   可提交: {} | 已提交: {}",
            yes_no(assignment.can_submit),
            yes_no(assignment.submit_status)
        );
    }
}

fn print_mooc_exams(exams: &[Exam], course: &Course) {
    if exams.is_empty() {
        println!("{} 暂无测验数据。", course.name);
        return;
    }
    println!("\n{} 的测验:", course.name);
    for (index, exam) in exams.iter().enumerate() {
        let retake = exam
            .allow_retake
            .map(|value| value.to_string())
            .unwrap_or_else(|| "不限制".to_owned());
        println!("{}. {}", index + 1, exam.title);
        println!("   开始时间: {}", exam.start_time);
        println!("   截止时间: {}", exam.end_time);
        println!(
            "   限时: {} 分钟 | 可重考次数: {retake} | 已交卷: {}",
            exam.time_limit,
            yes_no(exam.is_submitted)
        );
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "是" } else { "否" }
}
