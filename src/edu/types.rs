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

#[derive(Debug, thiserror::Error)]
pub enum EduError {
    #[error("教务客户端创建失败: {0}")]
    ClientBuildFailed(String),
    #[error("教务请求失败: {0}")]
    EducationRequest(#[from] EducationRequestError),
    #[error("个人信息获取失败: {0}")]
    ProfileRetrievalFailed(String),
    #[error("教务系统未登录")]
    NotLoggedIn,
}
