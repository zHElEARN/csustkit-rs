use serde::{Deserialize, Serialize};

/// 长沙理工大学校区。
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
}

/// 星期，数值从星期日的 0 到星期六的 6。
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
    pub(crate) fn from_chinese_name(name: &str) -> Option<Self> {
        Some(match name {
            "星期一" => Self::Monday,
            "星期二" => Self::Tuesday,
            "星期三" => Self::Wednesday,
            "星期四" => Self::Thursday,
            "星期五" => Self::Friday,
            "星期六" => Self::Saturday,
            "星期日" | "星期天" => Self::Sunday,
            _ => return None,
        })
    }
}
