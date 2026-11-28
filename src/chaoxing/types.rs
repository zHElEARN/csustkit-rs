use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::WebVpnError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "uniffi",
    derive(uniffi::Record),
    uniffi(name = "ChaoxingProfile")
)]
pub struct Profile {
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "uniffi",
    derive(uniffi::Enum),
    uniffi(name = "ChaoxingAssignmentStatus")
)]
pub enum AssignmentStatus {
    Unsubmitted,
    PendingReview,
    Completed,
}

impl AssignmentStatus {
    pub const ALL: [Self; 3] = [Self::Unsubmitted, Self::PendingReview, Self::Completed];

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Unsubmitted => "未提交",
            Self::PendingReview => "待批阅",
            Self::Completed => "已完成",
        }
    }

    pub(super) fn from_display_name(name: &str) -> Option<Self> {
        let name = name.trim();
        if name == "已互评" {
            return Some(Self::Completed);
        }
        if name == "待互评" {
            return Some(Self::Unsubmitted);
        }
        Self::ALL
            .into_iter()
            .find(|status| status.display_name() == name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "uniffi",
    derive(uniffi::Record),
    uniffi(name = "ChaoxingAssignment")
)]
pub struct Assignment {
    pub title: String,
    pub status: AssignmentStatus,
    pub course_name: String,
    /// 根据页面显示的剩余时间推算。页面未提供剩余时间时为空。
    pub deadline: Option<DateTime<Utc>>,
    /// 图标资源的原始链接。
    pub icon_url: Url,
    /// 作业详情链接，WebVPN 模式下需要对应的登录会话。
    pub detail_url: Url,
}

impl Assignment {
    pub const fn is_completed(&self) -> bool {
        matches!(self.status, AssignmentStatus::Completed)
    }
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error), uniffi(flat_error))]
pub enum ChaoxingError {
    #[error("获取学习通个人信息失败: {0}")]
    ProfileRetrievalFailed(String),
    #[error("获取学习通作业失败: {0}")]
    AssignmentsRetrievalFailed(String),
    #[error("学习通未登录")]
    NotLoggedIn,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error("处理学习通作业的 WebVPN 链接失败: {0}")]
    WebVpn(#[from] WebVpnError),
}

#[cfg(test)]
mod tests {
    use super::AssignmentStatus;

    #[test]
    fn parses_standard_and_peer_review_assignment_statuses() {
        use AssignmentStatus::{Completed, PendingReview, Unsubmitted};

        for (label, expected) in [
            ("未提交", Unsubmitted),
            ("待批阅", PendingReview),
            ("已完成", Completed),
            ("已互评", Completed),
            ("待互评", Unsubmitted),
            (" 待互评 ", Unsubmitted),
        ] {
            assert_eq!(AssignmentStatus::from_display_name(label), Some(expected));
        }
    }
}
