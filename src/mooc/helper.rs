use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use encoding_rs::GBK;
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;

use crate::{
    connection::ConnectionMode,
    session::CsustSession,
    url_factory::{ServiceDomain, make_url},
};

use super::types::{Assignment, Course, Exam, MoocError, Profile};

const PROFILE_PATH: &str = "/meol/personal.do";
const COURSES_PATH: &str = "/meol/lesson/blen.student.lesson.list.jsp";
const PENDING_ASSIGNMENTS_PATH: &str = "/meol/welcomepage/student/interaction_reminder_v8.jsp";
const ASSIGNMENTS_PATH_PREFIX: &str = "/meol/hw/stu/hwStuHwtList.do?sortDirection=-1&courseId=";
const EXAMS_PATH_PREFIX: &str =
    "/meol/common/question/test/student/list.jsp?sortColumn=createTime&sortDirection=-1&cateId=";

#[derive(Debug, Deserialize)]
struct AssignmentsResponse {
    datas: AssignmentsData,
}

#[derive(Debug, Deserialize)]
struct AssignmentsData {
    #[serde(rename = "hwtList")]
    assignments: Option<Vec<AssignmentResponse>>,
}

#[derive(Debug, Deserialize)]
struct AssignmentResponse {
    #[serde(rename = "realName")]
    publisher: String,
    #[serde(rename = "startDateTime")]
    start_time: String,
    #[serde(rename = "submitStruts")]
    can_submit: bool,
    id: i64,
    title: String,
    #[serde(rename = "deadLine")]
    deadline: String,
    #[serde(rename = "answerStatus")]
    answer_status: Option<bool>,
}

pub struct MoocHelper {
    mode: ConnectionMode,
    session: CsustSession,
}

impl MoocHelper {
    pub fn new(mode: ConnectionMode) -> Result<Self, MoocError> {
        let session =
            CsustSession::new().map_err(|error| MoocError::ClientBuildFailed(error.to_string()))?;
        Ok(Self::with_session(mode, session))
    }

    pub fn with_session(mode: ConnectionMode, session: CsustSession) -> Self {
        Self { mode, session }
    }

    pub async fn get_profile(&self) -> Result<Profile, MoocError> {
        let response = self.get_gbk(PROFILE_PATH).await?;
        if is_login_required(&response) {
            return Err(MoocError::NotLoggedIn);
        }

        let document = Html::parse_document(&response);
        let selector = selector(".userinfobody > ul > li")
            .map_err(|error| MoocError::ProfileRetrievalFailed(error.to_owned()))?;
        let elements = document.select(&selector).collect::<Vec<_>>();
        if elements.len() < 5 {
            return Err(MoocError::ProfileRetrievalFailed(
                "个人信息格式异常".to_owned(),
            ));
        }

        let name = text(elements.get(1).copied());
        let last_login_time = text(elements.get(2).copied()).replace("登录时间：", "");
        let total_online_time = text(elements.get(3).copied()).replace("在线总时长： ", "");
        let login_count_text = text(elements.get(4).copied()).replace("登录次数：", "");
        let login_count = login_count_text
            .parse::<i64>()
            .map_err(|_| MoocError::ProfileRetrievalFailed("登录次数格式无效".to_owned()))?;

        Ok(Profile {
            name,
            last_login_time,
            total_online_time,
            login_count,
        })
    }

