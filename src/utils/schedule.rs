//! 课表日期、节次与课程时段计算。

use std::{collections::BTreeMap, ops::Range, time::SystemTime};

use chrono::{
    DateTime, Datelike, Days, FixedOffset, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc, Weekday,
};

use crate::{
    DayOfWeek,
    edu::{Course, ScheduleSession},
};

const SCHOOL_OFFSET: FixedOffset = FixedOffset::east_opt(8 * 60 * 60).unwrap();
const UPCOMING_THRESHOLD: TimeDelta = TimeDelta::minutes(20);
const RECENTLY_FINISHED_THRESHOLD: TimeDelta = TimeDelta::minutes(5);

/// 学校课表的默认学期总周数。
pub const DEFAULT_WEEK_COUNT: i64 = 20;

/// 长沙当地的上下课时间，下标 0 对应第 1 节。
pub const SECTION_TIMES: [(NaiveTime, NaiveTime); 10] = [
    (clock_time(8, 0), clock_time(8, 45)),
    (clock_time(8, 55), clock_time(9, 40)),
    (clock_time(10, 10), clock_time(10, 55)),
    (clock_time(11, 5), clock_time(11, 50)),
    (clock_time(14, 0), clock_time(14, 45)),
    (clock_time(14, 55), clock_time(15, 40)),
    (clock_time(16, 10), clock_time(16, 55)),
    (clock_time(17, 5), clock_time(17, 50)),
    (clock_time(19, 30), clock_time(20, 15)),
    (clock_time(20, 25), clock_time(21, 10)),
];

const fn clock_time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemesterStatus {
    BeforeSemester { days_until_start: i64 },
    InSemester { week: i64 },
    AfterSemester,
}

/// 课程中的一个上课时段。借用调用方的课程数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CourseOccurrence<'a> {
    course: &'a Course,
    session_index: usize,
}

impl<'a> CourseOccurrence<'a> {
    pub fn course(&self) -> &'a Course {
        self.course
    }

    pub fn session_index(&self) -> usize {
        self.session_index
    }

    pub fn session(&self) -> &'a ScheduleSession {
        &self.course.sessions[self.session_index]
    }
}

/// 尚未下课的课程时段及其当前状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnfinishedCourse<'a> {
    pub occurrence: CourseOccurrence<'a>,
    pub is_current: bool,
}

/// 按输入时区的自然日取出日期，转换为长沙同一自然日的零点。
/// 教务接口直接返回的 `NaiveDate` 可用作各计算函数的学期开始日。
pub fn normalize_semester_start_date<Tz: TimeZone>(
    date: &DateTime<Tz>,
) -> Option<DateTime<FixedOffset>> {
    date.date_naive()
        .and_time(NaiveTime::MIN)
        .and_local_timezone(SCHOOL_OFFSET)
        .single()
}

/// 学期覆盖的半开区间，结束日为开始日加总周数。
pub fn semester_date_range(semester_start_date: NaiveDate, week_count: i64) -> Range<NaiveDate> {
    let end = TimeDelta::try_weeks(week_count)
        .and_then(|span| semester_start_date.checked_add_signed(span))
        .unwrap_or(semester_start_date);
    semester_start_date..end
}

/// 返回指定周的日期。第 1 周从学期开始日算起，学校课表以周日为周首。
pub fn dates_of_week(week: i64, semester_start_date: NaiveDate) -> Vec<NaiveDate> {
    let Some(first) = week_start(week, semester_start_date) else {
        return Vec::new();
    };
    (0..7)
        .filter_map(|day| first.checked_add_days(Days::new(day)))
        .collect()
}

pub fn day_of_week(date: NaiveDate) -> DayOfWeek {
    match date.weekday() {
        Weekday::Sun => DayOfWeek::Sunday,
        Weekday::Mon => DayOfWeek::Monday,
        Weekday::Tue => DayOfWeek::Tuesday,
        Weekday::Wed => DayOfWeek::Wednesday,
        Weekday::Thu => DayOfWeek::Thursday,
        Weekday::Fri => DayOfWeek::Friday,
        Weekday::Sat => DayOfWeek::Saturday,
    }
}

/// 根据长沙当地日期判断学期状态。
pub fn semester_status(
    semester_start_date: NaiveDate,
    date: NaiveDate,
    week_count: i64,
) -> SemesterStatus {
    let days_from_start = (date - semester_start_date).num_days();
    if days_from_start < 0 {
        SemesterStatus::BeforeSemester {
            days_until_start: -days_from_start,
        }
    } else if semester_date_range(semester_start_date, week_count).contains(&date) {
        SemesterStatus::InSemester {
            week: days_from_start / 7 + 1,
        }
    } else {
        SemesterStatus::AfterSemester
    }
}

/// 判断给定时刻在长沙是否属于今天。
pub fn is_today<Tz: TimeZone>(date: &DateTime<Tz>) -> bool {
    let now = DateTime::<Utc>::from(SystemTime::now());
    date.with_timezone(&SCHOOL_OFFSET).date_naive()
        == now.with_timezone(&SCHOOL_OFFSET).date_naive()
}

