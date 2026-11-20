use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use csustkit::{
    DayOfWeek,
    edu::{Course, ScheduleSession},
    utils::schedule::{
        CourseOccurrence, DEFAULT_WEEK_COUNT, SemesterStatus, courses_on, dates_of_week,
        day_of_week, is_today, normalize_semester_start_date, relevant_course, semester_date_range,
        semester_status, time_range, unfinished_courses, weekly_courses,
    },
};

const SEMESTER_START: NaiveDate = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn shanghai(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<FixedOffset> {
    FixedOffset::east_opt(28_800)
        .unwrap()
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .unwrap()
}

fn course(name: &str, weeks: &[i64], day: DayOfWeek, start: i64, end: i64) -> Course {
    Course {
        course_name: name.to_owned(),
        group_name: None,
        teacher: None,
        sessions: vec![ScheduleSession {
            weeks: weeks.to_vec(),
            start_section: start,
            end_section: end,
            day_of_week: day,
            classroom: Some("A101".to_owned()),
        }],
    }
}

fn names<'a>(occurrences: &[CourseOccurrence<'a>]) -> Vec<&'a str> {
    occurrences
        .iter()
        .map(|occurrence| occurrence.course().course_name.as_str())
        .collect()
}

#[test]
fn semester_status_reports_week_within_semester() {
    for (day, week) in [
        (date(2026, 9, 6), 1),
        (date(2026, 9, 12), 1),
        (date(2026, 9, 13), 2),
        (date(2027, 1, 23), 20),
    ] {
        assert_eq!(
            semester_status(SEMESTER_START, day, DEFAULT_WEEK_COUNT),
            SemesterStatus::InSemester { week }
        );
    }
    assert_eq!(
        semester_status(SEMESTER_START, date(2026, 9, 5), DEFAULT_WEEK_COUNT),
        SemesterStatus::BeforeSemester {
            days_until_start: 1
        }
    );
    assert_eq!(
        semester_status(SEMESTER_START, date(2027, 1, 24), DEFAULT_WEEK_COUNT),
        SemesterStatus::AfterSemester
    );
}

#[test]
fn dates_of_week_start_on_sunday() {
    let dates = dates_of_week(1, SEMESTER_START);
    assert_eq!(dates.len(), 7);
    assert_eq!(day_of_week(dates[0]), DayOfWeek::Sunday);
    assert_eq!(day_of_week(dates[1]), DayOfWeek::Monday);
    assert_eq!(dates[0], date(2026, 9, 6));
    assert_eq!(dates_of_week(2, SEMESTER_START)[0], date(2026, 9, 13));
    assert!(dates_of_week(0, SEMESTER_START).is_empty());
}

#[test]
fn day_of_week_maps_calendar_weekday() {
    assert_eq!(day_of_week(date(2026, 9, 6)), DayOfWeek::Sunday);
    assert_eq!(day_of_week(date(2026, 9, 7)), DayOfWeek::Monday);
    assert_eq!(day_of_week(date(2026, 9, 12)), DayOfWeek::Saturday);
}

#[test]
fn semester_status_carries_countdown_before_start() {
    for (day, remaining_days) in [(date(2026, 9, 1), 5), (date(2026, 9, 5), 1)] {
        assert_eq!(
            semester_status(SEMESTER_START, day, DEFAULT_WEEK_COUNT),
            SemesterStatus::BeforeSemester {
                days_until_start: remaining_days
            }
        );
    }
    assert_eq!(
        semester_status(SEMESTER_START, SEMESTER_START, DEFAULT_WEEK_COUNT),
        SemesterStatus::InSemester { week: 1 }
    );
    assert_eq!(
        semester_status(SEMESTER_START, date(2027, 1, 24), DEFAULT_WEEK_COUNT),
        SemesterStatus::AfterSemester
    );
}

#[test]
fn semester_date_range_spans_week_count() {
    let range = semester_date_range(SEMESTER_START, 2);
    assert_eq!(range, date(2026, 9, 6)..date(2026, 9, 20));
    assert!(range.contains(&date(2026, 9, 19)));
    assert!(!range.contains(&date(2026, 9, 20)));
}

