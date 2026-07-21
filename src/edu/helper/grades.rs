use reqwest::header::CONTENT_TYPE;
use scraper::{ElementRef, Html, Selector};
use url::{Url, form_urlencoded};

use crate::{
    connection::ConnectionMode,
    url_factory::{ServiceDomain, make_url},
};

use super::super::{
    CourseGrade, CourseGradeQuery, CourseNature, GradeComponent, GradeDetail, types::EduError,
};
use super::{EduHelper, element_text, ensure_logged_in};

const COURSE_GRADES_PATH: &str = "/jsxsd/kscj/cjcx_list";
const COURSE_GRADE_SEMESTERS_PATH: &str = "/jsxsd/kscj/cjcx_query";

impl EduHelper {
    pub async fn get_course_grades(
        &self,
        query: CourseGradeQuery,
    ) -> Result<Vec<CourseGrade>, EduError> {
        let url = make_url(self.mode, ServiceDomain::Education, COURSE_GRADES_PATH);
        let academic_year_semester = query.academic_year_semester.as_deref().unwrap_or("");
        let course_nature = query
            .course_nature
            .map(CourseNature::request_id)
            .unwrap_or("");
        let mut form = form_urlencoded::Serializer::new(String::new());
        form.append_pair("fxkc", query.study_mode.request_id());
        form.append_pair("kcmc", &query.course_name);
        form.append_pair("kcxz", course_nature);
        form.append_pair("kksj", academic_year_semester);
        form.append_pair("xsfs", query.display_mode.request_id());
        let form = form.finish();
        let response = self.session.send_with_retry(|| {
            self.session
                .client
                .post(&url)
                .header(
                    CONTENT_TYPE,
                    "application/x-www-form-urlencoded; charset=utf-8",
                )
                .body(form.clone())
        })
        .await?;
        let body = String::from_utf8_lossy(&response.body);

        ensure_logged_in(&body)?;
        parse_course_grades(self.mode, &body)
    }

    pub async fn get_available_semesters_for_course_grades(&self) -> Result<Vec<String>, EduError> {
        let url = make_url(
            self.mode,
            ServiceDomain::Education,
            COURSE_GRADE_SEMESTERS_PATH,
        );
        let response = self
            .session
            .send_with_retry(|| self.session.client.get(&url))
            .await?;
        let body = String::from_utf8_lossy(&response.body);

        ensure_logged_in(&body)?;
        parse_available_semesters_for_course_grades(&body)
    }

    pub async fn get_grade_detail(&self, url: &Url) -> Result<GradeDetail, EduError> {
        let response = self
            .session
            .send_with_retry(|| self.session.client.get(url.clone()))
            .await?;
        let body = String::from_utf8_lossy(&response.body);

        ensure_logged_in(&body)?;
        parse_grade_detail(&body)
    }
}

fn parse_course_grades(mode: ConnectionMode, body: &str) -> Result<Vec<CourseGrade>, EduError> {
    let document = Html::parse_document(body);
    let table_selector = Selector::parse("#dataList")
        .map_err(|error| course_grades_error(format!("解析课程成绩表格选择器失败: {error}")))?;
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| course_grades_error("未找到课程成绩表格"))?;
    if table.inner_html().contains("未查询到数据") {
        return Err(course_grades_error("未查询到数据"));
    }

    let row_selector = Selector::parse("tr")
        .map_err(|error| course_grades_error(format!("解析课程成绩行选择器失败: {error}")))?;
    let cell_selector = Selector::parse("td")
        .map_err(|error| course_grades_error(format!("解析课程成绩列选择器失败: {error}")))?;
    let link_selector = Selector::parse("a")
        .map_err(|error| course_grades_error(format!("解析成绩详情链接选择器失败: {error}")))?;
    let mut grades = Vec::new();

    for row in table.select(&row_selector).skip(1) {
        let cells = row.select(&cell_selector).collect::<Vec<_>>();
        if cells.len() < 17 {
            return Err(course_grades_error(format!("行列数不足: {}", cells.len())));
        }

        let grade_text = course_grade_cell(&cells, 5)?;
        let grade = parse_i64(&grade_text, "成绩", course_grades_error)?;
        let grade_link = cells
            .get(5)
            .and_then(|cell| cell.select(&link_selector).next())
            .and_then(|link| link.value().attr("href"))
            .map(str::trim)
            .filter(|href| !href.is_empty())
            .ok_or_else(|| course_grades_error("未找到成绩详情URL"))?;
        let course_nature_text = course_grade_cell(&cells, 15)?;
        let course_nature = CourseNature::from_display_name(&course_nature_text)
            .ok_or_else(|| course_grades_error(format!("课程性质无效: {course_nature_text}")))?;

        grades.push(CourseGrade {
            semester: course_grade_cell(&cells, 1)?,
            course_id: course_grade_cell(&cells, 2)?,
            course_name: course_grade_cell(&cells, 3)?,
            group_name: course_grade_cell(&cells, 4)?,
            grade,
            grade_detail_url: parse_grade_detail_url(mode, grade_link)?,
            study_mode: course_grade_cell(&cells, 6)?,
            grade_identifier: course_grade_cell(&cells, 7)?,
            credit: parse_f64(&course_grade_cell(&cells, 8)?, "学分", course_grades_error)?,
            total_hours: parse_f64(
                &course_grade_cell(&cells, 9)?,
                "总学时",
                course_grades_error,
            )?,
            grade_point: parse_f64(&course_grade_cell(&cells, 10)?, "绩点", course_grades_error)?,
            retake_semester: course_grade_cell(&cells, 11)?,
            assessment_method: course_grade_cell(&cells, 12)?,
            exam_nature: course_grade_cell(&cells, 13)?,
            course_attribute: course_grade_cell(&cells, 14)?,
            course_nature,
            course_category: course_grade_cell(&cells, 16)?,
        });
    }

    Ok(grades)
}

