use std::time::SystemTime;

use chrono::{DateTime, TimeDelta, Utc};
use encoding_rs::UTF_8;
use reqwest::header::ACCEPT_LANGUAGE;
use scraper::{ElementRef, Html, Selector};
use url::Url;

use crate::{
    ConnectionMode, CsustSession,
    request::{LoginCheck, RequestResponse, perform_request},
    url_factory::{ServiceDomain, make_url},
    webvpn::{WEBVPN_HOST, webvpn_decrypt_url, webvpn_encrypt_url},
};

use super::{Assignment, AssignmentStatus, ChaoxingError, Profile};

/// 通过共享的 SSO 会话访问学习通。
pub struct ChaoxingHelper {
    mode: ConnectionMode,
    session: CsustSession,
}

impl ChaoxingHelper {
    pub fn new(mode: ConnectionMode, session: CsustSession) -> Self {
        Self { mode, session }
    }

    pub async fn is_logged_in(&self) -> bool {
        self.get_profile().await.is_ok()
    }

    pub async fn get_profile(&self) -> Result<Profile, ChaoxingError> {
        let response = self.get(ServiceDomain::Chaoxing, "/space/index").await?;
        let document = Html::parse_document(&response.text());
        let selector = Selector::parse("p.personalName").expect("valid profile selector");
        let element = document
            .select(&selector)
            .next()
            .ok_or_else(|| ChaoxingError::ProfileRetrievalFailed("未找到姓名元素".to_owned()))?;
        let name = element
            .value()
            .attr("title")
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| element_text(element));
        Ok(Profile { name })
    }

    /// 获取全部作业。服务端标记仍有更多数据时返回错误。
    pub async fn get_assignments(&self) -> Result<Vec<Assignment>, ChaoxingError> {
        let response = self
            .get(ServiceDomain::ChaoxingApi, "/mooc-ans/work/stu-work")
            .await?;
        let html = response.text();
        let document = Html::parse_document(&html);
        let list = Selector::parse("#content ul.nav").expect("valid assignment list selector");
        let list = document
            .select(&list)
            .next()
            .ok_or_else(|| assignments_error("未找到作业列表"))?;
        if !html.contains("var haveMore = \"0\";") {
            return Err(assignments_error("作业列表未完整返回"));
        }

        let items = Selector::parse("li").expect("valid assignment item selector");
        let titles =
            Selector::parse("div[role=option] > p").expect("valid assignment title selector");
        let icons = Selector::parse("img").expect("valid assignment icon selector");
        let metadata =
            Selector::parse("div[role=option] > span").expect("valid assignment metadata selector");
        let times = Selector::parse("span.fr").expect("valid assignment time selector");
        let now = DateTime::<Utc>::from(SystemTime::now());

        list.select(&items)
            .map(|item| {
                let title = item
                    .select(&titles)
                    .next()
                    .map(element_text)
                    .ok_or_else(|| assignments_error("作业条目缺少名称"))?;
                let raw_icon_url = item
                    .select(&icons)
                    .next()
                    .and_then(|icon| icon.value().attr("src"))
                    .ok_or_else(|| assignments_error("作业条目缺少图标链接"))?;
                let mut details = item.select(&metadata);
                let (Some(status), Some(course)) = (details.next(), details.next()) else {
                    return Err(assignments_error("作业条目结构异常"));
                };
                let status_text = element_text(status);
                let status = AssignmentStatus::from_display_name(&status_text)
                    .ok_or_else(|| assignments_error(format!("未知的作业状态: {status_text}")))?;
                let deadline = item
                    .select(&times)
                    .next()
                    .map(|element| {
                        let remaining = element_text(element);
                        parse_deadline(&remaining, now).ok_or_else(|| {
                            assignments_error(format!("剩余时间格式异常: {remaining}"))
                        })
                    })
                    .transpose()?;
                let raw_detail_url = item
                    .value()
                    .attr("data")
                    .ok_or_else(|| assignments_error("作业条目缺少详情链接"))?;

                let mut icon_url = parse_url(raw_icon_url, "图标")?;
                let mut detail_url = parse_url(raw_detail_url, "详情")?;
                if self.mode == ConnectionMode::WebVpn {
                    if icon_url.host_str() == Some(WEBVPN_HOST) {
                        icon_url = webvpn_decrypt_url(icon_url)?;
                    }
                    if detail_url.host_str() != Some(WEBVPN_HOST) {
                        detail_url = webvpn_encrypt_url(detail_url)?;
                    }
                }
                Ok(Assignment {
                    title,
                    status,
                    course_name: element_text(course),
                    deadline,
                    icon_url,
                    detail_url,
                })
            })
            .collect()
    }

    async fn get(
        &self,
        domain: ServiceDomain,
        path: &str,
    ) -> Result<RequestResponse, ChaoxingError> {
        let url = make_url(self.mode, domain, path);
        perform_request(
            self.session
                .client
                .get(url)
                .header(ACCEPT_LANGUAGE, "zh-CN,zh-Hans;q=0.9"),
            Some(LoginCheck {
                mode: self.mode,
                fallback_encoding: UTF_8,
                subsystem_login: |body, _| {
                    body.contains("<title>用户登录</title>") || body.contains("请输入账号")
                },
                not_logged_in: ChaoxingError::NotLoggedIn,
            }),
        )
        .await
    }
}

fn parse_url(value: &str, kind: &str) -> Result<Url, ChaoxingError> {
    let url = if value.starts_with("//") {
        Url::parse(&format!("https:{value}"))
    } else {
        Url::parse(value)
    }
    .ok()
    .filter(|url| matches!(url.scheme(), "http" | "https") && url.has_host());
    url.ok_or_else(|| assignments_error(format!("作业{kind}链接格式异常")))
}

fn parse_deadline(text: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let text = text.strip_prefix("剩余")?;
    let (hours, remainder) = match text.split_once("小时") {
        Some((hours, remainder)) => (Some(parse_nonnegative_int(hours)?), remainder),
        None => (None, text),
    };
    let minutes = match remainder.strip_suffix("分钟") {
        Some(minutes) => parse_nonnegative_int(minutes)?,
        None if hours.is_some() && remainder.is_empty() => 0,
        None => return None,
    };
    let seconds = hours
        .unwrap_or(0)
        .checked_mul(3600)?
        .checked_add(minutes.checked_mul(60)?)?;
    now.checked_add_signed(TimeDelta::try_seconds(seconds)?)
}

fn parse_nonnegative_int(text: &str) -> Option<i64> {
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

fn element_text(element: ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_owned()
}

fn assignments_error(message: impl Into<String>) -> ChaoxingError {
    ChaoxingError::AssignmentsRetrievalFailed(message.into())
}
