use std::{ops::RangeInclusive, sync::Arc, time::SystemTime};

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeDelta, Utc};
use url::Url;

use crate::{
    ConnectionMode, CsustSession, chaoxing::ChaoxingHelper, edu::EduHelper, mooc::MoocHelper,
    sso::SsoHelper,
};

uniffi::custom_type!(Url, String, {
    remote,
    lower: |url| url.into(),
    try_lift: |value| Ok(Url::parse(&value)?),
});

/// 公历日期，不含时间或时区。
#[derive(uniffi::Record)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

uniffi::custom_type!(NaiveDate, CalendarDate, {
    remote,
    lower: |date| CalendarDate {
        year: date.year(),
        month: date.month(),
        day: date.day(),
    },
    try_lift: |date| NaiveDate::from_ymd_opt(date.year, date.month, date.day)
        .ok_or_else(|| uniffi::deps::anyhow::anyhow!("无效的公历日期")),
});

type UtcDateTime = DateTime<Utc>;

uniffi::custom_type!(UtcDateTime, SystemTime, {
    remote,
    lower: |date_time| date_time.into(),
    try_lift: |time| utc_from_system_time(time),
});

type OffsetDateTime = DateTime<FixedOffset>;

uniffi::custom_type!(OffsetDateTime, SystemTime, {
    remote,
    lower: |date_time| date_time.into(),
    // A foreign timestamp represents an instant. Lift it with a UTC offset.
    try_lift: |time| Ok(utc_from_system_time(time)?.fixed_offset()),
});

fn utc_from_system_time(time: SystemTime) -> uniffi::Result<UtcDateTime> {
    let epoch = DateTime::<Utc>::UNIX_EPOCH;
    let date_time = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(elapsed) => epoch.checked_add_signed(TimeDelta::from_std(elapsed)?),
        Err(error) => epoch.checked_sub_signed(TimeDelta::from_std(error.duration())?),
    };
    date_time.ok_or_else(|| uniffi::deps::anyhow::anyhow!("时间戳超出日期范围"))
}

/// 整数闭区间，包含起点和终点。
#[derive(uniffi::Record)]
pub struct InclusiveRange {
    pub start: i64,
    pub end: i64,
}

type InclusiveIntRange = RangeInclusive<i64>;

uniffi::custom_type!(InclusiveIntRange, InclusiveRange, {
    remote,
    lower: |range| InclusiveRange {
        start: *range.start(),
        end: *range.end(),
    },
    try_lift: |range| Ok(range.start..=range.end),
});

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum SessionInitError {
    #[error("无法创建校园服务会话")]
    ClientBuild(#[from] reqwest::Error),
}

#[uniffi::export]
impl CsustSession {
    #[uniffi::constructor(name = "new")]
    pub fn new_shared() -> Result<Arc<Self>, SessionInitError> {
        Ok(Arc::new(Self::new()?))
    }
}

#[uniffi::export]
impl SsoHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}

#[uniffi::export]
impl EduHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}

#[uniffi::export]
impl MoocHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}

#[uniffi::export]
impl ChaoxingHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}
