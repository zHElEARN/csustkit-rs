//! 学校绩点档位与成绩统计。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::edu::CourseGrade;

/// 一个百分制成绩档位及其对应绩点。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct GradeScale {
    pub min_grade: i64,
    pub max_grade: i64,
    pub grade_point: f64,
}

impl GradeScale {
    /// 学校标准档位，按成绩从高到低排列。
    pub const STANDARD: [Self; 11] = [
        Self::new(90, 100, 4.0),
        Self::new(85, 89, 3.7),
        Self::new(82, 84, 3.3),
        Self::new(78, 81, 3.0),
        Self::new(75, 77, 2.7),
        Self::new(72, 74, 2.3),
        Self::new(68, 71, 2.0),
        Self::new(66, 67, 1.7),
        Self::new(64, 65, 1.3),
        Self::new(60, 63, 1.0),
        Self::new(0, 59, 0.0),
    ];

    pub const fn new(min_grade: i64, max_grade: i64, grade_point: f64) -> Self {
        Self {
            min_grade,
            max_grade,
            grade_point,
        }
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
impl GradeScale {
    /// 显示用的成绩段，例如 `90-100` 或 `≤59`。
    pub fn display_range(self) -> String {
        if self.min_grade <= 0 {
            format!("≤{}", self.max_grade)
        } else {
            format!("{}-{}", self.min_grade, self.max_grade)
        }
    }
}

impl GradeScale {
    /// 查询百分制成绩对应的绩点。
    pub fn grade_point(grade: i64) -> Option<f64> {
        Self::STANDARD
            .iter()
            .find(|scale| (scale.min_grade..=scale.max_grade).contains(&grade))
            .map(|scale| scale.grade_point)
    }

    /// 查询绩点对应的档位，容许小于 0.0001 的浮点误差。
    pub fn scale_for_grade_point(point: f64) -> Option<Self> {
        Self::STANDARD
            .into_iter()
            .find(|scale| (scale.grade_point - point).abs() < 0.0001)
    }
}

/// 一个学期的算术平均分和学分加权 GPA。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct SemesterSummary {
    pub semester: String,
    pub average_grade: f64,
    pub gpa: f64,
}

/// 具有同一绩点的课程数。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct GradePointBucket {
    pub grade_point: f64,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct GradeStatistics {
    pub total_courses: usize,
    pub total_hours: f64,
    pub total_credits: f64,
    pub average_grade: f64,
    pub weighted_average_grade: f64,
    pub gpa: f64,
    /// 按绩点从高到低排列。
    pub grade_point_distribution: Vec<GradePointBucket>,
    /// 按学期升序排列。
    pub semesters: Vec<SemesterSummary>,
}

impl GradeStatistics {
    /// 按教务返回的成绩、学分和绩点计算统计结果。
    pub fn from_course_grades(course_grades: &[CourseGrade]) -> Self {
        let mut totals = GradeTotals::default();
        let mut semesters = BTreeMap::<&str, GradeTotals>::new();
        let mut grade_points = Vec::with_capacity(course_grades.len());
        for grade in course_grades {
            totals.add(grade);
            semesters.entry(&grade.semester).or_default().add(grade);
            grade_points.push(grade.grade_point);
        }
        grade_points.sort_unstable_by(|left, right| right.total_cmp(left));

        Self {
            total_courses: totals.count,
            total_hours: totals.hours,
            total_credits: totals.credits,
            average_grade: totals.average_grade(),
            weighted_average_grade: totals.averaged_over_credits(totals.weighted_grade),
            gpa: totals.averaged_over_credits(totals.weighted_grade_point),
            grade_point_distribution: grade_points
                .chunk_by(|left, right| left == right)
                .map(|points| GradePointBucket {
                    grade_point: points[0],
                    count: points.len(),
                })
                .collect(),
            semesters: semesters
                .into_iter()
                .map(|(semester, totals)| SemesterSummary {
                    semester: semester.to_owned(),
                    average_grade: totals.average_grade(),
                    gpa: totals.averaged_over_credits(totals.weighted_grade_point),
                })
                .collect(),
        }
    }
}

#[cfg(feature = "uniffi")]
#[uniffi::export]
fn standard_grade_scales() -> Vec<GradeScale> {
    GradeScale::STANDARD.to_vec()
}

/// 查询百分制成绩对应的绩点。
#[cfg(feature = "uniffi")]
#[uniffi::export]
fn grade_point(grade: i64) -> Option<f64> {
    GradeScale::grade_point(grade)
}

/// 查询绩点对应的档位。
#[cfg(feature = "uniffi")]
#[uniffi::export]
fn scale_for_grade_point(point: f64) -> Option<GradeScale> {
    GradeScale::scale_for_grade_point(point)
}

/// 按教务返回的成绩、学分和绩点计算统计结果。
#[cfg(feature = "uniffi")]
#[uniffi::export]
fn grade_statistics(course_grades: &[CourseGrade]) -> GradeStatistics {
    GradeStatistics::from_course_grades(course_grades)
}

#[derive(Default)]
struct GradeTotals {
    count: usize,
    hours: f64,
    credits: f64,
    grade: f64,
    weighted_grade: f64,
    weighted_grade_point: f64,
}

impl GradeTotals {
    fn add(&mut self, course: &CourseGrade) {
        let grade = course.grade as f64;
        self.count += 1;
        self.hours += course.total_hours;
        self.credits += course.credit;
        self.grade += grade;
        self.weighted_grade += grade * course.credit;
        self.weighted_grade_point += course.grade_point * course.credit;
    }

    fn average_grade(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.grade / self.count as f64
        }
    }

    fn averaged_over_credits(&self, total: f64) -> f64 {
        if self.credits > 0.0 {
            total / self.credits
        } else {
            0.0
        }
    }
}
