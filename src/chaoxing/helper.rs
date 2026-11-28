use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeDelta, TimeZone, Utc};
use encoding_rs::UTF_8;
use reqwest::{StatusCode, header::ACCEPT_LANGUAGE};
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct ChaoxingHelper {
    mode: ConnectionMode,
    session: CsustSession,
}

impl ChaoxingHelper {
    pub fn new(mode: ConnectionMode, session: CsustSession) -> Self {
        Self { mode, session }
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export(async_runtime = "tokio"))]
impl ChaoxingHelper {
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
        let response = match self
            .get(ServiceDomain::ChaoxingApi, "/mooc-ans/work/stu-work")
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let kind = if matches!(error, ChaoxingError::NotLoggedIn) {
                    "not_logged_in"
                } else {
                    "request_failed"
                };
                assignment_diagnostic(kind);
                return Err(error);
            }
        };
        if response.status == StatusCode::UNAUTHORIZED {
            assignment_diagnostic("not_logged_in: HTTP 401");
            return Err(ChaoxingError::NotLoggedIn);
        }
        if !response.status.is_success() {
            let message = format!("获取作业列表的 HTTP 状态为 {}", response.status.as_u16());
            assignment_diagnostic(&message);
            return Err(assignments_error(message));
        }
        parse_assignments(
            &response.text(),
            self.mode,
            DateTime::<Utc>::from(SystemTime::now()),
        )
    }
}

// A malformed row must not discard otherwise valid assignments.
fn parse_assignments(
    html: &str,
    mode: ConnectionMode,
    now: DateTime<Utc>,
) -> Result<Vec<Assignment>, ChaoxingError> {
    let document = Html::parse_document(html);
    let list_selector = Selector::parse("#content ul.nav").expect("valid assignment list selector");
    let Some(list) = document.select(&list_selector).next() else {
        assignment_diagnostic("list_missing");
        return Err(assignments_error("未找到作业列表"));
    };
    match has_more_assignments(&document) {
        Some(false) => {}
        Some(true) => {
            assignment_diagnostic("more_pages");
            return Err(assignments_error("作业列表还有下一页"));
        }
        None => {
            assignment_diagnostic("pagination_unknown");
            return Err(assignments_error("无法确认作业列表是否完整"));
        }
    }

    let titles = Selector::parse("div[role=option] > p").expect("valid assignment title selector");
    let icons = Selector::parse("img").expect("valid assignment icon selector");
    let metadata =
        Selector::parse("div[role=option] > span").expect("valid assignment metadata selector");
    let times = Selector::parse("span.fr").expect("valid assignment time selector");
    let mut assignments = Vec::new();
    let mut skipped = 0;
    let mut unknown_status = 0;
    let mut unknown_deadline = 0;
    let mut missing_icon = 0;

    for item in list
        .child_elements()
        .filter(|item| item.value().name() == "li")
    {
        let Some(title) = item
            .select(&titles)
            .next()
            .map(element_text)
            .filter(|text| !text.is_empty())
        else {
            skipped += 1;
            continue;
        };
        let mut details = item.select(&metadata);
        let (Some(status), Some(course)) = (details.next(), details.next()) else {
            skipped += 1;
            continue;
        };
        let Some(status) = AssignmentStatus::from_display_name(&element_text(status)) else {
            unknown_status += 1;
            continue;
        };
        let detail_url = item
            .value()
            .attr("data")
            .and_then(parse_url)
            .and_then(|url| {
                if mode == ConnectionMode::WebVpn && url.host_str() != Some(WEBVPN_HOST) {
                    webvpn_encrypt_url(url).ok()
                } else {
                    Some(url)
                }
            });
        let Some(detail_url) = detail_url else {
            skipped += 1;
            continue;
        };

        let icon_url = item
            .select(&icons)
            .next()
            .and_then(|icon| icon.value().attr("src"))
            .and_then(parse_url)
            .and_then(|url| {
                if mode == ConnectionMode::WebVpn && url.host_str() == Some(WEBVPN_HOST) {
                    webvpn_decrypt_url(url).ok()
                } else {
                    Some(url)
                }
            })
            .unwrap_or_else(|| {
                missing_icon += 1;
                Url::parse("about:blank").expect("valid local placeholder URL")
            });
        let deadline = item.select(&times).next().and_then(|element| {
            let value = parse_deadline(&element_text(element), now);
            if value.is_none() {
                unknown_deadline += 1;
            }
            value
        });
        assignments.push(Assignment {
            title,
            status,
            course_name: element_text(course),
            deadline,
            icon_url,
            detail_url,
        });
    }

    assignment_diagnostic(&format!(
        "rows={} skipped={skipped} unknown_status={unknown_status} unknown_deadline={unknown_deadline} missing_icon={missing_icon} more_pages=false",
        assignments.len()
    ));
    if assignments.is_empty() && skipped + unknown_status > 0 {
        return Err(assignments_error("页面包含作业，但没有可解析的作业条目"));
    }
    Ok(assignments)
}

