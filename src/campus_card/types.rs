use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CampusCardError {
    #[error("个人信息获取失败: {0}")]
    ProfileRetrievalFailed(String),
    #[error("楼栋信息获取失败: {0}")]
    BuildingsRetrievalFailed(String),
    #[error("宿舍列表获取失败: {0}")]
    RoomsRetrievalFailed(String),
    #[error("宿舍电量获取失败: {0}")]
    ElectricityRetrievalFailed(String),
    #[error("校园卡系统未登录")]
    NotLoggedIn,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}

/// 校区。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Campus {
    Yuntang,
    Jinpenling,
}

impl Campus {
    pub const ALL: [Self; 2] = [Self::Yuntang, Self::Jinpenling];

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Yuntang => "云塘校区",
            Self::Jinpenling => "金盆岭校区",
        }
    }

    pub(super) const fn fee_item_id(self) -> &'static str {
        match self {
            Self::Yuntang => "448",
            Self::Jinpenling => "468",
        }
    }

    pub(super) const fn campus_id(self) -> &'static str {
        match self {
            Self::Yuntang => "1",
            Self::Jinpenling => "22",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Building {
    pub name: String,
    pub id: String,
    pub campus: Campus,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub id: String,
    pub building: Building,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: i64,
    pub account: String,
    pub name: String,
    pub avatar: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub user_type: String,
    pub sno: String,
    pub card_id: String,
    pub card_account: String,
    pub sex: i64,
    pub country_code: String,
    pub nation_code: String,
    pub politics_status_code: String,
    pub identity_code: String,
    pub campus_code: String,
    pub department_code: String,
    pub department_name: String,
    pub profession_code: String,
    pub profession_name: String,
    pub class_code: String,
    pub class_name: String,
    pub flag: String,
    pub user_id: i64,
    pub id_number: String,
    pub identity_name: String,
    pub identity_exp_date: String,
    pub remark: String,
}
