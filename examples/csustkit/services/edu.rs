use csustkit::{
    DayOfWeek,
    edu::{
        ClassroomScheduleQuery, ClassroomScheduleSession, Course, CourseGrade, CourseGradeQuery,
        EduError, EduHelper, Exam, ExamScheduleQuery, SemesterOptions,
    },
    sso::SsoHelper,
};

use super::{Console, choose_campus};

#[derive(Clone, Copy)]
enum Action {
    Profile,
    Exams,
    Grades,
    Schedule,
    Classrooms,
    SemesterStartDate,
}

pub async fn run_menu(console: &mut Console, helper: &EduHelper, sso: &SsoHelper) {
    if let Err(error) = sso.login_to_education().await {
        eprintln!("进入教务系统失败：{error}");
        return;
    }

    while let Some(action) = console.menu(
        "教务系统",
        &[
            (Action::Profile, "查看个人信息"),
            (Action::Exams, "查看考试安排"),
            (Action::Grades, "查看课程成绩"),
            (Action::Schedule, "查看课程表"),
            (Action::Classrooms, "查看教室课表"),
            (Action::SemesterStartDate, "查看学期首日"),
        ],
        "返回",
    ) {
        let result = match action {
            Action::Profile => profile(helper).await,
            Action::Exams => exams(console, helper).await,
            Action::Grades => grades(console, helper).await,
            Action::Schedule => schedule(console, helper).await,
            Action::Classrooms => classrooms(console, helper).await,
            Action::SemesterStartDate => semester_start_date(console, helper).await,
        };
        if let Err(error) = result {
            eprintln!("操作失败：{error}");
        }
    }
}

async fn profile(helper: &EduHelper) -> Result<(), EduError> {
    let profile = helper.get_profile().await?;
    println!("\n姓名: {}", profile.name);
    println!("学号: {}", profile.student_id);
    println!("院系: {}", profile.department);
    println!("专业: {}", profile.major);
    println!("班级: {}", profile.class_name);
    println!("联系电话: {}", profile.personal_phone);
    Ok(())
}

async fn exams(console: &mut Console, helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_exam_schedule().await?;
    let Some(semester) = choose_semester(console, &semesters) else {
        return Ok(());
    };
    let exams = helper
        .get_exam_schedule(ExamScheduleQuery {
            academic_year_semester: Some(semester.to_owned()),
            semester_type: None,
        })
        .await?;
    print_education_exams(&exams);
    Ok(())
}