fn has_more_assignments(document: &Html) -> Option<bool> {
    let scripts = Selector::parse("script").expect("valid script selector");
    let mut complete = false;
    for script in document.select(&scripts) {
        let text = script.text().collect::<String>();
        for statement in text.split(';') {
            let Some((key, value)) = statement.split_once('=') else {
                continue;
            };
            let mut words = key.split_whitespace();
            let Some(first) = words.next() else {
                continue;
            };
            let name = if matches!(first, "var" | "let" | "const") {
                words.next().unwrap_or_default()
            } else {
                first
            };
            if name != "haveMore" || words.next().is_some() {
                continue;
            }
            match value.trim() {
                "0" | "'0'" | "\"0\"" => complete = true,
                "1" | "'1'" | "\"1\"" => return Some(true),
                _ => return None,
            }
        }
    }
    complete.then_some(false)
}

impl ChaoxingHelper {
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

fn parse_url(value: &str) -> Option<Url> {
    let url = if value.starts_with("//") {
        Url::parse(&format!("https:{value}"))
    } else {
        Url::parse(value)
    }
    .ok()?;
    (matches!(url.scheme(), "http" | "https") && url.has_host()).then_some(url)
}

fn parse_deadline(text: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let text = text.trim();
    if let Some(remaining) = text.strip_prefix("剩余") {
        let mut remaining = remaining.trim();
        let mut seconds = 0_i64;
        let mut previous_unit = i64::MAX;
        while !remaining.is_empty() {
            let digits = remaining.bytes().take_while(u8::is_ascii_digit).count();
            let count: i64 = remaining[..digits].parse().ok()?;
            let suffix = remaining[digits..].trim_start();
            let (unit, rest) = [
                ("天", 86_400),
                ("小时", 3_600),
                ("时", 3_600),
                ("分钟", 60),
                ("分", 60),
            ]
            .into_iter()
            .find_map(|(label, unit)| suffix.strip_prefix(label).map(|rest| (unit, rest)))?;
            if unit >= previous_unit {
                return None;
            }
            seconds = seconds.checked_add(count.checked_mul(unit)?)?;
            previous_unit = unit;
            remaining = rest.trim_start();
        }
        if previous_unit == i64::MAX {
            return None;
        }
        return now.checked_add_signed(TimeDelta::try_seconds(seconds)?);
    }

    let text = text
        .strip_prefix("截止时间")
        .unwrap_or(text)
        .trim_start_matches([':', '：', ' ']);
    if let Ok(deadline) = DateTime::parse_from_rfc3339(text) {
        return Some(deadline.to_utc());
    }
    let local = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y年%m月%d日 %H:%M",
    ]
    .into_iter()
    .find_map(|format| NaiveDateTime::parse_from_str(text, format).ok())?;
    FixedOffset::east_opt(8 * 3600)?
        .from_local_datetime(&local)
        .single()
        .map(|deadline| deadline.to_utc())
}

#[cfg(target_os = "android")]
fn assignment_diagnostic(message: &str) {
    use std::ffi::{CString, c_char, c_int};

    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    if let Ok(message) = CString::new(message) {
        // Both pointers remain valid for the duration of this call.
        unsafe { __android_log_write(4, c"CsustkitAssignments".as_ptr(), message.as_ptr()) };
    }
}

