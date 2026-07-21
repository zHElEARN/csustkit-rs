use std::sync::Arc;

use csustkit::{
    edu::{
        AvailableClassroomsQuery, Campus, CourseGradeQuery, DayOfWeek, EduError, EduHelper,
        ExamScheduleQuery, SemesterOptions,
    },
    sso::SsoHelper,
};

use super::{console, formatting};

pub async fn run_menu(helper: Arc<EduHelper>, sso: Arc<SsoHelper>) {
    if let Err(error) = sso.login_to_education().await {
        println!("进入教务系统失败: {error}");
        return;
    }
    loop {
        println!("\n=== 教务系统 ===");
        println!("1. 查看个人信息");
        println!("2. 查看考试安排");
        println!("3. 查看课程成绩");
        println!("4. 查看课程表");
        println!("5. 查看空闲教室");
        println!("6. 查看学期首日");
        println!("0. 返回上一级");
        let Some(choice) = console::prompt("请选择") else {
            return;
        };
        let result = match choice.as_str() {
            "1" => profile(&helper).await,
            "2" => exams(&helper).await,
            "3" => grades(&helper).await,
            "4" => schedule(&helper).await,
            "5" => classrooms(&helper).await,
            "6" => semester_start_date(&helper).await,
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

async fn exams(helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_exam_schedule().await?;
    let Some(semester) = choose_semester(&semesters)? else {
        return Ok(());
    };
    let exams = helper
        .get_exam_schedule(ExamScheduleQuery {
            academic_year_semester: Some(semester),
            semester_type: None,
        })
        .await?;
    formatting::print_education_exams(&exams);
    Ok(())
}

async fn grades(helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_course_grades().await?;
    let Some(semester) = console::select("请选择成绩学期", &semesters, Clone::clone) else {
        return Ok(());
    };
    let result = helper
        .get_course_grades(CourseGradeQuery {
            academic_year_semester: Some(semester.clone()),
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
    formatting::print_course_grades(&grades);
    if let Some(grade) = console::select("查看成绩详情（可选）", &grades, |grade| {
        format!("{} [{}]", grade.course_name, grade.grade)
    }) {
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

async fn schedule(helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_course_schedule().await?;
    let Some(semester) = choose_semester(&semesters)? else {
        return Ok(());
    };
    let schedule = helper.get_course_schedule(Some(semester)).await?;
    formatting::print_course_schedule(&schedule.courses);
    if !schedule.remarks.is_empty() {
        println!("备注:");
        for remark in schedule.remarks {
            println!("- {remark}");
        }
    }
    Ok(())
}

async fn classrooms(helper: &EduHelper) -> Result<(), EduError> {
    let Some(campus) = console::select_value(
        "请选择校区",
        &[(Campus::Yuntang, "云塘"), (Campus::Jinpenling, "金盆岭")],
    ) else {
        return Ok(());
    };
    let Some(week) = console::prompt_int("请输入周次", 1, 30) else {
        return Ok(());
    };
    let Some(day) = console::prompt_int("请输入星期（1-7）", 1, 7) else {
        return Ok(());
    };
    let Some(section) = console::prompt_int("请输入大节（1-5）", 1, 5) else {
        return Ok(());
    };
    let day_of_week = match day {
        1 => DayOfWeek::Monday,
        2 => DayOfWeek::Tuesday,
        3 => DayOfWeek::Wednesday,
        4 => DayOfWeek::Thursday,
        5 => DayOfWeek::Friday,
        6 => DayOfWeek::Saturday,
        _ => DayOfWeek::Sunday,
    };
    let classrooms = helper
        .get_available_classrooms(AvailableClassroomsQuery {
            campus,
            week,
            day_of_week,
            section,
        })
        .await?;
    formatting::print_available_classrooms(
        &classrooms,
        campus_name(campus),
        week,
        day as u8,
        section,
    );
    Ok(())
}

async fn semester_start_date(helper: &EduHelper) -> Result<(), EduError> {
    let semesters = helper.get_available_semesters_for_start_date().await?;
    let Some(semester) = choose_semester(&semesters)? else {
        return Ok(());
    };
    let date = helper.get_semester_start_date(Some(semester)).await?;
    println!("学期首日: {date}");
    Ok(())
}

fn choose_semester(options: &SemesterOptions) -> Result<Option<String>, EduError> {
    if options.semesters.is_empty() {
        println!("暂无可用学期。");
        return Ok(None);
    }
    println!("默认学期: {}", options.default_semester);
    Ok(console::select("请选择学期", &options.semesters, Clone::clone).cloned())
}

fn campus_name(campus: Campus) -> &'static str {
    match campus {
        Campus::Yuntang => "云塘校区",
        Campus::Jinpenling => "金盆岭校区",
    }
}
