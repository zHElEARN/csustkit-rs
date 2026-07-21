use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
use reqwest::header::CONTENT_TYPE;
use scraper::{ElementRef, Html, Selector};
use url::form_urlencoded;

use crate::url_factory::{ServiceDomain, make_url};

use super::super::{
    Exam, ExamScheduleQuery, SemesterOptions, SemesterType, request::send_with_retry,
    types::EduError,
};
use super::{EduHelper, element_text, ensure_logged_in};

const EXAM_SCHEDULE_PATH: &str = "/jsxsd/xsks/xsksap_list";
const EXAM_SCHEDULE_SEMESTERS_PATH: &str = "/jsxsd/xsks/xsksap_query";

impl EduHelper {
    pub async fn get_available_semesters_for_exam_schedule(
        &self,
    ) -> Result<SemesterOptions, EduError> {
        let url = make_url(
            self.mode,
            ServiceDomain::Education,
            EXAM_SCHEDULE_SEMESTERS_PATH,
        );
        let response = send_with_retry(|| self.session.client.get(&url)).await?;
        let body = String::from_utf8_lossy(&response.body);

        ensure_logged_in(&body)?;
        parse_exam_schedule_semesters(&body)
    }

    pub async fn get_exam_schedule(&self, query: ExamScheduleQuery) -> Result<Vec<Exam>, EduError> {
        let academic_year_semester = if let Some(value) = query.academic_year_semester {
            value
        } else {
            self.get_available_semesters_for_exam_schedule()
                .await?
                .default_semester
        };
        let semester_type = query.semester_type;
        let url = make_url(self.mode, ServiceDomain::Education, EXAM_SCHEDULE_PATH);
        let mut form = form_urlencoded::Serializer::new(String::new());
        form.append_pair(
            "xqlb",
            semester_type.map(SemesterType::request_id).unwrap_or(""),
        );
        form.append_pair(
            "xqlbmc",
            semester_type.map(SemesterType::display_name).unwrap_or(""),
        );
        form.append_pair("xnxqid", &academic_year_semester);
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
        parse_exam_schedule(&body)
    }
}

fn parse_exam_schedule_semesters(body: &str) -> Result<SemesterOptions, EduError> {
    let document = Html::parse_document(body);
    let select_selector = Selector::parse("#xnxqid")
        .map_err(|error| exam_semesters_error(format!("解析考试学期选择器失败: {error}")))?;
    let semester_select = document
        .select(&select_selector)
        .next()
        .ok_or_else(|| exam_semesters_error("未找到学期选择元素"))?;
    let option_selector = Selector::parse("option")
        .map_err(|error| exam_semesters_error(format!("解析考试学期选项失败: {error}")))?;
    let options = semester_select.select(&option_selector).collect::<Vec<_>>();
    if options.is_empty() {
        return Err(exam_semesters_error("学期选择元素中未找到学期"));
    }

    let mut semesters = Vec::with_capacity(options.len());
    let mut default_semester = None;
    for option in options {
        let semester = element_text(option);
        if option.value().attr("selected").is_some() {
            default_semester = Some(semester.clone());
        }
        semesters.push(semester);
    }
    let default_semester =
        default_semester.ok_or_else(|| exam_semesters_error("未找到默认学期"))?;

    Ok(SemesterOptions {
        semesters,
        default_semester,
    })
}

fn parse_exam_schedule(body: &str) -> Result<Vec<Exam>, EduError> {
    let document = Html::parse_document(body);
    let table_selector = Selector::parse("#dataList")
        .map_err(|error| exam_schedule_error(format!("解析考试安排表格选择器失败: {error}")))?;
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| exam_schedule_error("未找到考试安排表"))?;
    if table.inner_html().contains("未查询到数据") {
        return Ok(Vec::new());
    }

    let row_selector = Selector::parse("tr")
        .map_err(|error| exam_schedule_error(format!("解析考试安排行选择器失败: {error}")))?;
    let cell_selector = Selector::parse("td")
        .map_err(|error| exam_schedule_error(format!("解析考试安排列选择器失败: {error}")))?;
    let mut exams = Vec::new();
    for row in table.select(&row_selector).skip(1) {
        let cells = row.select(&cell_selector).collect::<Vec<_>>();
        if cells.len() < 11 {
            return Err(exam_schedule_error(format!("行列数不足: {}", cells.len())));
        }
        let exam_time = exam_cell(&cells, 6)?;
        let (exam_start_time, exam_end_time) = parse_exam_time(&exam_time)?;
        exams.push(Exam {
            campus: exam_cell(&cells, 1)?,
            session: exam_cell(&cells, 2)?,
            course_id: exam_cell(&cells, 3)?,
            course_name: exam_cell(&cells, 4)?,
            teacher: exam_cell(&cells, 5)?,
            exam_time,
            exam_start_time,
            exam_end_time,
            exam_room: exam_cell(&cells, 7)?,
            seat_number: exam_cell(&cells, 8)?,
            admission_ticket_number: exam_cell(&cells, 9)?,
            remarks: exam_cell(&cells, 10)?,
        });
    }
    Ok(exams)
}

fn parse_exam_time(
    value: &str,
) -> Result<(DateTime<FixedOffset>, DateTime<FixedOffset>), EduError> {
    let mut parts = value.split_whitespace();
    let date = match parts.next() {
        Some(date) => date,
        None => return Err(date_parsing_error(format!("日期字符串格式无效: {value}"))),
    };
    let time_range = match parts.next() {
        Some(time_range) => time_range,
        None => return Err(date_parsing_error(format!("日期字符串格式无效: {value}"))),
    };
    if parts.next().is_some() {
        return Err(date_parsing_error(format!("日期字符串格式无效: {value}")));
    }
    let mut times = time_range.split('~');
    let start = match times.next() {
        Some(start) => start,
        None => {
            return Err(date_parsing_error(format!(
                "日期字符串中的时间格式无效: {value}"
            )));
        }
    };
    let end = match times.next() {
        Some(end) => end,
        None => {
            return Err(date_parsing_error(format!(
                "日期字符串中的时间格式无效: {value}"
            )));
        }
    };
    if times.next().is_some() {
        return Err(date_parsing_error(format!(
            "日期字符串中的时间格式无效: {value}"
        )));
    }
    let offset = FixedOffset::east_opt(8 * 60 * 60)
        .ok_or_else(|| date_parsing_error("无法构建 Asia/Shanghai 时区"))?;
    let parse = |time: &str| {
        let text = format!("{date} {time}");
        let naive = NaiveDateTime::parse_from_str(&text, "%Y-%m-%d %H:%M")
            .map_err(|_| date_parsing_error(format!("无法从字符串解析日期: {value}")))?;
        offset
            .from_local_datetime(&naive)
            .single()
            .ok_or_else(|| date_parsing_error(format!("无法从字符串解析日期: {value}")))
    };
    Ok((parse(start)?, parse(end)?))
}

fn exam_semesters_error(message: impl Into<String>) -> EduError {
    EduError::AvailableSemestersForExamScheduleRetrievalFailed(message.into())
}

fn exam_schedule_error(message: impl Into<String>) -> EduError {
    EduError::ExamScheduleRetrievalFailed(message.into())
}

fn date_parsing_error(message: impl Into<String>) -> EduError {
    EduError::DateParsingFailed(message.into())
}

fn exam_cell(cells: &[ElementRef<'_>], index: usize) -> Result<String, EduError> {
    cells
        .get(index)
        .copied()
        .map(element_text)
        .ok_or_else(|| exam_schedule_error(format!("列索引越界: {index}")))
}
