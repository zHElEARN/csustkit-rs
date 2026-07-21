use reqwest::header::CONTENT_TYPE;
use scraper::{Html, Selector};
use url::form_urlencoded;

use crate::url_factory::{ServiceDomain, make_url};

use super::super::{
    AvailableClassroomsQuery, DayOfWeek, request::send_with_retry, types::EduError,
};
use super::{EduHelper, element_text, ensure_logged_in};

const AVAILABLE_CLASSROOMS_PATH: &str = "/jsxsd/kbcx/kbxx_classroom_ifr";

impl EduHelper {
    pub async fn get_available_classrooms(
        &self,
        query: AvailableClassroomsQuery,
    ) -> Result<Vec<String>, EduError> {
        let (start_section, end_section) = match query.section {
            1 => ("01", "02"),
            2 => ("03", "04"),
            3 => ("05", "06"),
            4 => ("07", "08"),
            5 => ("09", "10"),
            _ => return Err(available_classrooms_error("节次范围错误：1-5")),
        };
        let day_of_week = match query.day_of_week {
            DayOfWeek::Sunday => 7,
            day => day as u8,
        }
        .to_string();
        let week = query.week.to_string();
        let url = make_url(
            self.mode,
            ServiceDomain::Education,
            AVAILABLE_CLASSROOMS_PATH,
        );
        let mut form = form_urlencoded::Serializer::new(String::new());
        form.append_pair("skyx", "");
        form.append_pair("xqid", query.campus.request_id());
        form.append_pair("jzwid", "");
        form.append_pair("gnq", "");
        form.append_pair("skjsid", "");
        form.append_pair("skjs", "");
        form.append_pair("zc1", &week);
        form.append_pair("zc2", &week);
        form.append_pair("skxq1", &day_of_week);
        form.append_pair("skxq2", &day_of_week);
        form.append_pair("jc1", start_section);
        form.append_pair("jc2", end_section);
        let form = form.finish();
        let response = send_with_retry(|| {
            self.session
                .client
                .post(&url)
                .header(
                    CONTENT_TYPE,
                    "application/x-www-form-urlencoded; charset=utf-8",
                )
                .body(form.clone())
        })
        .await?;
        let body = String::from_utf8_lossy(&response.body);

        ensure_logged_in(&body)?;
        parse_available_classrooms(&body)
    }
}

fn parse_available_classrooms(body: &str) -> Result<Vec<String>, EduError> {
    let document = Html::parse_document(body);
    let table_selector = Selector::parse("#kbtable")
        .map_err(|error| course_schedule_error(format!("解析课程表格选择器失败: {error}")))?;
    document
        .select(&table_selector)
        .next()
        .ok_or_else(|| course_schedule_error("未找到课程表"))?;
    let row_selector = Selector::parse("#kbtable > tbody > tr")
        .map_err(|error| available_classrooms_error(format!("解析教室行选择器失败: {error}")))?;
    let cell_selector = Selector::parse("td")
        .map_err(|error| available_classrooms_error(format!("解析教室列选择器失败: {error}")))?;
    let course_selector = Selector::parse("div.kbcontent1")
        .map_err(|error| available_classrooms_error(format!("解析课程内容选择器失败: {error}")))?;
    let mut available_classrooms = Vec::new();

    for row in document.select(&row_selector) {
        let cells = row.select(&cell_selector).collect::<Vec<_>>();
        let Some(first_cell) = cells.first().copied() else {
            continue;
        };
        let classroom = element_text(first_cell);
        let occupied = cells.iter().skip(1).any(|cell| {
            cell.select(&course_selector).next().is_some() || !element_text(*cell).is_empty()
        });
        if !occupied {
            available_classrooms.push(classroom);
        }
    }

    Ok(available_classrooms)
}

fn available_classrooms_error(message: impl Into<String>) -> EduError {
    EduError::AvailableClassroomsRetrievalFailed(message.into())
}

fn course_schedule_error(message: impl Into<String>) -> EduError {
    EduError::CourseScheduleRetrievalFailed(message.into())
}
