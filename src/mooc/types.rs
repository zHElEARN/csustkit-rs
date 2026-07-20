use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub last_login_time: String,
    pub total_online_time: String,
    pub login_count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub id: String,
    pub name: String,
    pub number: Option<String>,
    pub department: Option<String>,
    pub teacher: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub id: i64,
    pub title: String,
    pub publisher: String,
    pub can_submit: bool,
    pub submit_status: bool,
    pub deadline: DateTime<FixedOffset>,
    pub start_time: DateTime<FixedOffset>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exam {
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub allow_retake: Option<i64>,
    pub time_limit: i64,
    pub is_submitted: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MoocError {
    #[error("网络课程中心客户端创建失败: {0}")]
    ClientBuildFailed(String),
    #[error("获取个人信息失败: {0}")]
    ProfileRetrievalFailed(String),
    #[error("获取课程信息失败: {0}")]
    CourseRetrievalFailed(String),
    #[error("获取作业信息失败: {0}")]
    AssignmentsRetrievalFailed(String),
    #[error("获取测验信息失败: {0}")]
    TestRetrievalFailed(String),
    #[error("获取有待完成作业的课程名称失败: {0}")]
    CourseNamesWithPendingAssignmentsRetrievalFailed(String),
    #[error("网络课程中心未登录")]
    NotLoggedIn,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}
