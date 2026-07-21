use std::collections::HashMap;

use reqwest::header::CONTENT_TYPE;
use scraper::{ElementRef, Html, Selector, node::Node};
use url::form_urlencoded;

use crate::url_factory::{ServiceDomain, make_url};

use super::super::{
    Course, CourseSchedule, DayOfWeek, ScheduleSession, SemesterOptions, types::EduError,
};
use super::{EduHelper, element_text, ensure_logged_in};

const COURSE_SCHEDULE_PATH: &str = "/jsxsd/xskb/xskb_list.do";

impl EduHelper {
    pub async fn get_available_semesters_for_course_schedule(
        &self,
    ) -> Result<SemesterOptions, EduError> {
        let url = make_url(self.mode, ServiceDomain::Education, COURSE_SCHEDULE_PATH);
        let response = self
            .session
            .send_with_retry(|| self.session.client.get(&url))
            .await?;
        let body = String::from_utf8_lossy(&response.body);

        ensure_logged_in(&body)?;
        parse_course_schedule_semesters(&body)
    }

    pub async fn get_course_schedule(
        &self,
        academic_year_semester: Option<String>,
    ) -> Result<CourseSchedule, EduError> {
        let url = make_url(self.mode, ServiceDomain::Education, COURSE_SCHEDULE_PATH);
        let mut form = form_urlencoded::Serializer::new(String::new());
        form.append_pair("xnxq01id", academic_year_semester.as_deref().unwrap_or(""));
        let form = form.finish();
        let response = self.session.send_with_retry(|| {
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
        parse_course_schedule(&body)
    }
}

fn parse_course_schedule_semesters(body: &str) -> Result<SemesterOptions, EduError> {
    let document = Html::parse_document(body);
    let select_selector = Selector::parse("#xnxq01id")
        .map_err(|error| schedule_semesters_error(format!("解析课表学期选择器失败: {error}")))?;
    let semester_select = document
        .select(&select_selector)
        .next()
        .ok_or_else(|| schedule_semesters_error("未找到学期选择元素"))?;
    let option_selector = Selector::parse("option")
        .map_err(|error| schedule_semesters_error(format!("解析课表学期选项失败: {error}")))?;
    let options = semester_select.select(&option_selector).collect::<Vec<_>>();
    if options.is_empty() {
        return Err(schedule_semesters_error("学期选择元素中未找到学期"));
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
        default_semester.ok_or_else(|| schedule_semesters_error("未找到默认学期"))?;

    Ok(SemesterOptions {
        semesters,
        default_semester,
    })
}

struct ParsedScheduleItem {
    course_name: String,
    group_name: Option<String>,
    teacher: Option<String>,
    weeks: Vec<i64>,
    start_section: i64,
    end_section: i64,
    classroom: Option<String>,
}

fn parse_course_schedule(body: &str) -> Result<CourseSchedule, EduError> {
    let document = Html::parse_document(body);
    let table_selector = Selector::parse("#kbtable")
        .map_err(|error| schedule_error(format!("解析课程表格选择器失败: {error}")))?;
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| schedule_error("未找到课程表"))?;
    let row_selector = Selector::parse("tr")
        .map_err(|error| schedule_error(format!("解析课程表行选择器失败: {error}")))?;
    let cell_selector = Selector::parse("td")
        .map_err(|error| schedule_error(format!("解析课程表列选择器失败: {error}")))?;
    let header_selector = Selector::parse("th")
        .map_err(|error| schedule_error(format!("解析课程表备注表头失败: {error}")))?;
    let rows = table.select(&row_selector).collect::<Vec<_>>();
    let mut remarks = Vec::new();
    if let Some(last_row) = rows.last() {
        let has_remarks_header = last_row
            .select(&header_selector)
            .next()
            .map(|header| element_text(header).contains("备注"))
            .unwrap_or(false);
        if has_remarks_header {
            remarks = last_row
                .select(&cell_selector)
                .next()
                .map(element_text)
                .unwrap_or_default()
                .split(';')
                .map(str::trim)
                .filter(|remark| !remark.is_empty())
                .map(str::to_owned)
                .collect();
        }
    }

    let mut courses = HashMap::<String, Course>::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row_index == 0 || row_index + 1 == rows.len() {
            continue;
        }
        for (column_index, cell) in row.select(&cell_selector).enumerate() {
            let day_of_week = DayOfWeek::from_column(column_index)
                .ok_or_else(|| schedule_error(format!("星期索引无效: {column_index}")))?;
            for item in parse_schedule_cell(cell)? {
                let session = ScheduleSession {
                    weeks: item.weeks,
                    start_section: item.start_section,
                    end_section: item.end_section,
                    day_of_week,
                    classroom: item.classroom,
                };
                let course = courses
                    .entry(item.course_name.clone())
                    .or_insert_with(|| Course {
                        course_name: item.course_name,
                        group_name: item.group_name,
                        teacher: item.teacher,
                        sessions: Vec::new(),
                    });
                if !course.sessions.contains(&session) {
                    course.sessions.push(session);
                }
            }
        }
    }

    Ok(CourseSchedule {
        courses: courses.into_values().collect(),
        remarks,
    })
}

fn parse_schedule_cell(element: ElementRef<'_>) -> Result<Vec<ParsedScheduleItem>, EduError> {
    if element_text(element).is_empty() {
        return Ok(Vec::new());
    }
    let content_selector = Selector::parse("div.kbcontent")
        .map_err(|error| schedule_error(format!("解析课程内容选择器失败: {error}")))?;
    let content = element
        .select(&content_selector)
        .next()
        .ok_or_else(|| schedule_error("未找到课程内容元素"))?;
    let teacher_selector = Selector::parse("font[title='老师']")
        .map_err(|error| schedule_error(format!("解析课程教师选择器失败: {error}")))?;
    let classroom_selector = Selector::parse("font[title='教室']")
        .map_err(|error| schedule_error(format!("解析课程教室选择器失败: {error}")))?;
    let date_selector = Selector::parse("font[title='周次(节次)']")
        .map_err(|error| schedule_error(format!("解析课程日期选择器失败: {error}")))?;
    let body_selector = Selector::parse("body")
        .map_err(|error| schedule_error(format!("解析课程片段主体选择器失败: {error}")))?;
    let mut items = Vec::new();

    for course_html in content.inner_html().split("---------------------") {
        let course_html = course_html.trim();
        if course_html.is_empty() {
            continue;
        }
        let fragment_html = format!("<html><body>{course_html}</body></html>");
        let fragment = Html::parse_document(&fragment_html);
        let course_body = fragment
            .select(&body_selector)
            .next()
            .ok_or_else(|| schedule_error("未找到课程主体"))?;
        let text_nodes = direct_text_nodes(course_body);
        let course_name = text_nodes
            .first()
            .map(|text| text.trim().to_owned())
            .ok_or_else(|| schedule_error("未找到课程名称"))?;
        let group_name = text_nodes
            .get(1)
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
        let teacher = course_body
            .select(&teacher_selector)
            .next()
            .map(element_text);
        let classroom = course_body
            .select(&classroom_selector)
            .next()
            .map(element_text);
        let date = course_body
            .select(&date_selector)
            .next()
            .map(element_text)
            .ok_or_else(|| schedule_error("未找到日期文本"))?;
        let (weeks, sections) = parse_schedule_date(&date)?;
        if weeks.is_empty() || sections.is_empty() {
            return Err(schedule_error("周或节次无效"));
        }
        let start_section = sections
            .first()
            .copied()
            .ok_or_else(|| schedule_error("节次范围无效"))?;
        let end_section = sections
            .last()
            .copied()
            .ok_or_else(|| schedule_error("节次范围无效"))?;
        items.push(ParsedScheduleItem {
            course_name,
            group_name,
            teacher,
            weeks,
            start_section,
            end_section,
            classroom,
        });
    }
    Ok(items)
}

#[derive(Clone, Copy)]
enum ScheduleWeekType {
    Single,
    Double,
    All,
}

fn parse_schedule_date(value: &str) -> Result<(Vec<i64>, Vec<i64>), EduError> {
    let (marker, week_type) = if value.contains("(单周)") {
        ("(单周)", ScheduleWeekType::Single)
    } else if value.contains("(双周)") {
        ("(双周)", ScheduleWeekType::Double)
    } else if value.contains("(周)") {
        ("(周)", ScheduleWeekType::All)
    } else {
        return Err(schedule_error(format!("日期中周类型无效: {value}")));
    };
    let parts = value.split(marker).collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(schedule_error(format!("日期格式无效: {value}")));
    }
    let week_part = parts[0];
    let section_part = parts[1].trim_matches(|character| matches!(character, '[' | ']' | '节'));
    let mut weeks = Vec::new();
    let mut sections = Vec::new();

