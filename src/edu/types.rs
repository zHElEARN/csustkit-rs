use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum EducationRequestError {
    #[error("教务请求连续 {attempts} 次返回空响应: {url}")]
    EmptyResponse { url: Url, attempts: u8 },
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}

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
    #[error("教务客户端创建失败: {0}")]
    ClientBuildFailed(String),
    #[error("教务请求失败: {0}")]
    EducationRequest(#[from] EducationRequestError),
    #[error("个人信息获取失败: {0}")]
    ProfileRetrievalFailed(String),
    #[error("课程成绩获取失败: {0}")]
    CourseGradesRetrievalFailed(String),
    #[error("课程成绩可选学期获取失败: {0}")]
    AvailableSemestersForCourseGradesRetrievalFailed(String),
    #[error("成绩详情获取失败: {0}")]
    GradeDetailRetrievalFailed(String),
    #[error("教务系统未登录")]
    NotLoggedIn,
}