fn parse_grade_detail_url(mode: ConnectionMode, value: &str) -> Result<Url, EduError> {
    const PREFIX: &str = "javascript:openWindow('/";
    const SUFFIX: &str = "',700,500)";

    let relative_path = value
        .strip_prefix(PREFIX)
        .and_then(|value| value.strip_suffix(SUFFIX))
        .ok_or_else(|| course_grades_error(format!("成绩详情URL格式无效: {value}")))?;
    let url = make_url(mode, ServiceDomain::Education, &format!("/{relative_path}"));
    Url::parse(&url).map_err(|error| course_grades_error(format!("成绩详情URL解析失败: {error}")))
}

fn parse_available_semesters_for_course_grades(body: &str) -> Result<Vec<String>, EduError> {
    let document = Html::parse_document(body);
    let select_selector = Selector::parse("#kksj")
        .map_err(|error| available_semesters_error(format!("解析学期选择器失败: {error}")))?;
    let semester_select = document
        .select(&select_selector)
        .next()
        .ok_or_else(|| available_semesters_error("未找到学期选择元素"))?;
    let option_selector = Selector::parse("option")
        .map_err(|error| available_semesters_error(format!("解析学期选项失败: {error}")))?;

    Ok(semester_select
        .select(&option_selector)
        .map(element_text)
        .filter(|semester| !semester.contains("全部学期"))
        .collect())
}

fn parse_grade_detail(body: &str) -> Result<GradeDetail, EduError> {
    let document = Html::parse_document(body);
    let table_selector = Selector::parse("#dataList")
        .map_err(|error| grade_detail_error(format!("解析成绩详情表格选择器失败: {error}")))?;
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| grade_detail_error("未找到成绩详情表格"))?;
    let row_selector = Selector::parse("tr")
        .map_err(|error| grade_detail_error(format!("解析成绩详情行选择器失败: {error}")))?;
    let rows = table.select(&row_selector).collect::<Vec<_>>();
    if rows.len() < 2 {
        return Err(grade_detail_error("成绩详情表行数不足"));
    }

    let header_selector = Selector::parse("th")
        .map_err(|error| grade_detail_error(format!("解析成绩详情表头失败: {error}")))?;
    let value_selector = Selector::parse("td")
        .map_err(|error| grade_detail_error(format!("解析成绩详情数据列失败: {error}")))?;
    let header_columns = rows[0].select(&header_selector).collect::<Vec<_>>();
    let value_columns = rows[1].select(&value_selector).collect::<Vec<_>>();
    if header_columns.len() < 4 || value_columns.len() < 4 {
        return Err(grade_detail_error(format!(
            "成绩详情表列数不足: {}, {}",
            header_columns.len(),
            value_columns.len()
        )));
    }

    let mut components = Vec::new();
    for index in (1..header_columns.len() - 1).step_by(2) {
        let grade_column = value_columns
            .get(index)
            .copied()
            .ok_or_else(|| grade_detail_error(format!("成绩列索引越界: {index}")))?;
        let ratio_index = index + 1;
        let ratio_column = value_columns
            .get(ratio_index)
            .copied()
            .ok_or_else(|| grade_detail_error(format!("比例列索引越界: {ratio_index}")))?;
        let grade_text = element_text(grade_column);
        let ratio_text = element_text(ratio_column).replace('%', "");

        components.push(GradeComponent {
            component_type: element_text(header_columns[index]),
            grade: parse_f64(&grade_text, "成绩", grade_detail_error)?,
            ratio: parse_i64(&ratio_text, "比例", grade_detail_error)?,
        });
    }

    let total_grade_text = value_columns
        .last()
        .copied()
        .map(element_text)
        .ok_or_else(|| grade_detail_error("未找到总成绩"))?;
    let total_grade = parse_i64(&total_grade_text, "总成绩", grade_detail_error)?;

    Ok(GradeDetail {
        components,
        total_grade,
    })
}

fn course_grade_cell(cells: &[ElementRef<'_>], index: usize) -> Result<String, EduError> {
    cells
        .get(index)
        .copied()
        .map(element_text)
        .ok_or_else(|| course_grades_error(format!("列索引越界: {index}")))
}

fn parse_i64(value: &str, field: &str, error: fn(String) -> EduError) -> Result<i64, EduError> {
    value
        .parse::<i64>()
        .map_err(|_| error(format!("{field}格式无效: {value}")))
}

fn parse_f64(value: &str, field: &str, error: fn(String) -> EduError) -> Result<f64, EduError> {
    value
        .parse::<f64>()
        .map_err(|_| error(format!("{field}格式无效: {value}")))
}

fn course_grades_error(message: impl Into<String>) -> EduError {
    EduError::CourseGradesRetrievalFailed(message.into())
}

fn available_semesters_error(message: impl Into<String>) -> EduError {
    EduError::AvailableSemestersForCourseGradesRetrievalFailed(message.into())
}

fn grade_detail_error(message: impl Into<String>) -> EduError {
    EduError::GradeDetailRetrievalFailed(message.into())
}
