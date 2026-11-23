use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::DayOfWeek;

/// 一次已排定的物理实验。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "uniffi",
    derive(uniffi::Record),
    uniffi(name = "PhysicsExperimentCourse")
)]
pub struct Course {
    pub id: i64,
    pub name: String,
    pub batch: String,
    pub teacher: String,
    pub location: String,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub class_hours: i64,
    pub week: i64,
    pub day_of_week: DayOfWeek,
}

/// 一个物理实验项目的成绩，分项成绩可能尚未录入。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "uniffi",
    derive(uniffi::Record),
    uniffi(name = "PhysicsExperimentCourseGrade")
)]
pub struct CourseGrade {
    pub course_code: String,
    pub course_name: String,
    pub item_name: String,
    pub preview_grade: Option<i64>,
    pub operation_grade: Option<i64>,
    pub report_grade: Option<i64>,
    pub total_grade: i64,
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error), uniffi(flat_error))]
pub enum PhysicsExperimentError {
    #[error("大物实验平台未确认登录成功（HTTP {0}）")]
    LoginFailed(reqwest::StatusCode),
    #[error("获取大物实验课表失败: {0}")]
    SchedulesRetrievalFailed(String),
    #[error("获取大物实验成绩失败: {0}")]
    CourseGradesRetrievalFailed(String),
    #[error("大物实验平台未登录")]
    NotLoggedIn,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}
