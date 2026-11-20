use csustkit::{
    edu::{CourseGrade, CourseNature},
    utils::grade::{GradePointBucket, GradeStatistics, SemesterSummary},
};

fn course(semester: &str, grade: i64, credit: f64, grade_point: f64) -> CourseGrade {
    CourseGrade {
        semester: semester.to_owned(),
        course_id: format!("{semester}-{grade}"),
        course_name: "课程".to_owned(),
        group_name: String::new(),
        grade,
        grade_detail_url: "https://example.test/grades".parse().unwrap(),
        study_mode: "正常".to_owned(),
        grade_identifier: String::new(),
        credit,
        total_hours: 48.0,
        grade_point,
        retake_semester: String::new(),
        assessment_method: "考试".to_owned(),
        exam_nature: "正常考试".to_owned(),
        course_attribute: "必修".to_owned(),
        course_nature: CourseNature::ProfessionalCourse,
        course_category: "专业课".to_owned(),
    }
}

#[test]
fn empty_grades_produce_zero_statistics() {
    let statistics = GradeStatistics::from_course_grades(&[]);
    assert_eq!(statistics.total_courses, 0);
    assert_eq!(statistics.total_credits, 0.0);
    assert_eq!(statistics.total_hours, 0.0);
    assert_eq!(statistics.average_grade, 0.0);
    assert_eq!(statistics.weighted_average_grade, 0.0);
    assert_eq!(statistics.gpa, 0.0);
    assert!(statistics.grade_point_distribution.is_empty());
    assert!(statistics.semesters.is_empty());
}

#[test]
fn single_course_statistics() {
    let statistics = GradeStatistics::from_course_grades(&[course("2025-2026-1", 90, 3.0, 4.0)]);
    assert_eq!(statistics.total_courses, 1);
    assert_eq!(statistics.total_credits, 3.0);
    assert_eq!(statistics.total_hours, 48.0);
    assert_eq!(statistics.average_grade, 90.0);
    assert_eq!(statistics.weighted_average_grade, 90.0);
    assert_eq!(statistics.gpa, 4.0);
}

#[test]
fn average_and_weighted_average_differ() {
    let statistics = GradeStatistics::from_course_grades(&[
        course("2025-2026-1", 90, 4.0, 4.0),
        course("2025-2026-1", 80, 1.0, 3.0),
    ]);
    assert_eq!(statistics.total_credits, 5.0);
    assert_eq!(statistics.average_grade, 85.0);
    assert_eq!(statistics.weighted_average_grade, 88.0);
    assert_eq!(statistics.gpa, 3.8);
}

#[test]
fn grade_point_distribution_is_sorted_descending() {
    let statistics = GradeStatistics::from_course_grades(&[
        course("2025-2026-1", 80, 1.0, 3.0),
        course("2025-2026-1", 90, 1.0, 4.0),
        course("2025-2026-2", 85, 1.0, 4.0),
    ]);
    assert_eq!(
        statistics.grade_point_distribution,
        [
            GradePointBucket {
                grade_point: 4.0,
                count: 2
            },
            GradePointBucket {
                grade_point: 3.0,
                count: 1
            },
        ]
    );
}

#[test]
fn semesters_are_grouped_and_sorted() {
    let statistics = GradeStatistics::from_course_grades(&[
        course("2025-2026-2", 70, 2.0, 2.0),
        course("2025-2026-1", 90, 2.0, 4.0),
        course("2025-2026-1", 80, 2.0, 3.0),
    ]);
    assert_eq!(
        statistics.semesters,
        [
            SemesterSummary {
                semester: "2025-2026-1".to_owned(),
                average_grade: 85.0,
                gpa: 3.5
            },
            SemesterSummary {
                semester: "2025-2026-2".to_owned(),
                average_grade: 70.0,
                gpa: 2.0
            },
        ]
    );
}

#[test]
fn zero_credits_keep_unweighted_grades() {
    let statistics = GradeStatistics::from_course_grades(&[course("2025-2026-1", 90, 0.0, 4.0)]);
    assert_eq!(statistics.average_grade, 90.0);
    assert_eq!(statistics.weighted_average_grade, 0.0);
    assert_eq!(statistics.gpa, 0.0);
    assert_eq!(statistics.semesters[0].gpa, 0.0);
}
