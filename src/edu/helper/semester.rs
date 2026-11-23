use chrono::{Days, NaiveDate};
use reqwest::header::CONTENT_TYPE;
use scraper::{Html, Selector};

use crate::url_factory::{ServiceDomain, make_url};

use super::super::{SemesterOptions, types::EduError};
use super::{EduHelper, element_text};

const SEMESTER_START_DATE_PATH: &str = "/jsxsd/jxzl/jxzl_query";

impl EduHelper {
    pub async fn get_semester_start_date(
        &self,
        academic_year_semester: Option<String>,
    ) -> Result<NaiveDate, EduError> {
        let url = make_url(
            self.mode,
            ServiceDomain::Education,
            SEMESTER_START_DATE_PATH,
        );
        let params = [("xnxq01id", academic_year_semester.as_deref().unwrap_or(""))];
        let response = self
            .request(|| {
                self.session
                    .client
                    .post(&url)
                    .header(
                        CONTENT_TYPE,
                        "application/x-www-form-urlencoded; charset=utf-8",
                    )
                    .form(&params)
            })
            .await?;
        let body = response.text();

        parse_semester_start_date(&body)
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export(async_runtime = "tokio"))]
impl EduHelper {
    pub async fn get_available_semesters_for_start_date(
        &self,
    ) -> Result<SemesterOptions, EduError> {
        let url = make_url(
            self.mode,
            ServiceDomain::Education,
            SEMESTER_START_DATE_PATH,
        );
        let response = self.request(|| self.session.client.get(&url)).await?;
        let body = response.text();

        parse_available_semesters_for_start_date(&body)
    }
}

fn parse_semester_start_date(body: &str) -> Result<NaiveDate, EduError> {
    let document = Html::parse_document(body);
    let table_selector = Selector::parse("#kbtable").map_err(|error| {
        semester_start_date_error(format!("解析学期首日表格选择器失败: {error}"))
    })?;
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| semester_start_date_error("未找到学期首日表"))?;
    let row_selector = Selector::parse("tr")
        .map_err(|error| semester_start_date_error(format!("解析学期首日行选择器失败: {error}")))?;
    let second_week_row = table
        .select(&row_selector)
        .nth(2)
        .ok_or_else(|| semester_start_date_error("学期首日表行数不足"))?;
    let cell_selector = Selector::parse("td")
        .map_err(|error| semester_start_date_error(format!("解析学期首日列选择器失败: {error}")))?;
    let second_week_date_text = second_week_row
        .select(&cell_selector)
        .nth(1)
        .ok_or_else(|| semester_start_date_error("目标行列数不足"))?
        .value()
        .attr("title")
        .unwrap_or_default()
        .trim();

    let second_week_date =
        NaiveDate::parse_from_str(second_week_date_text, "%Y年%m月%d").map_err(|_| {
            semester_start_date_error(format!("无法解析第二周周日日期: {second_week_date_text}"))
        })?;
    second_week_date
        .checked_sub_days(Days::new(7))
        .ok_or_else(|| semester_start_date_error("无法回推学期首日"))
}

fn parse_available_semesters_for_start_date(body: &str) -> Result<SemesterOptions, EduError> {
    let document = Html::parse_document(body);
    let select_selector = Selector::parse("#xnxq01id")
        .map_err(|error| start_date_semesters_error(format!("解析学期选择器失败: {error}")))?;
    let select = document
        .select(&select_selector)
        .next()
        .ok_or_else(|| start_date_semesters_error("未找到学期选择元素"))?;
    let option_selector = Selector::parse("option")
        .map_err(|error| start_date_semesters_error(format!("解析学期选项失败: {error}")))?;
    let options = select.select(&option_selector).collect::<Vec<_>>();
    if options.is_empty() {
        return Err(start_date_semesters_error("学期选择元素中未找到学期"));
    }

    let mut semesters = Vec::with_capacity(options.len());
    let mut default_semester = None;
    for option in options {
        let semester = element_text(option);
        if option.value().attr("selected").is_some() {
            default_semester = Some(semester.clone());
        }
        semesters.push(semester);
    }
    let default_semester =
        default_semester.ok_or_else(|| start_date_semesters_error("未找到默认学期"))?;

    Ok(SemesterOptions {
        semesters,
        default_semester,
    })
}

fn semester_start_date_error(message: impl Into<String>) -> EduError {
    EduError::SemesterStartDateRetrievalFailed(message.into())
}

fn start_date_semesters_error(message: impl Into<String>) -> EduError {
    EduError::AvailableSemestersForStartDateRetrievalFailed(message.into())
}