    for week_section in week_part.split(',') {
        if week_section.contains('-') {
            let range_parts = week_section.split('-').collect::<Vec<_>>();
            if range_parts.len() != 2 {
                return Err(schedule_error(format!("周范围格式无效: {week_section}")));
            }
            let start_week = range_parts[0]
                .trim()
                .parse::<i64>()
                .map_err(|_| schedule_error(format!("周范围格式无效: {week_section}")))?;
            let end_week = range_parts[1]
                .trim()
                .parse::<i64>()
                .map_err(|_| schedule_error(format!("周范围格式无效: {week_section}")))?;
            if start_week > end_week {
                return Err(schedule_error(format!(
                    "起始周 {start_week} 大于结束周 {end_week}"
                )));
            }
            weeks.extend((start_week..=end_week).filter(|week| match week_type {
                ScheduleWeekType::Single => week % 2 != 0,
                ScheduleWeekType::Double => week % 2 == 0,
                ScheduleWeekType::All => true,
            }));
        } else {
            let week = week_section
                .parse::<i64>()
                .map_err(|_| schedule_error(format!("周格式无效: {week_section}")))?;
            weeks.push(week);
        }
    }
    for section in section_part.split('-') {
        let section = section
            .parse::<i64>()
            .map_err(|_| schedule_error(format!("节次格式无效: {section}")))?;
        sections.push(section);
    }
    Ok((weeks, sections))
}

fn direct_text_nodes(element: ElementRef<'_>) -> Vec<String> {
    element
        .children()
        .filter_map(|node| match node.value() {
            Node::Text(text) => Some(text.text.to_string()),
            _ => None,
        })
        .collect()
}

fn schedule_semesters_error(message: impl Into<String>) -> EduError {
    EduError::AvailableSemestersForCourseScheduleRetrievalFailed(message.into())
}

fn schedule_error(message: impl Into<String>) -> EduError {
    EduError::CourseScheduleRetrievalFailed(message.into())
}