#[cfg(not(target_os = "android"))]
fn assignment_diagnostic(_: &str) {}

fn element_text(element: ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_owned()
}

fn assignments_error(message: impl Into<String>) -> ChaoxingError {
    ChaoxingError::AssignmentsRetrievalFailed(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-28T12:00:00Z")
            .unwrap()
            .to_utc()
    }

    #[test]
    fn keeps_valid_rows_when_other_rows_or_optional_fields_are_invalid() {
        let html = r#"
            <script>var haveMore = '0';</script>
            <div id="content"><ul class="nav">
              <li data="https://example.org/detail/1">
                <div role="option"><p>作业一</p><span>未提交</span><span>课程</span></div>
                <span class="fr">时间未定</span>
                <ul><li>嵌套说明</li></ul>
              </li>
              <li data="https://example.org/detail/2">
                <div role="option"><p>作业二</p><span>已完成</span><span>课程</span></div>
                <img src="//example.org/icon.png">
                <span class="fr">剩余 1 天 2 小时 3 分</span>
              </li>
              <li data="https://example.org/detail/3">
                <div role="option"><p>未知状态</p><span>其他状态</span><span>课程</span></div>
              </li>
              <li><div role="option"><p>无详情</p><span>未提交</span><span>课程</span></div></li>
            </ul></div>
        "#;

        let assignments = parse_assignments(html, ConnectionMode::Direct, now()).unwrap();
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].status, AssignmentStatus::Unsubmitted);
        assert_eq!(assignments[0].deadline, None);
        assert_eq!(assignments[0].icon_url.as_str(), "about:blank");
        assert_eq!(assignments[1].status, AssignmentStatus::Completed);
        assert_eq!(
            assignments[1].deadline,
            now().checked_add_signed(TimeDelta::minutes(26 * 60 + 3))
        );
    }

    #[test]
    fn distinguishes_an_empty_list_an_unusable_list_and_more_pages() {
        let empty = r#"<div id="content"><ul class="nav"></ul></div>"#;
        let invalid = r#"<div id="content"><ul class="nav"><li>坏数据</li></ul></div>"#;
        let complete = "<script>let haveMore=0;</script>";
        let fixture = format!("{empty}{complete}");
        assert!(
            parse_assignments(&fixture, ConnectionMode::Direct, now())
                .unwrap()
                .is_empty()
        );
        let fixture = format!("{invalid}{complete}");
        assert!(matches!(
            parse_assignments(&fixture, ConnectionMode::Direct, now()),
            Err(ChaoxingError::AssignmentsRetrievalFailed(message))
                if message == "页面包含作业，但没有可解析的作业条目"
        ));
        let fixture = format!("{empty}<script>var haveMore=1;</script>");
        assert!(matches!(
            parse_assignments(&fixture, ConnectionMode::Direct, now()),
            Err(ChaoxingError::AssignmentsRetrievalFailed(message))
                if message == "作业列表还有下一页"
        ));
    }

    #[test]
    fn pagination_ignores_page_text_and_never_prefers_zero_over_a_next_page() {
        let document = Html::parse_document(r#"<div>var haveMore = "0";</div>"#);
        assert_eq!(has_more_assignments(&document), None);
        let document = Html::parse_document(
            r#"<script>let   haveMore='0';</script><script>haveMore=1;</script>"#,
        );
        assert_eq!(has_more_assignments(&document), Some(true));
        let document = Html::parse_document(r#"<script>const haveMore = '0';</script>"#);
        assert_eq!(has_more_assignments(&document), Some(false));
    }

    #[test]
    fn parses_local_absolute_dates_and_rejects_unfinished_time_strings() {
        assert_eq!(
            parse_deadline("截止时间：2026-11-28 20:00", now()),
            Some(now())
        );
        assert_eq!(
            parse_deadline("剩余 2 小时 10 分钟", now()),
            now().checked_add_signed(TimeDelta::minutes(130))
        );
        assert_eq!(parse_deadline("剩余 2", now()), None);
        assert_eq!(parse_deadline("剩余 2分 1小时", now()), None);
    }
}