async fn grades(console: &mut Console, helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_course_grades().await?;
    let choices: Vec<_> = std::iter::once(None)
        .chain(semesters.iter().map(|semester| Some(semester.as_str())))
        .collect();
    let Some(&semester) = console.select("选择成绩学期", &choices, |semester| {
        semester.unwrap_or("全部学期")
    }) else {
        return Ok(());
    };
    let result = helper
        .get_course_grades(CourseGradeQuery {
            academic_year_semester: semester.map(str::to_owned),
            ..CourseGradeQuery::default()
        })
        .await;
    let grades = match result {
        Ok(grades) => grades,
        Err(EduError::CourseGradesRetrievalFailed(message)) if message == "未查询到数据" => {
            println!("暂无课程成绩。");
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    if grades.is_empty() {
        println!("暂无课程成绩。");
        return Ok(());
    }
    if let Some(grade) = console.select("课程成绩（选择课程查看详情）", &grades, grade_label)
    {
        let detail = helper.get_grade_detail(&grade.grade_detail_url).await?;
        println!(
            "\n{} 成绩详情（总评 {}）:",
            grade.course_name, detail.total_grade
        );
        for component in detail.components {
            println!(
                "{}: {}（占比 {}%）",
                component.component_type, component.grade, component.ratio
            );
        }
    }
    Ok(())
}

async fn schedule(console: &mut Console, helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_course_schedule().await?;
    let Some(semester) = choose_semester(console, &semesters) else {
        return Ok(());
    };
    let schedule = helper
        .get_course_schedule(Some(semester.to_owned()))
        .await?;
    print_course_schedule(&schedule.courses);
    if !schedule.remarks.is_empty() {
        println!("备注:");
        for remark in schedule.remarks {
            println!("- {remark}");
        }
    }
    Ok(())
}

async fn classrooms(console: &mut Console, helper: &EduHelper) -> Result<(), EduError> {
    const WEEKDAYS: [DayOfWeek; 7] = [
        DayOfWeek::Monday,
        DayOfWeek::Tuesday,
        DayOfWeek::Wednesday,
        DayOfWeek::Thursday,
        DayOfWeek::Friday,
        DayOfWeek::Saturday,
        DayOfWeek::Sunday,
    ];

    let Some(campus) = choose_campus(console) else {
        return Ok(());
    };
    let Some(week) = console.number("请输入周次", 1..=30) else {
        return Ok(());
    };
    let Some(day) = console.select("选择星期", &WEEKDAYS, |day| day_name(*day)) else {
        return Ok(());
    };
    let Some(section) = console.number("请输入大节", 1..=5) else {
        return Ok(());
    };
    let day = weekday_number(*day);
    let first_section = section * 2 - 1;
    let schedule = helper
        .get_classroom_schedule(ClassroomScheduleQuery {
            campus: Some(campus),
            weeks: Some(week..=week),
            weekdays: Some(day..=day),
            sections: Some(first_section..=first_section + 1),
            ..ClassroomScheduleQuery::default()
        })
        .await?;
    print_classroom_schedule(&schedule, campus.display_name());
    Ok(())
}

async fn semester_start_date(console: &mut Console, helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_start_date().await?;
    let Some(semester) = choose_semester(console, &semesters) else {
        return Ok(());
    };
    let date = helper
        .get_semester_start_date(Some(semester.to_owned()))
        .await?;
    println!("学期首日: {date}");
    Ok(())
}

fn choose_semester<'a>(console: &mut Console, options: &'a SemesterOptions) -> Option<&'a str> {
    if options.semesters.is_empty() {
        println!("暂无可用学期。");
        return None;
    }
    console
        .select("选择学期", &options.semesters, |semester| {
            if semester == &options.default_semester {
                format!("{semester}（默认）")
            } else {
                semester.clone()
            }
        })
        .map(String::as_str)
}

fn grade_label(grade: &CourseGrade) -> String {
    format!(
        "{} [{}]
   学期：{} | 成绩：{} | 绩点：{} | 学分：{} | 考核方式：{} | 课程性质：{}",
        grade.course_name,
        grade.course_id,
        grade.semester,
        grade.grade,
        grade.grade_point,
        grade.credit,
        grade.assessment_method,
        grade.course_nature.display_name()
    )
}

fn weekday_number(day: DayOfWeek) -> i64 {
    match day {
        DayOfWeek::Sunday => 7,
        _ => i64::from(day as u8),
    }
}

fn day_name(day: DayOfWeek) -> &'static str {
    match day {
        DayOfWeek::Monday => "星期一",
        DayOfWeek::Tuesday => "星期二",
        DayOfWeek::Wednesday => "星期三",
        DayOfWeek::Thursday => "星期四",
        DayOfWeek::Friday => "星期五",
        DayOfWeek::Saturday => "星期六",
        DayOfWeek::Sunday => "星期日",
    }
}

fn print_education_exams(exams: &[Exam]) {
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

fn print_course_schedule(courses: &[Course]) {
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
        let mut sessions = course.sessions.iter().collect::<Vec<_>>();
        sessions.sort_by_key(|session| {
            (
                weekday_number(session.day_of_week),
                session.start_section,
                session.end_section,
            )
        });
        for session in sessions {
            let classroom = session.classroom.as_deref().unwrap_or("未提供");
            println!(
                "   {} 第{}-{}节 | 周次: {} | 教室: {classroom}",
                day_name(session.day_of_week),
                session.start_section,
                session.end_section,
                display_weeks(&session.weeks)
            );
        }
    }
}

fn print_classroom_schedule(schedule: &[ClassroomScheduleSession], campus: &str) {
    if schedule.is_empty() {
        println!("{campus}在所选时段暂无排课记录。");
        return;
    }
    println!("\n{campus}教室课表:");
    for session in schedule {
        println!(
            "{} | {} 第{}-{}节 | 周次: {} | {}",
            session.classroom,
            day_name(session.day_of_week),
            session.start_section,
            session.end_section,
            display_weeks(&session.weeks),
            session.course_label,
        );
    }
}

fn display_weeks(weeks: &[i64]) -> String {
    weeks
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
