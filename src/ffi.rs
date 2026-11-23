use std::{ops::RangeInclusive, sync::Arc, time::SystemTime};

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeDelta, Utc};
use url::Url;

use crate::{
    ConnectionMode, CsustSession,
    campus_card::CampusCardHelper,
    chaoxing::ChaoxingHelper,
    edu::{EduError, EduHelper},
    mooc::MoocHelper,
    physics_experiment::PhysicsExperimentHelper,
    sso::SsoHelper,
};

mod labels;
mod schedule;

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

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CalendarDateError {
    #[error("无效的公历日期：{year} 年 {month} 月 {day} 日")]
    InvalidDate { year: i32, month: u32, day: u32 },
}

impl From<NaiveDate> for CalendarDate {
    fn from(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        }
    }
}

impl TryFrom<CalendarDate> for NaiveDate {
    type Error = CalendarDateError;

    fn try_from(CalendarDate { year, month, day }: CalendarDate) -> Result<Self, Self::Error> {
        Self::from_ymd_opt(year, month, day).ok_or(CalendarDateError::InvalidDate {
            year,
            month,
            day,
        })
    }
}

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

/// 浮点数闭区间，包含起点和终点。
#[derive(uniffi::Record)]
pub struct FloatingPointRange {
    pub start: f64,
    pub end: f64,
}

type InclusiveFloatRange = RangeInclusive<f64>;

uniffi::custom_type!(InclusiveFloatRange, FloatingPointRange, {
    remote,
    lower: |range| FloatingPointRange {
        start: *range.start(),
        end: *range.end(),
    },
    try_lift: |range| Ok(range.start..=range.end),
});

type Count = usize;

uniffi::custom_type!(Count, u64, {
    remote,
    lower: |value| value as u64,
    try_lift: |value| Ok(usize::try_from(value)?),
});

/// 为会话附加的根证书数据。
#[derive(uniffi::Enum)]
pub enum SessionRootCertificate {
    Pem { data: Vec<u8> },
    Der { data: Vec<u8> },
}

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

    #[uniffi::constructor]
    pub fn configured(
        user_agent: Option<String>,
        root_certificates: Vec<SessionRootCertificate>,
    ) -> Result<Arc<Self>, SessionInitError> {
        let mut builder = Self::builder();
        if let Some(user_agent) = user_agent {
            builder = builder.user_agent(user_agent);
        }
        for certificate in root_certificates {
            let certificate = match certificate {
                SessionRootCertificate::Pem { data } => reqwest::Certificate::from_pem(&data),
                SessionRootCertificate::Der { data } => reqwest::Certificate::from_der(&data),
            }?;
            builder = builder.add_root_certificate(certificate);
        }
        Ok(Arc::new(builder.build()?))
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

#[uniffi::export(async_runtime = "tokio")]
impl EduHelper {
    #[uniffi::method(name = "get_semester_start_date")]
    async fn get_semester_start_calendar_date(
        &self,
        academic_year_semester: Option<String>,
    ) -> Result<CalendarDate, EduError> {
        self.get_semester_start_date(academic_year_semester)
            .await
            .map(CalendarDate::from)
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

#[uniffi::export]
impl CampusCardHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}

#[uniffi::export]
impl PhysicsExperimentHelper {
    #[uniffi::constructor(name = "new")]
    pub fn with_shared_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self::new(mode, session.as_ref().clone()))
    }
}
