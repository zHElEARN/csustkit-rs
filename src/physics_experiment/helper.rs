use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, FixedOffset, NaiveDateTime, NaiveTime};
use encoding_rs::UTF_8;
use reqwest::header::CONTENT_LENGTH;
use scraper::{ElementRef, Html, Selector};
use url::form_urlencoded;

use crate::{
    ConnectionMode, CsustSession, DayOfWeek,
    request::{LoginCheck, RequestResponse, perform_request},
    url_factory::{ServiceDomain, make_url},
};

use super::{Course, CourseGrade, PhysicsExperimentError};

const COURSES_PATH: &str = "/Student/myalltasklist.aspx?generalCourseId=2&generalCourseName=%E5%A4%A7%E5%AD%A6%E7%89%A9%E7%90%86%E5%AE%9E%E9%AA%8C";
const GRADES_PATH: &str = "/Student/GeneralCourseScore.aspx";
const SCHOOL_OFFSET: FixedOffset = FixedOffset::east_opt(8 * 60 * 60).unwrap();

/// 使用独立的物理实验账号访问课表和成绩。
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct PhysicsExperimentHelper {
    mode: ConnectionMode,
    session: CsustSession,
}

impl PhysicsExperimentHelper {
    pub fn new(mode: ConnectionMode, session: CsustSession) -> Self {
        Self { mode, session }
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export(async_runtime = "tokio"))]
impl PhysicsExperimentHelper {
    /// 登录物理实验平台。WebVPN 模式下应先在共享会话中登录 WebVPN。
    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(), PhysicsExperimentError> {
        let query = form_urlencoded::Serializer::new(String::new())
            .append_pair("UserType", "0")
            .append_pair("txtUserName", &STANDARD.encode(username))
            .append_pair("txtPass", &STANDARD.encode(password))
            .finish();
        let url = make_url(
            self.mode,
            ServiceDomain::PhysicsExperiment,
            &format!("/login.aspx?{query}"),
        );
        // 平台要求空的登录 POST 也发送 Content-Length，否则返回 HTTP 411。
        let request = self.session.client.post(url).header(CONTENT_LENGTH, "0");
        let response = perform_request::<reqwest::Error>(request, None)
            .await
            .map_err(reqwest::Error::without_url)?;
        if !response.status.is_success() || !response.text().contains("true") {
            return Err(PhysicsExperimentError::LoginFailed(response.status));
        }
        Ok(())
    }

    pub async fn is_logged_in(&self) -> bool {
        self.get_courses().await.is_ok()
    }

    pub async fn get_courses(&self) -> Result<Vec<Course>, PhysicsExperimentError> {
        let response = self.get(COURSES_PATH).await?;
        let document = Html::parse_document(&response.text());
        table_rows(&document, ".msgtable")
            .map_err(PhysicsExperimentError::SchedulesRetrievalFailed)?
            .into_iter()
            .skip(1)
            .map(parse_course)
            .collect()
    }

    pub async fn get_course_grades(&self) -> Result<Vec<CourseGrade>, PhysicsExperimentError> {
        let response = self.get(GRADES_PATH).await?;
        let document = Html::parse_document(&response.text());
        table_rows(&document, "#gvList")
            .map_err(PhysicsExperimentError::CourseGradesRetrievalFailed)?
            .into_iter()
            .skip(1)
            .map(parse_grade)
            .collect()
    }

    pub async fn logout(&self) -> Result<(), PhysicsExperimentError> {
        let url = make_url(self.mode, ServiceDomain::PhysicsExperiment, "/logout.aspx");
        perform_request::<PhysicsExperimentError>(self.session.client.get(url), None).await?;
        Ok(())
    }
}

impl PhysicsExperimentHelper {
    async fn get(&self, path: &str) -> Result<RequestResponse, PhysicsExperimentError> {
        let url = make_url(self.mode, ServiceDomain::PhysicsExperiment, path);
        perform_request(
            self.session.client.get(url),
            Some(LoginCheck {
                mode: self.mode,
                fallback_encoding: UTF_8,
                subsystem_login: |body, _| body.contains("name=\"txtUserName\""),
                not_logged_in: PhysicsExperimentError::NotLoggedIn,
            }),
        )
        .await
    }
}

