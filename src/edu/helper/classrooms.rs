use std::{collections::HashSet, ops::RangeInclusive};

use reqwest::header::CONTENT_TYPE;
use scraper::{ElementRef, Html, Selector, node::Node};

use crate::{
    Campus, DayOfWeek,
    url_factory::{ServiceDomain, make_url},
};

use super::super::{ClassroomScheduleQuery, ClassroomScheduleSession, EduError};
use super::{
    EduHelper, element_text,
    schedule::{ScheduleWeekType, parse_schedule_weeks},
};

const CLASSROOM_SCHEDULE_PATH: &str = "/jsxsd/kbcx/kbxx_classroom_ifr";

#[cfg_attr(feature = "uniffi", uniffi::export(async_runtime = "tokio"))]
impl EduHelper {
    /// 获取教室课表。指定周次时，返回的排课记录只包含范围内的周次。
    pub async fn get_classroom_schedule(
        &self,
        query: ClassroomScheduleQuery,
    ) -> Result<Vec<ClassroomScheduleSession>, EduError> {
        for (field, range) in [
            ("周次", query.weeks.as_ref()),
            ("星期", query.weekdays.as_ref()),
            ("节次", query.sections.as_ref()),
        ] {
            if range.is_some_and(RangeInclusive::is_empty) {
                return Err(classroom_error(format!("{field}范围的起点不能大于终点")));
            }
        }

        let campus_id = match query.campus {
            Some(Campus::Yuntang) => "1",
            Some(Campus::Jinpenling) => "2",
            None => "",
        };
        let (start_week, end_week) = query
            .weeks
            .as_ref()
            .map(|range| (range.start().to_string(), range.end().to_string()))
            .unwrap_or_default();
        let (start_day, end_day) = query
            .weekdays
            .as_ref()
            .map(|range| (*range.start(), *range.end()))
            .unwrap_or((1, 7));
        let (start_section, end_section) = query
            .sections
            .as_ref()
            .map(|range| {
                (
                    format!("{:02}", range.start()),
                    format!("{:02}", range.end()),
                )
            })
            .unwrap_or_default();
        let params = [
            (
                "xnxqh",
                query.academic_year_semester.as_deref().unwrap_or(""),
            ),
            ("skyx", ""),
            ("xqid", campus_id),
            ("jzwid", ""),
            ("gnq", ""),
            ("skjsid", ""),
            ("skjs", ""),
            ("zc1", start_week.as_str()),
            ("zc2", end_week.as_str()),
            ("skxq1", &start_day.to_string()),
            ("skxq2", &end_day.to_string()),
            ("jc1", start_section.as_str()),
            ("jc2", end_section.as_str()),
        ];
        let url = make_url(self.mode, ServiceDomain::Education, CLASSROOM_SCHEDULE_PATH);
        let response = self
            .request(|| {
                self.session
                    .client
                    .post(&url)
                    .header(
                        CONTENT_TYPE,
                        "application/x-www-form-urlencoded; charset=utf-8",
                    )
                    .form(&params)
            })
            .await?;
        parse_classroom_schedule(&response.text(), query.weeks.as_ref())
    }
}

struct ClassroomColumn {
    day_of_week: DayOfWeek,
    start_section: i64,
    end_section: i64,
}

fn parse_classroom_schedule(
    body: &str,
    weeks: Option<&RangeInclusive<i64>>,
) -> Result<Vec<ClassroomScheduleSession>, EduError> {
    let document = Html::parse_document(body);
    let row_selector = Selector::parse("#kbtable > thead > tr, #kbtable > tbody > tr")
        .map_err(|error| classroom_error(format!("解析教室课表行选择器失败: {error}")))?;
    let block_selector = Selector::parse("div.kbcontent1")
        .map_err(|error| classroom_error(format!("解析课程块选择器失败: {error}")))?;
    let rows = document.select(&row_selector).collect::<Vec<_>>();
    let [day_header, section_header, classroom_rows @ ..] = rows.as_slice() else {
        return Err(classroom_error("未找到教室课表表头"));
    };
    let columns = parse_columns(*day_header, *section_header)?;

    let mut schedule = Vec::new();
    for (index, row) in classroom_rows.iter().enumerate() {
        let mut cells = row.child_elements();
        let classroom = cells.next().map(element_text).unwrap_or_default();
        let cells = cells.collect::<Vec<_>>();
        if cells.len() != columns.len() || classroom.is_empty() {
            return Err(classroom_error(format!(
                "第 {} 行教室或节次列无效",
                index + 1
            )));
        }

        for (column, cell) in columns.iter().zip(cells) {
            let mut blocks = cell.select(&block_selector).peekable();
            if blocks.peek().is_none() && !element_text(cell).is_empty() {
                return Err(classroom_error(format!(
                    "教室 {classroom} 的非空课表格缺少课程块"
                )));
            }
            for block in blocks {
                let (course_label, mut course_weeks) = parse_classroom_block(block, &classroom)?;
                if let Some(weeks) = weeks {
                    course_weeks.retain(|week| weeks.contains(week));
                }
                if course_weeks.is_empty() {
                    continue;
                }
                schedule.push(ClassroomScheduleSession {
                    classroom: classroom.clone(),
                    day_of_week: column.day_of_week,
                    start_section: column.start_section,
                    end_section: column.end_section,
                    weeks: course_weeks,
                    course_label,
                });
            }
        }
    }
    Ok(schedule)
}

