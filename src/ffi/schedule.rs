use std::{collections::HashMap, ptr};

use chrono::{NaiveDate, NaiveTime};

use super::{OffsetDateTime, UtcDateTime};
use crate::{
    edu::{Course, ScheduleSession},
    utils::schedule::{self, CourseOccurrence as BorrowedCourseOccurrence},
};

/// 一天内的上下课时间，使用长沙当地时间 HH:mm。
#[derive(uniffi::Record)]
pub struct SectionTime {
    pub start: String,
    pub end: String,
}

/// 某个上课时段在原课程数组中的位置。两个索引都从 0 开始。
#[derive(uniffi::Record)]
pub struct CourseOccurrence {
    pub course_index: u64,
    pub session_index: u64,
}

#[derive(uniffi::Record)]
pub struct UnfinishedCourse {
    pub occurrence: CourseOccurrence,
    pub is_current: bool,
}

/// 半开日期区间，包含 start，排除 end。
#[derive(uniffi::Record)]
pub struct CalendarDateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// 半开时间区间，包含 start，排除 end。
#[derive(uniffi::Record)]
pub struct TimeRange {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

#[uniffi::export]
fn default_week_count() -> i64 {
    schedule::DEFAULT_WEEK_COUNT
}

/// 返回学校各节次的上下课时间，下标 0 对应第 1 节。
#[uniffi::export]
fn section_times() -> Vec<SectionTime> {
    schedule::SECTION_TIMES
        .iter()
        .map(|(start, end)| SectionTime {
            start: start.format("%H:%M").to_string(),
            end: end.format("%H:%M").to_string(),
        })
        .collect()
}

/// 将给定自然日转换为长沙同一自然日的零点。
#[uniffi::export]
fn normalize_semester_start_date(date: NaiveDate) -> Option<OffsetDateTime> {
    schedule::normalize_semester_start_date(&date.and_time(NaiveTime::MIN).and_utc())
}

/// 从学期开始日计算半开日期区间。
#[uniffi::export]
fn semester_date_range(semester_start_date: NaiveDate, week_count: i64) -> CalendarDateRange {
    let range = schedule::semester_date_range(semester_start_date, week_count);
    CalendarDateRange {
        start: range.start,
        end: range.end,
    }
}

/// 判断给定时刻在长沙是否属于今天。
#[uniffi::export]
fn is_today(date: UtcDateTime) -> bool {
    schedule::is_today(&date)
}

/// 将课程时段按周次分组。
#[uniffi::export]
fn weekly_courses(courses: &[Course]) -> HashMap<i64, Vec<CourseOccurrence>> {
    let indices = course_indices(courses);
    schedule::weekly_courses(courses)
        .into_iter()
        .map(|(week, occurrences)| {
            let occurrences = occurrences
                .into_iter()
                .map(|occurrence| indexed_occurrence(occurrence, &indices))
                .collect();
            (week, occurrences)
        })
        .collect()
}

/// 返回指定周中某个上课时段的具体起止时间。
#[uniffi::export]
fn time_range(
    session: &ScheduleSession,
    week: i64,
    semester_start_date: NaiveDate,
) -> Option<TimeRange> {
    schedule::time_range(session, week, semester_start_date).map(|range| TimeRange {
        start: range.start,
        end: range.end,
    })
}

/// 返回给定长沙当天的课程时段，按起始节次排序。
#[uniffi::export]
fn courses_on(
    date: NaiveDate,
    semester_start_date: NaiveDate,
    courses: &[Course],
    week_count: i64,
) -> Vec<CourseOccurrence> {
    let indices = course_indices(courses);
    schedule::courses_on(date, semester_start_date, courses, week_count)
        .into_iter()
        .map(|occurrence| indexed_occurrence(occurrence, &indices))
        .collect()
}

/// 返回当前或当日尚未开始的课程。
#[uniffi::export]
fn unfinished_courses(
    now: UtcDateTime,
    semester_start_date: NaiveDate,
    courses: &[Course],
    week_count: i64,
) -> Vec<UnfinishedCourse> {
    let indices = course_indices(courses);
    schedule::unfinished_courses(&now, semester_start_date, courses, week_count)
        .into_iter()
        .map(|course| UnfinishedCourse {
            occurrence: indexed_occurrence(course.occurrence, &indices),
            is_current: course.is_current,
        })
        .collect()
}

/// 依次选取正在上的课、20 分钟内即将开始的课、5 分钟内刚下课的课。
#[uniffi::export]
fn relevant_course(
    now: UtcDateTime,
    semester_start_date: NaiveDate,
    courses: &[Course],
    week_count: i64,
) -> Option<CourseOccurrence> {
    let occurrence = schedule::relevant_course(&now, semester_start_date, courses, week_count)?;
    Some(indexed_occurrence(occurrence, &course_indices(courses)))
}

fn course_indices(courses: &[Course]) -> HashMap<*const Course, u64> {
    courses
        .iter()
        .enumerate()
        .map(|(index, course)| (ptr::from_ref(course), index as u64))
        .collect()
}

fn indexed_occurrence(
    occurrence: BorrowedCourseOccurrence<'_>,
    indices: &HashMap<*const Course, u64>,
) -> CourseOccurrence {
    CourseOccurrence {
        course_index: indices[&ptr::from_ref(occurrence.course())],
        session_index: occurrence.session_index() as u64,
    }
}
