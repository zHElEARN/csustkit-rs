use scraper::{ElementRef, Html, Selector};

use crate::url_factory::{ServiceDomain, make_url};

use super::super::{Profile, types::EduError};
use super::EduHelper;

const PROFILE_PATH: &str = "/jsxsd/grxx/xsxx";

impl EduHelper {
    pub async fn get_profile(&self) -> Result<Profile, EduError> {
        let url = make_url(self.mode, ServiceDomain::Education, PROFILE_PATH);
        let response = self
            .session
            .send_with_retry(|| self.session.client.get(&url))
            .await?;
        let body = String::from_utf8_lossy(&response.body);

        if body.contains("请输入账号") {
            return Err(EduError::NotLoggedIn);
        }

        parse_profile(&body)
    }
}

fn parse_profile(body: &str) -> Result<Profile, EduError> {
    let document = Html::parse_document(body);
    let table_selector = selector("#xjkpTable > tbody")?;
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| profile_error("未找到个人信息表"))?;
    let row_selector = selector("tr")?;
    let rows = table.select(&row_selector).collect::<Vec<_>>();

    let department = labeled_value(&table_cell(&rows, 2, 0)?, "院系")?;
    let major = labeled_value(&table_cell(&rows, 2, 1)?, "专业")?;
    let education_system = labeled_value(&table_cell(&rows, 2, 2)?, "学制")?;
    let class_name = labeled_value(&table_cell(&rows, 2, 3)?, "班级")?;
    let student_id = labeled_value(&table_cell(&rows, 2, 4)?, "学号")?;

    Ok(Profile {
        department,
        major,
        education_system,
        class_name,
        student_id,
        name: table_cell(&rows, 3, 1)?,
        gender: table_cell(&rows, 3, 3)?,
        name_pinyin: table_cell(&rows, 3, 5)?,
        birth_date: table_cell(&rows, 4, 1)?,
        ethnicity: table_cell(&rows, 7, 3)?,
        study_level: table_cell(&rows, 8, 3)?,
        home_address: table_cell(&rows, 9, 1)?,
        home_phone: table_cell(&rows, 10, 3)?,
        personal_phone: table_cell(&rows, 4, 5)?,
        enrollment_date: table_cell(&rows, 46, 1)?,
        entrance_exam_id: table_cell(&rows, 47, 1)?,
        id_card_number: table_cell(&rows, 47, 3)?,
    })
}

fn table_cell(
    rows: &[ElementRef<'_>],
    row_index: usize,
    column_index: usize,
) -> Result<String, EduError> {
    let row = rows
        .get(row_index)
        .ok_or_else(|| profile_error(format!("行索引越界: {row_index}")))?;
    let cell_selector = selector("td")?;
    let cells = row.select(&cell_selector).collect::<Vec<_>>();
    let cell = cells
        .get(column_index)
        .ok_or_else(|| profile_error(format!("列索引越界: {column_index}")))?;
    Ok(cell.text().collect::<String>().trim().to_owned())
}

fn labeled_value(value: &str, field: &str) -> Result<String, EduError> {
    value
        .split('：')
        .nth(1)
        .map(str::trim)
        .map(str::to_owned)
        .ok_or_else(|| profile_error(format!("{field}字段格式无效: {value}")))
}

fn selector(value: &str) -> Result<Selector, EduError> {
    Selector::parse(value)
        .map_err(|error| profile_error(format!("解析个人信息选择器失败: {error}")))
}

fn profile_error(message: impl Into<String>) -> EduError {
    EduError::ProfileRetrievalFailed(message.into())
}