fn table_rows<'a>(document: &'a Html, table: &str) -> Result<Vec<ElementRef<'a>>, String> {
    let table_selector = Selector::parse(table).map_err(|error| error.to_string())?;
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| "未找到表格".to_owned())?;
    let row_selector = Selector::parse("tr").expect("valid table row selector");
    let rows = table.select(&row_selector).collect::<Vec<_>>();
    if rows.is_empty() {
        return Err("未找到表头".to_owned());
    }
    Ok(rows)
}

fn parse_course(row: ElementRef<'_>) -> Result<Course, PhysicsExperimentError> {
    let cells = cells_text(row);
    let [
        id,
        name,
        batch,
        teacher,
        location,
        time,
        class_hours,
        week_info,
        ..,
    ] = cells.as_slice()
    else {
        return Err(schedule_error("课表列数不足"));
    };
    let (start_time, end_time) = parse_time(time)?;
    let (week, day_of_week) = parse_week(week_info)?;
    Ok(Course {
        id: id.parse().map_err(|_| schedule_error("课程 ID 格式无效"))?,
        name: name.clone(),
        batch: batch.clone(),
        teacher: teacher.clone(),
        location: location.clone(),
        start_time,
        end_time,
        class_hours: class_hours
            .parse()
            .map_err(|_| schedule_error("课时格式无效"))?,
        week,
        day_of_week,
    })
}

fn parse_grade(row: ElementRef<'_>) -> Result<CourseGrade, PhysicsExperimentError> {
    let cells = cells_text(row);
    let [code, course, item, preview, operation, report, total, ..] = cells.as_slice() else {
        return Err(PhysicsExperimentError::CourseGradesRetrievalFailed(
            "成绩表格列数不足".to_owned(),
        ));
    };
    Ok(CourseGrade {
        course_code: code.clone(),
        course_name: course.clone(),
        item_name: item.clone(),
        preview_grade: preview.parse().ok(),
        operation_grade: operation.parse().ok(),
        report_grade: report.parse().ok(),
        total_grade: total.parse().map_err(|_| {
            PhysicsExperimentError::CourseGradesRetrievalFailed("总成绩格式无效".to_owned())
        })?,
    })
}

fn cells_text(row: ElementRef<'_>) -> Vec<String> {
    row.child_elements()
        .filter(|cell| cell.value().name() == "td")
        .map(|cell| cell.text().collect::<String>().trim().to_owned())
        .collect()
}

fn parse_time(
    text: &str,
) -> Result<(DateTime<FixedOffset>, DateTime<FixedOffset>), PhysicsExperimentError> {
    let invalid = || schedule_error(format!("时间格式无效: {text}"));
    let (start, end) = text.split_once(" - ").ok_or_else(invalid)?;
    let start = NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M").map_err(|_| invalid())?;
    let end = NaiveTime::parse_from_str(end, "%H:%M").map_err(|_| invalid())?;
    let end = start.date().and_time(end);
    let start = start
        .and_local_timezone(SCHOOL_OFFSET)
        .single()
        .ok_or_else(invalid)?;
    let end = end
        .and_local_timezone(SCHOOL_OFFSET)
        .single()
        .ok_or_else(invalid)?;
    Ok((start, end))
}

fn parse_week(text: &str) -> Result<(i64, DayOfWeek), PhysicsExperimentError> {
    let mut parts = text.split_whitespace();
    let (Some(week), Some(day), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(schedule_error(format!("周信息格式无效: {text}")));
    };
    let week = week
        .strip_prefix('第')
        .and_then(|week| week.strip_suffix('周'))
        .and_then(|week| week.parse().ok())
        .ok_or_else(|| schedule_error(format!("无法解析周次: {week}")))?;
    let day = DayOfWeek::from_chinese_name(day)
        .ok_or_else(|| schedule_error(format!("无法解析星期: {day}")))?;
    Ok((week, day))
}

fn schedule_error(message: impl Into<String>) -> PhysicsExperimentError {
    PhysicsExperimentError::SchedulesRetrievalFailed(message.into())
}