#[test]
fn weekly_courses_grouped_by_week() {
    let courses = [course("高数", &[1, 2, 3], DayOfWeek::Monday, 1, 2)];
    let weekly = weekly_courses(&courses);
    assert_eq!(weekly.keys().copied().collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(names(&weekly[&2]), ["高数"]);
}

#[test]
fn weekly_courses_preserve_the_session_index() {
    let mut maths = course("高数", &[1], DayOfWeek::Monday, 1, 2);
    let second = course("高数", &[1, 2], DayOfWeek::Friday, 3, 4)
        .sessions
        .remove(0);
    maths.sessions.push(second);
    let courses = [maths];
    let weekly = weekly_courses(&courses);
    assert_eq!(
        weekly[&1]
            .iter()
            .map(CourseOccurrence::session_index)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(weekly[&2][0].session_index(), 1);
    assert_eq!(weekly[&2][0].session().day_of_week, DayOfWeek::Friday);
}

#[test]
fn courses_on_date_filters_by_week_and_day() {
    let courses = [
        course("高数", &[1, 2], DayOfWeek::Monday, 1, 2),
        course("英语", &[3], DayOfWeek::Monday, 3, 4),
    ];
    let on = |day| courses_on(day, SEMESTER_START, &courses, DEFAULT_WEEK_COUNT);
    assert_eq!(names(&on(date(2026, 9, 7))), ["高数"]);
    assert_eq!(names(&on(date(2026, 9, 21))), ["英语"]);
    assert!(on(date(2026, 9, 8)).is_empty());
    assert!(on(date(2026, 9, 5)).is_empty());
}

#[test]
fn time_range_maps_sections_to_clock() {
    let maths = course("高数", &[1], DayOfWeek::Monday, 3, 4);
    let range = time_range(&maths.sessions[0], 1, SEMESTER_START).unwrap();
    assert_eq!(range.start, shanghai(2026, 9, 7, 10, 10));
    assert_eq!(range.end, shanghai(2026, 9, 7, 11, 50));
    assert_eq!(range.start.offset().local_minus_utc(), 28_800);
}

#[test]
fn time_range_rejects_out_of_bounds_sections() {
    for (start, end) in [(11, 12), (5, 3), (0, 2)] {
        let invalid = course("无效节次", &[1], DayOfWeek::Monday, start, end);
        assert_eq!(time_range(&invalid.sessions[0], 1, SEMESTER_START), None);
    }
}

#[test]
fn unfinished_courses_ignore_finished_ones() {
    let courses = [course("高数", &[1], DayOfWeek::Monday, 1, 2)];
    let at = |hour, minute| {
        unfinished_courses(
            &shanghai(2026, 9, 7, hour, minute),
            SEMESTER_START,
            &courses,
            DEFAULT_WEEK_COUNT,
        )
    };
    let before = at(7, 0);
    assert_eq!(before.len(), 1);
    assert!(!before[0].is_current);
    assert!(at(9, 0)[0].is_current);
    assert!(at(9, 40).is_empty());
    assert!(at(9, 45).is_empty());
}

#[test]
fn relevant_course_observes_time_thresholds() {
    let courses = [course("高数", &[1], DayOfWeek::Monday, 1, 2)];
    let at = |hour, minute| {
        relevant_course(
            &shanghai(2026, 9, 7, hour, minute),
            SEMESTER_START,
            &courses,
            DEFAULT_WEEK_COUNT,
        )
    };
    for (hour, minute, visible) in [
        (8, 30, true),
        (7, 50, true),
        (7, 40, true),
        (7, 35, false),
        (9, 42, true),
        (9, 45, true),
        (9, 50, false),
    ] {
        assert_eq!(at(hour, minute).is_some(), visible, "{hour}:{minute:02}");
    }
}

#[test]
fn relevant_course_prioritizes_current_then_upcoming_over_finished() {
    let courses = [
        course("早课", &[1], DayOfWeek::Monday, 1, 1),
        course("下节课", &[1], DayOfWeek::Monday, 2, 2),
    ];
    let at = |minute| {
        relevant_course(
            &shanghai(2026, 9, 7, 8, minute),
            SEMESTER_START,
            &courses,
            DEFAULT_WEEK_COUNT,
        )
        .unwrap()
        .course()
        .course_name
        .as_str()
    };
    assert_eq!(at(40), "早课");
    assert_eq!(at(47), "下节课");
}

#[test]
fn normalize_semester_start_date_uses_source_calendar_day() {
    let new_york_summer = FixedOffset::west_opt(4 * 60 * 60).unwrap();
    for hour in [9, 21] {
        let date = new_york_summer
            .with_ymd_and_hms(2026, 9, 6, hour, 0, 0)
            .unwrap();
        assert_eq!(
            normalize_semester_start_date(&date),
            Some(shanghai(2026, 9, 6, 0, 0))
        );
    }
}

#[test]
fn is_today_recognizes_current_moment() {
    assert!(is_today(&DateTime::<Utc>::from(SystemTime::now())));
    assert!(!is_today(&shanghai(2000, 1, 1, 0, 0)));
}

#[test]
fn courses_on_same_day_are_sorted_by_start_section() {
    let courses = [
        course("下午课", &[1], DayOfWeek::Monday, 5, 6),
        course("上午课", &[1], DayOfWeek::Monday, 1, 2),
    ];
    assert_eq!(
        names(&courses_on(
            date(2026, 9, 7),
            SEMESTER_START,
            &courses,
            DEFAULT_WEEK_COUNT
        )),
        ["上午课", "下午课"]
    );
}

#[test]
fn unfinished_courses_skip_out_of_bounds_sections() {
    let courses = [course("坏数据", &[1], DayOfWeek::Monday, 11, 12)];
    assert!(
        unfinished_courses(
            &shanghai(2026, 9, 7, 9, 0),
            SEMESTER_START,
            &courses,
            DEFAULT_WEEK_COUNT
        )
        .is_empty()
    );
}

#[test]
fn relevant_course_picks_nearest_upcoming() {
    let courses = [
        course("早课", &[1], DayOfWeek::Monday, 1, 2),
        course("午课", &[1], DayOfWeek::Monday, 5, 6),
    ];
    let chosen = relevant_course(
        &shanghai(2026, 9, 7, 7, 45),
        SEMESTER_START,
        &courses,
        DEFAULT_WEEK_COUNT,
    )
    .unwrap();
    assert_eq!(chosen.course().course_name, "早课");
}

#[test]
fn relevant_course_picks_most_recently_finished() {
    let courses = [
        course("早课", &[1], DayOfWeek::Monday, 1, 2),
        course("晚课", &[1], DayOfWeek::Monday, 3, 4),
    ];
    let chosen = relevant_course(
        &shanghai(2026, 9, 7, 11, 52),
        SEMESTER_START,
        &courses,
        DEFAULT_WEEK_COUNT,
    )
    .unwrap();
    assert_eq!(chosen.course().course_name, "晚课");
}

#[test]
fn relevant_course_skips_out_of_bounds_sections() {
    let courses = [course("坏数据", &[1], DayOfWeek::Monday, 11, 12)];
    assert!(
        relevant_course(
            &shanghai(2026, 9, 7, 9, 0),
            SEMESTER_START,
            &courses,
            DEFAULT_WEEK_COUNT
        )
        .is_none()
    );
}