fn parse_columns(
    day_header: ElementRef<'_>,
    section_header: ElementRef<'_>,
) -> Result<Vec<ClassroomColumn>, EduError> {
    let mut section_cells = section_header.child_elements();
    if section_cells.next().map(element_text).as_deref() != Some(r"教室\节次") {
        return Err(classroom_error("未找到节次表头"));
    }

    let mut columns = Vec::new();
    let mut seen_days = HashSet::new();
    for header in day_header.child_elements().skip(1) {
        let day = DayOfWeek::from_chinese_name(&element_text(header))
            .ok_or_else(|| classroom_error("星期表头无效"))?;
        if !seen_days.insert(day) {
            return Err(classroom_error("星期表头重复"));
        }
        let span = header
            .value()
            .attr("colspan")
            .unwrap_or("1")
            .parse::<usize>()
            .map_err(|_| classroom_error("星期表头跨列数无效"))?;
        for _ in 0..span {
            let cell = section_cells
                .next()
                .ok_or_else(|| classroom_error("节次表头列数不足"))?;
            let label = element_text(cell);
            let invalid_label = || classroom_error(format!("节次表头无效: {label}"));
            if label.len() != 4 || !label.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid_label());
            }
            let (start, end) = label.split_at(2);
            let start = start.parse::<i64>().map_err(|_| invalid_label())?;
            let end = end.parse::<i64>().map_err(|_| invalid_label())?;
            if start == 0 || start > end {
                return Err(invalid_label());
            }
            columns.push(ClassroomColumn {
                day_of_week: day,
                start_section: start,
                end_section: end,
            });
        }
    }
    if seen_days.len() != DayOfWeek::ALL.len() || section_cells.next().is_some() {
        return Err(classroom_error("星期与节次表头不匹配"));
    }
    Ok(columns)
}

fn parse_classroom_block(
    block: ElementRef<'_>,
    classroom: &str,
) -> Result<(String, Vec<i64>), EduError> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for child in block.children() {
        match child.value() {
            Node::Text(text) => line.push_str(&text.text),
            Node::Element(element) if element.name() == "br" => {
                lines.push(std::mem::take(&mut line));
            }
            Node::Element(_) => {
                if let Some(element) = ElementRef::wrap(child) {
                    line.push_str(&element_text(element));
                }
            }
            _ => {}
        }
    }
    lines.push(line);
    let lines = lines
        .iter()
        .take(2)
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>();

    let invalid_block = || classroom_error(format!("无法解析教室 {classroom} 的课程标签或周次"));
    // 多条排课共用一个课表格时，标签和周次可能分属两行。
    let mut date_lines = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.ends_with("周)") || line.ends_with("周）"));
    let (Some((index, line)), None) = (date_lines.next(), date_lines.next()) else {
        return Err(invalid_block());
    };
    let (prefix, date) = line.rsplit_once(['(', '（']).ok_or_else(invalid_block)?;
    let date = date.strip_suffix([')', '）']).ok_or_else(invalid_block)?;
    let label = if index == 0 { prefix.trim() } else { &lines[0] };
    if label.is_empty() {
        return Err(invalid_block());
    }

    let (week_text, week_type) = [
        ("单周", ScheduleWeekType::Single),
        ("双周", ScheduleWeekType::Double),
        ("周", ScheduleWeekType::All),
    ]
    .into_iter()
    .find_map(|(suffix, week_type)| date.strip_suffix(suffix).map(|text| (text, week_type)))
    .ok_or_else(invalid_block)?;
    let mut weeks =
        parse_schedule_weeks(week_text.trim(), week_type).map_err(|_| invalid_block())?;
    weeks.retain(|week| week_type.includes(*week));
    weeks.sort_unstable();
    weeks.dedup();
    if weeks.first().is_none_or(|week| *week < 1) {
        return Err(invalid_block());
    }
    Ok((label.to_owned(), weeks))
}

fn classroom_error(message: impl Into<String>) -> EduError {
    EduError::ClassroomScheduleRetrievalFailed(message.into())
}