    pub async fn get_courses(&self) -> Result<Vec<Course>, MoocError> {
        let response = self.get_gbk(COURSES_PATH).await?;
        if is_login_required(&response) {
            return Err(MoocError::NotLoggedIn);
        }

        let document = Html::parse_document(&response);
        let table_selector = selector("#table2")
            .map_err(|error| MoocError::CourseRetrievalFailed(error.to_owned()))?;
        let table = document
            .select(&table_selector)
            .next()
            .ok_or_else(|| MoocError::CourseRetrievalFailed("未找到课程表格".to_owned()))?;
        let row_selector =
            selector("tr").map_err(|error| MoocError::CourseRetrievalFailed(error.to_owned()))?;
        let cell_selector =
            selector("td").map_err(|error| MoocError::CourseRetrievalFailed(error.to_owned()))?;
        let link_selector =
            selector("a").map_err(|error| MoocError::CourseRetrievalFailed(error.to_owned()))?;
        let rows = table.select(&row_selector).collect::<Vec<_>>();
        if rows.is_empty() {
            return Err(MoocError::CourseRetrievalFailed(
                "课程表格格式无效".to_owned(),
            ));
        }

        let mut courses = Vec::new();
        for row in rows.iter().skip(1) {
            let cells = row.select(&cell_selector).collect::<Vec<_>>();
            if cells.len() < 4 {
                return Err(MoocError::CourseRetrievalFailed(
                    "课程行格式异常".to_owned(),
                ));
            }
            let name = text(cells.get(1).copied());
            let link = cells
                .get(1)
                .and_then(|cell| cell.select(&link_selector).next())
                .ok_or_else(|| MoocError::CourseRetrievalFailed("未找到课程ID".to_owned()))?;
            let onclick = link
                .value()
                .attr("onclick")
                .ok_or_else(|| MoocError::CourseRetrievalFailed("未找到课程ID".to_owned()))?;
            let mut id = onclick
                .replace(
                    "window.open('../homepage/course/course_index.jsp?courseId=",
                    "",
                )
                .replace("','manage_course')", "");
            if self.mode == ConnectionMode::WebVpn {
                id = id
                    .replace("var vpn_return;eval(vpn_rewrite_js((function () { ", "")
                    .replace(" }).toString().slice(14, -2), 2));return vpn_return;", "");
            }

            courses.push(Course {
                id,
                name,
                number: Some(text(cells.first().copied())),
                department: Some(text(cells.get(2).copied())),
                teacher: Some(text(cells.get(3).copied())),
            });
        }
        Ok(courses)
    }

    pub async fn get_course_assignments(
        &self,
        course: &Course,
    ) -> Result<Vec<Assignment>, MoocError> {
        let path = format!(
            "{ASSIGNMENTS_PATH_PREFIX}{}&pagingPage=1&pagingNumberPer=1000&sortColumn=deadline",
            course.id
        );
        let response = self.get_utf8(&path).await?;
        if is_login_required(&response) {
            return Err(MoocError::NotLoggedIn);
        }
        let parsed = serde_json::from_str::<AssignmentsResponse>(&response).map_err(|error| {
            MoocError::AssignmentsRetrievalFailed(format!("作业信息格式无效: {error}"))
        })?;
        let offset = FixedOffset::east_opt(8 * 60 * 60).ok_or_else(|| {
            MoocError::AssignmentsRetrievalFailed("无法构建 Asia/Shanghai 时区".to_owned())
        })?;

        Ok(parsed
            .datas
            .assignments
            .unwrap_or_default()
            .into_iter()
            .filter_map(|assignment| {
                let deadline = parse_datetime(&assignment.deadline, offset)?;
                let start_time = parse_datetime(&assignment.start_time, offset)?;
                Some(Assignment {
                    id: assignment.id,
                    title: assignment.title,
                    publisher: assignment.publisher,
                    can_submit: assignment.can_submit,
                    submit_status: assignment.answer_status.is_some(),
                    deadline,
                    start_time,
                })
            })
            .collect())
    }

    pub async fn get_course_exams(&self, course: &Course) -> Result<Vec<Exam>, MoocError> {
        let path = format!(
            "{EXAMS_PATH_PREFIX}{}&pagingPage=1&status=1&pagingNumberPer=1000",
            course.id
        );
        let response = self.get_gbk(&path).await?;
        if is_login_required(&response) {
            return Err(MoocError::NotLoggedIn);
        }

        let document = Html::parse_document(&response);
        let table_selector = selector(".valuelist")
            .map_err(|error| MoocError::TestRetrievalFailed(error.to_owned()))?;
        let table = document
            .select(&table_selector)
            .next()
            .ok_or_else(|| MoocError::TestRetrievalFailed("未找到测试表格".to_owned()))?;
        let row_selector =
            selector("tr").map_err(|error| MoocError::TestRetrievalFailed(error.to_owned()))?;
        let cell_selector =
            selector("td").map_err(|error| MoocError::TestRetrievalFailed(error.to_owned()))?;
        let rows = table.select(&row_selector).collect::<Vec<_>>();
        if rows.is_empty() {
            return Err(MoocError::TestRetrievalFailed(
                "测试表格格式无效".to_owned(),
            ));
        }

        let mut exams = Vec::new();
        for row in rows.iter().skip(1) {
            let cells = row.select(&cell_selector).collect::<Vec<_>>();
            if cells.len() < 8 {
                return Err(MoocError::TestRetrievalFailed(
                    "测试表格行格式异常".to_owned(),
                ));
            }
            let allow_retake_text = text(cells.get(3).copied());
            let allow_retake = if allow_retake_text == "不限制" {
                None
            } else {
                allow_retake_text.parse::<i64>().ok()
            };
            let time_limit = text(cells.get(4).copied())
                .parse::<i64>()
                .map_err(|_| MoocError::TestRetrievalFailed("时间限制格式无效".to_owned()))?;
            let result_html = cells
                .get(7)
                .map(|cell| cell.inner_html())
                .unwrap_or_default();
            exams.push(Exam {
                title: text(cells.first().copied()),
                start_time: text(cells.get(1).copied()),
                end_time: text(cells.get(2).copied()),
                allow_retake,
                time_limit,
                is_submitted: result_html.contains("查看结果"),
            });
        }
        Ok(exams)
    }

