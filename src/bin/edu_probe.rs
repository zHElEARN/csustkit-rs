use std::{env, sync::Arc};

use csustkit::{
    ConnectionMode, CsustSession,
    edu::{
        AvailableClassroomsQuery, Campus, CourseGradeQuery, DayOfWeek, EduError, EduHelper,
        ExamScheduleQuery,
    },
    sso::{SsoError, SsoHelper},
};

#[derive(Debug)]
enum ProbeError {
    MissingCredentials,
    CaptchaRequired(&'static str),
    Validation(String),
    Session(reqwest::Error),
    Sso(SsoError),
    Education(EduError),
}

impl From<SsoError> for ProbeError {
    fn from(error: SsoError) -> Self {
        Self::Sso(error)
    }
}

impl From<EduError> for ProbeError {
    fn from(error: EduError) -> Self {
        Self::Education(error)
    }
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials => write!(formatter, "未从环境变量读取到必要凭据"),
            Self::CaptchaRequired(mode) => {
                write!(formatter, "{mode} 登录需要验证码，按要求停止测试")
            }
            Self::Validation(message) => write!(formatter, "探测校验失败: {message}"),
            Self::Session(error) => write!(formatter, "共享会话创建失败: {error}"),
            Self::Sso(error) => write!(formatter, "SSO 操作失败: {error}"),
            Self::Education(error) => write!(formatter, "教务操作失败: {error}"),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Session(error) => Some(error),
            Self::Sso(error) => Some(error),
            Self::Education(error) => Some(error),
            _ => None,
        }
    }
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => println!("Direct 和 WebVPN 的教务查询与登出链路均验证成功。"),
        Err(error) => {
            println!("测试停止: {error}");
            std::process::exit(1);
        }
    }
}

async fn run() -> Result<(), ProbeError> {
    let username = required_environment("CSUST_AUTHSERVER_USERNAME")?;
    let password = required_environment("CSUST_AUTHSERVER_PASSWORD")?;

    test_mode(ConnectionMode::Direct, &username, &password).await?;
    test_mode(ConnectionMode::WebVpn, &username, &password).await?;
    Ok(())
}

fn required_environment(name: &str) -> Result<String, ProbeError> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(ProbeError::MissingCredentials)
}

async fn test_mode(mode: ConnectionMode, username: &str, password: &str) -> Result<(), ProbeError> {
    let mode_name = match mode {
        ConnectionMode::Direct => "Direct",
        ConnectionMode::WebVpn => "WebVPN",
    };
    println!("开始测试 {mode_name} 教务链路 ...");

    let session = CsustSession::new().map_err(ProbeError::Session)?;
    let sso = SsoHelper::with_session(mode, Arc::clone(&session));
    let education = EduHelper::with_session(mode, session);

    let login_form = sso.get_login_form().await?;
    if sso.check_need_captcha(username.to_owned()).await? {
        return Err(ProbeError::CaptchaRequired(mode_name));
    }
    sso.login(login_form, username.to_owned(), password.to_owned(), None)
        .await?;
    sso.login_to_education().await?;

    if !education.is_logged_in().await {
        return Err(ProbeError::Education(EduError::NotLoggedIn));
    }
    let profile = education.get_profile().await?;
    if profile.name.is_empty() || profile.student_id.is_empty() {
        println!(
            "{mode_name} 教务档案姓名或学号为空，继续验证成绩链路。警告：这是已有档案解析结果。"
        );
    }
    let exam_semesters = education
        .get_available_semesters_for_exam_schedule()
        .await?;
    println!(
        "{mode_name} 可用考试学期读取成功，共 {} 个。",
        exam_semesters.semesters.len()
    );
    let exams = education
        .get_exam_schedule(ExamScheduleQuery::default())
        .await?;
    println!("{mode_name} 考试安排读取成功，共 {} 条。", exams.len());

    let schedule_semesters = education
        .get_available_semesters_for_course_schedule()
        .await?;
    println!(
        "{mode_name} 可用课表学期读取成功，共 {} 个。",
        schedule_semesters.semesters.len()
    );
    let schedule = education.get_course_schedule(None).await?;
    let session_count = schedule
        .courses
        .iter()
        .map(|course| course.sessions.len())
        .sum::<usize>();
    println!(
        "{mode_name} 课程表读取成功，共 {} 门课、{session_count} 个时段、{} 个备注。",
        schedule.courses.len(),
        schedule.remarks.len()
    );

    let start_date_semesters = education.get_available_semesters_for_start_date().await?;
    println!(
        "{mode_name} 学期首日可选学期读取成功，共 {} 个。",
        start_date_semesters.semesters.len()
    );
    let semester_start_date = education.get_semester_start_date(None).await?;
    println!("{mode_name} 默认学期首日读取成功：{semester_start_date}。");

    let classrooms = education
        .get_available_classrooms(AvailableClassroomsQuery {
            campus: Campus::Yuntang,
            week: 1,
            day_of_week: DayOfWeek::Monday,
            section: 1,
        })
        .await?;
    println!("{mode_name} 空闲教室读取成功，共 {} 间。", classrooms.len());

    let semesters = education
        .get_available_semesters_for_course_grades()
        .await?;
    if semesters.is_empty() {
        return Err(ProbeError::Validation(format!(
            "{mode_name} 未返回可用成绩学期"
        )));
    }
    println!(
        "{mode_name} 可用成绩学期读取成功，共 {} 个。",
        semesters.len()
    );

    match education
        .get_course_grades(CourseGradeQuery::default())
        .await
    {
        Ok(grades) => {
            println!("{mode_name} 课程成绩读取成功，共 {} 条。", grades.len());
            if let Some(first_grade) = grades.first() {
                let detail = education
                    .get_grade_detail(&first_grade.grade_detail_url)
                    .await?;
                println!(
                    "{mode_name} 成绩详情读取成功，包含 {} 个组成项。",
                    detail.components.len()
                );
            } else {
                println!(
                    "{mode_name} 课程成绩为空，跳过成绩详情验证。警告：这是已有成绩查询结果。"
                );
            }
        }
        Err(EduError::CourseGradesRetrievalFailed(message)) if message == "未查询到数据" => {
            println!(
                "{mode_name} 未查询到课程成绩，跳过成绩详情验证。警告：这是已有成绩查询结果。"
            );
        }
        Err(error) => return Err(ProbeError::Education(error)),
    }

    education.logout().await?;
    sso.logout().await?;
    Ok(())
}