/// 将每门课程的上课时段归入对应的周次。
pub fn weekly_courses(courses: &[Course]) -> BTreeMap<i64, Vec<CourseOccurrence<'_>>> {
    let mut weeks = BTreeMap::<i64, Vec<CourseOccurrence<'_>>>::new();
    for course in courses {
        for (session_index, session) in course.sessions.iter().enumerate() {
            let occurrence = CourseOccurrence {
                course,
                session_index,
            };
            for &week in &session.weeks {
                weeks.entry(week).or_default().push(occurrence);
            }
        }
    }
    weeks
}

/// 返回指定周中某个上课时段的具体起止时间。节次无效时返回 `None`。
pub fn time_range(
    session: &ScheduleSession,
    week: i64,
    semester_start_date: NaiveDate,
) -> Option<Range<DateTime<FixedOffset>>> {
    let day = week_start(week, semester_start_date)?
        .checked_add_days(Days::new(session.day_of_week as u64))?;
    time_range_on(session, day)
}

/// 返回给定长沙当天的课程时段，按起始节次排序。
pub fn courses_on(
    date: NaiveDate,
    semester_start_date: NaiveDate,
    courses: &[Course],
    week_count: i64,
) -> Vec<CourseOccurrence<'_>> {
    let SemesterStatus::InSemester { week } =
        semester_status(semester_start_date, date, week_count)
    else {
        return Vec::new();
    };
    let weekday = day_of_week(date);
    let mut occurrences = Vec::new();
    for course in courses {
        for (session_index, session) in course.sessions.iter().enumerate() {
            if session.day_of_week == weekday && session.weeks.contains(&week) {
                occurrences.push(CourseOccurrence {
                    course,
                    session_index,
                });
            }
        }
    }
    occurrences.sort_by_key(|occurrence| occurrence.session().start_section);
    occurrences
}

/// 返回当前或当日尚未开始的课程。
pub fn unfinished_courses<'a, Tz: TimeZone>(
    now: &DateTime<Tz>,
    semester_start_date: NaiveDate,
    courses: &'a [Course],
    week_count: i64,
) -> Vec<UnfinishedCourse<'a>> {
    let now = now.with_timezone(&SCHOOL_OFFSET);
    let day = now.date_naive();
    courses_on(day, semester_start_date, courses, week_count)
        .into_iter()
        .filter_map(|occurrence| {
            let range = time_range_on(occurrence.session(), day)?;
            (now < range.end).then_some(UnfinishedCourse {
                occurrence,
                is_current: now >= range.start,
            })
        })
        .collect()
}

/// 依次选取正在上的课、20 分钟内即将开始的课、5 分钟内刚下课的课。
pub fn relevant_course<'a, Tz: TimeZone>(
    now: &DateTime<Tz>,
    semester_start_date: NaiveDate,
    courses: &'a [Course],
    week_count: i64,
) -> Option<CourseOccurrence<'a>> {
    let now = now.with_timezone(&SCHOOL_OFFSET);
    let day = now.date_naive();
    let mut upcoming = None;
    let mut finished = None;

    for occurrence in courses_on(day, semester_start_date, courses, week_count) {
        let Some(range) = time_range_on(occurrence.session(), day) else {
            continue;
        };
        if range.contains(&now) {
            return Some(occurrence);
        }
        let (nearest, interval) = if now < range.start {
            (&mut upcoming, range.start - now)
        } else {
            (&mut finished, now - range.end)
        };
        if nearest.is_none_or(|(_, previous)| interval < previous) {
            *nearest = Some((occurrence, interval));
        }
    }

    if let Some((course, _)) = upcoming.filter(|(_, interval)| *interval <= UPCOMING_THRESHOLD) {
        return Some(course);
    }
    finished
        .filter(|(_, interval)| *interval <= RECENTLY_FINISHED_THRESHOLD)
        .map(|(course, _)| course)
}

fn week_start(week: i64, semester_start_date: NaiveDate) -> Option<NaiveDate> {
    let offset = u64::try_from(week.checked_sub(1)?).ok()?.checked_mul(7)?;
    semester_start_date.checked_add_days(Days::new(offset))
}

fn time_range_on(
    session: &ScheduleSession,
    date: NaiveDate,
) -> Option<Range<DateTime<FixedOffset>>> {
    if session.start_section < 1 || session.end_section < session.start_section {
        return None;
    }
    let start_index = usize::try_from(session.start_section - 1).ok()?;
    let end_index = usize::try_from(session.end_section - 1).ok()?;
    let (start, _) = *SECTION_TIMES.get(start_index)?;
    let (_, end) = *SECTION_TIMES.get(end_index)?;
    let start = date
        .and_time(start)
        .and_local_timezone(SCHOOL_OFFSET)
        .single()?;
    let end = date
        .and_time(end)
        .and_local_timezone(SCHOOL_OFFSET)
        .single()?;
    Some(start..end)
}
