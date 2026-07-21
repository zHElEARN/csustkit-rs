use chrono::{DateTime, FixedOffset};
use csustkit::{
    edu::{Course, CourseGrade, Exam, ScheduleSession},
    mooc::{Assignment, Course as MoocCourse, Exam as MoocExam},
};

pub fn print_mooc_courses(courses: &[MoocCourse]) {
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

pub fn print_assignments(assignments: &[Assignment], course: &MoocCourse) {
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

pub fn print_mooc_exams(exams: &[MoocExam], course: &MoocCourse) {
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

pub fn print_education_exams(exams: &[Exam]) {
    if exams.is_empty() {
        println!("暂无考试安排。");
        return;
    }
    println!("\n考试安排:");
    for (index, exam) in exams.iter().enumerate() {
        println!("{}. {} [{}]", index + 1, exam.course_name, exam.course_id);
        println!("   时间: {}", exam.exam_time);
        println!(
            "   校区: {} | 考场: {} | 座位号: {}",
            exam.campus, exam.exam_room, exam.seat_number
        );
        println!("   教师: {} | 场次: {}", exam.teacher, exam.session);
    }
}

pub fn print_course_grades(grades: &[CourseGrade]) {
    if grades.is_empty() {
        println!("暂无课程成绩。");
        return;
    }
    println!("\n课程成绩:");
    for (index, grade) in grades.iter().enumerate() {
        println!("{}. {} [{}]", index + 1, grade.course_name, grade.course_id);
        println!(
            "   学期: {} | 成绩: {} | 绩点: {}",
            grade.semester, grade.grade, grade.grade_point
        );
        println!(
            "   学分: {} | 考核方式: {} | 课程性质: {}",
            grade.credit,
            grade.assessment_method,
            grade.course_nature.display_name()
        );
    }
}

pub fn print_course_schedule(courses: &[Course]) {
    if courses.is_empty() {
        println!("暂无课程表数据。");
        return;
    }
    println!("\n课程表:");
    for (index, course) in courses.iter().enumerate() {
        let teacher = course
            .teacher
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("未知教师");
        println!("{}. {} - {teacher}", index + 1, course.course_name);
        let mut sessions = course.sessions.clone();
        sessions.sort_by_key(|session| {
            (
                session.day_of_week as u8,
                session.start_section,
                session.end_section,
            )
        });
        for session in sessions {
            let classroom = session.classroom.as_deref().unwrap_or("未提供");
            println!(
                "   {} 第{}-{}节 | 周次: {} | 教室: {classroom}",
                display_day(session.day_of_week as u8),
                session.start_section,
                session.end_section,
                display_weeks(&session)
            );
        }
    }
}

pub fn print_available_classrooms(
    classrooms: &[String],
    campus: &str,
    week: i64,
    day: u8,
    section: i64,
) {
    if classrooms.is_empty() {
        println!(
            "{campus} 第{week}周 {} 第{section}大节暂无空闲教室。",
            display_day(day)
        );
        return;
    }
    println!(
        "\n{campus} 第{week}周 {} 第{section}大节空闲教室:",
        display_day(day)
    );
    for (index, classroom) in classrooms.iter().enumerate() {
        println!("{}. {classroom}", index + 1);
    }
}

pub fn display_date(date: DateTime<FixedOffset>) -> String {
    date.format("%Y-%m-%d %H:%M").to_string()
}

fn display_weeks(session: &ScheduleSession) -> String {
    session
        .weeks
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn display_day(day: u8) -> &'static str {
    [
        "星期日",
        "星期一",
        "星期二",
        "星期三",
        "星期四",
        "星期五",
        "星期六",
    ]
    .get(day as usize)
    .copied()
    .unwrap_or("未知日期")
}

fn yes_no(value: bool) -> &'static str {
    if value { "是" } else { "否" }
}
