use std::time::SystemTimeError;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use url::Url;
use crate::SessionRequestError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub department: String,
    pub major: String,
    pub education_system: String,
    pub class_name: String,
    pub student_id: String,
    pub name: String,
    pub gender: String,
    pub name_pinyin: String,
    pub birth_date: String,
    pub ethnicity: String,
    pub study_level: String,
    pub home_address: String,
    pub home_phone: String,
    pub personal_phone: String,
    pub enrollment_date: String,
    pub entrance_exam_id: String,
    pub id_card_number: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamScheduleQuery {
    pub academic_year_semester: Option<String>,
    pub semester_type: Option<SemesterType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SemesterType {
    Beginning,
    Middle,
    End,
}

impl SemesterType {
    pub(super) const fn display_name(self) -> &'static str {
        match self {
            Self::Beginning => "期初",
            Self::Middle => "期中",
            Self::End => "期末",
        }
    }

    pub(super) const fn request_id(self) -> &'static str {
        match self {
            Self::Beginning => "1",
            Self::Middle => "2",
            Self::End => "3",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemesterOptions {
    pub semesters: Vec<String>,
    pub default_semester: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Campus {
    Yuntang,
    Jinpenling,
}

impl Campus {
    pub(super) const fn request_id(self) -> &'static str {
        match self {
            Self::Yuntang => "1",
            Self::Jinpenling => "2",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSchedule {
    pub courses: Vec<Course>,
    pub remarks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub course_name: String,
    pub group_name: Option<String>,
    pub teacher: Option<String>,
    pub sessions: Vec<ScheduleSession>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSession {
    pub weeks: Vec<i64>,
    pub start_section: i64,
    pub end_section: i64,
    pub day_of_week: DayOfWeek,
    pub classroom: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum DayOfWeek {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

impl DayOfWeek {
    pub(super) const fn from_column(column: usize) -> Option<Self> {
        match column {
            0 => Some(Self::Sunday),
            1 => Some(Self::Monday),
            2 => Some(Self::Tuesday),
            3 => Some(Self::Wednesday),
            4 => Some(Self::Thursday),
            5 => Some(Self::Friday),
            6 => Some(Self::Saturday),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableClassroomsQuery {
    pub campus: Campus,
    pub week: i64,
    pub day_of_week: DayOfWeek,
    pub section: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exam {
    pub campus: String,
    pub session: String,
    pub course_id: String,
    pub course_name: String,
    pub teacher: String,
    pub exam_time: String,
    pub exam_start_time: DateTime<FixedOffset>,
    pub exam_end_time: DateTime<FixedOffset>,
    pub exam_room: String,
    pub seat_number: String,
    pub admission_ticket_number: String,
    pub remarks: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayMode {
    #[default]
    BestGrade,
    AllGrades,
}

impl DisplayMode {
    pub(super) const fn request_id(self) -> &'static str {
        match self {
            Self::BestGrade => "max",
            Self::AllGrades => "all",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StudyMode {
    #[default]
    Major,
    Minor,
    All,
}

impl StudyMode {
    pub(super) const fn request_id(self) -> &'static str {
        match self {
            Self::Major => "0",
            Self::Minor => "1",
            Self::All => "2",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseNature {
    Other,
    PublicCourse,
    PublicBasicCourse,
    ProfessionalBasicCourse,
    ProfessionalCourse,
    ProfessionalElectiveCourse,
    PublicElectiveCourse,
    ProfessionalCoreCourse,
    CrossProfessionalElectiveCourse,
    ProfessionalPracticalCourse,
}

impl CourseNature {
    pub const ALL: [Self; 10] = [
        Self::Other,
        Self::PublicCourse,
        Self::PublicBasicCourse,
        Self::ProfessionalBasicCourse,
        Self::ProfessionalCourse,
        Self::ProfessionalElectiveCourse,
        Self::PublicElectiveCourse,
        Self::ProfessionalCoreCourse,
        Self::CrossProfessionalElectiveCourse,
        Self::ProfessionalPracticalCourse,
    ];

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Other => "其它",
            Self::PublicCourse => "公共课",
            Self::PublicBasicCourse => "公共基础课",
            Self::ProfessionalBasicCourse => "专业基础课",
            Self::ProfessionalCourse => "专业课",
            Self::ProfessionalElectiveCourse => "专业选修课",
            Self::PublicElectiveCourse => "公共选修课",
            Self::ProfessionalCoreCourse => "专业核心课",
            Self::CrossProfessionalElectiveCourse => "跨专业选修课",
            Self::ProfessionalPracticalCourse => "专业集中实践",
        }
    }

    pub(super) const fn request_id(self) -> &'static str {
        match self {
            Self::Other => "00",
            Self::PublicCourse => "01",
            Self::PublicBasicCourse => "02",
            Self::ProfessionalBasicCourse => "03",
            Self::ProfessionalCourse => "04",
            Self::ProfessionalElectiveCourse => "05",
            Self::PublicElectiveCourse => "06",
            Self::ProfessionalCoreCourse => "07",
            Self::CrossProfessionalElectiveCourse => "09",
            Self::ProfessionalPracticalCourse => "20",
        }
    }

    pub(super) fn from_display_name(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|nature| nature.display_name() == value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGradeQuery {
    pub academic_year_semester: Option<String>,
    pub course_nature: Option<CourseNature>,
    pub course_name: String,
    pub display_mode: DisplayMode,
    pub study_mode: StudyMode,
}

impl Default for CourseGradeQuery {
    fn default() -> Self {
        Self {
            academic_year_semester: None,
            course_nature: None,
            course_name: String::new(),
            display_mode: DisplayMode::BestGrade,
            study_mode: StudyMode::Major,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseGrade {
    pub semester: String,
    pub course_id: String,
    pub course_name: String,
    pub group_name: String,
    pub grade: i64,
    pub grade_detail_url: Url,
    pub study_mode: String,
    pub grade_identifier: String,
    pub credit: f64,
    pub total_hours: f64,
    pub grade_point: f64,
    pub retake_semester: String,
    pub assessment_method: String,
    pub exam_nature: String,
    pub course_attribute: String,
    pub course_nature: CourseNature,
    pub course_category: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeComponent {
    pub component_type: String,
    pub grade: f64,
    pub ratio: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeDetail {
    pub components: Vec<GradeComponent>,
    pub total_grade: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum EduError {
    #[error("会话请求失败: {0}")]
    SessionRequest(#[from] SessionRequestError),
    #[error("个人信息获取失败: {0}")]
    ProfileRetrievalFailed(String),
    #[error("课程成绩获取失败: {0}")]
    CourseGradesRetrievalFailed(String),
    #[error("课程成绩可选学期获取失败: {0}")]
    AvailableSemestersForCourseGradesRetrievalFailed(String),
    #[error("成绩详情获取失败: {0}")]
    GradeDetailRetrievalFailed(String),
    #[error("考试安排获取失败: {0}")]
    ExamScheduleRetrievalFailed(String),
    #[error("考试安排可选学期获取失败: {0}")]
    AvailableSemestersForExamScheduleRetrievalFailed(String),
    #[error("日期解析失败: {0}")]
    DateParsingFailed(String),
    #[error("课程表获取失败: {0}")]
    CourseScheduleRetrievalFailed(String),
    #[error("课程表可选学期获取失败: {0}")]
    AvailableSemestersForCourseScheduleRetrievalFailed(String),
    #[error("学期开始日期获取失败: {0}")]
    SemesterStartDateRetrievalFailed(String),
    #[error("开始日期可选学期获取失败: {0}")]
    AvailableSemestersForStartDateRetrievalFailed(String),
    #[error("指定校区在指定时间内空闲的教室列表获取失败: {0}")]
    AvailableClassroomsRetrievalFailed(String),
    #[error("教务系统登出失败: {0}")]
    LogoutFailed(#[source] reqwest::Error),
    #[error("获取当前时间失败: {0}")]
    Time(#[source] SystemTimeError),
    #[error("教务系统未登录")]
    NotLoggedIn,
}