    pub async fn get_courses_with_pending_assignments(&self) -> Result<Vec<Course>, MoocError> {
        let response = self.get_gbk(PENDING_ASSIGNMENTS_PATH).await?;
        if is_login_required(&response) {
            return Err(MoocError::NotLoggedIn);
        }

        let document = Html::parse_document(&response);
        let reminder_selector = selector("#reminder").map_err(|error| {
            MoocError::CourseNamesWithPendingAssignmentsRetrievalFailed(error.to_owned())
        })?;
        let _reminder = document.select(&reminder_selector).next().ok_or_else(|| {
            MoocError::CourseNamesWithPendingAssignmentsRetrievalFailed("未找到提醒区域".to_owned())
        })?;
        let child_selector = selector("#reminder > *").map_err(|error| {
            MoocError::CourseNamesWithPendingAssignmentsRetrievalFailed(error.to_owned())
        })?;
        let anchor_selector = selector("a").map_err(|error| {
            MoocError::CourseNamesWithPendingAssignmentsRetrievalFailed(error.to_owned())
        })?;
        let course_selector = selector("ul > li > a").map_err(|error| {
            MoocError::CourseNamesWithPendingAssignmentsRetrievalFailed(error.to_owned())
        })?;
        let assignments_container = document.select(&child_selector).find(|element| {
            element
                .select(&anchor_selector)
                .next()
                .map(|anchor| text(Some(anchor)).contains("待提交作业"))
                .unwrap_or(false)
        });
        let Some(container) = assignments_container else {
            return Ok(Vec::new());
        };

        let mut courses = Vec::new();
        for element in container.select(&course_selector) {
            let Some(onclick) = element.value().attr("onclick") else {
                continue;
            };
            let Some(id) = extract_lid(onclick) else {
                continue;
            };
            courses.push(Course {
                id,
                name: text(Some(element)),
                number: None,
                department: None,
                teacher: None,
            });
        }
        Ok(courses)
    }

    pub async fn logout(&self) -> Result<(), MoocError> {
        self.session
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::Mooc,
                "/meol/homepage/V8/include/logout.jsp",
            ))
            .send()
            .await?
            .bytes()
            .await?;
        Ok(())
    }

    pub async fn is_logged_in(&self) -> bool {
        self.get_profile().await.is_ok()
    }

    async fn get_gbk(&self, path: &str) -> Result<String, MoocError> {
        let bytes = self
            .session
            .client
            .get(make_url(self.mode, ServiceDomain::Mooc, path))
            .send()
            .await?
            .bytes()
            .await?;
        Ok(GBK.decode(&bytes).0.into_owned())
    }

    async fn get_utf8(&self, path: &str) -> Result<String, MoocError> {
        let bytes = self
            .session
            .client
            .get(make_url(self.mode, ServiceDomain::Mooc, path))
            .send()
            .await?
            .bytes()
            .await?;
        String::from_utf8(bytes.to_vec()).map_err(|error| {
            MoocError::AssignmentsRetrievalFailed(format!("作业信息不是有效的 UTF-8: {error}"))
        })
    }
}

fn selector(value: &str) -> Result<Selector, String> {
    Selector::parse(value).map_err(|error| format!("解析 HTML 选择器失败: {error}"))
}

fn text(element: Option<ElementRef<'_>>) -> String {
    element
        .map(|element| element.text().collect::<String>().trim().to_owned())
        .unwrap_or_default()
}

fn is_login_required(response: &str) -> bool {
    response.contains("<TITLE>错误！</TITLE>") || response.contains("请登录！")
}

fn parse_datetime(value: &str, offset: FixedOffset) -> Option<chrono::DateTime<FixedOffset>> {
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").ok()?;
    offset.from_local_datetime(&naive).single()
}

fn extract_lid(onclick: &str) -> Option<String> {
    let start = onclick.find("lid=")? + "lid=".len();
    let id = onclick[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    (!id.is_empty()).then_some(id)
}
