use csustkit::utils::grade::GradeScale;

#[test]
fn standard_covers_eleven_ranges_in_descending_order() {
    let scales = GradeScale::STANDARD;
    assert_eq!(scales.len(), 11);
    assert_eq!(scales.first().unwrap().min_grade, 90);
    assert_eq!(scales.last().unwrap().grade_point, 0.0);
    for pair in scales.windows(2) {
        assert!(pair[0].min_grade > pair[1].min_grade);
    }
}

#[test]
fn display_range_formats() {
    assert_eq!(GradeScale::new(90, 100, 4.0).display_range(), "90-100");
    assert_eq!(GradeScale::new(0, 59, 0.0).display_range(), "≤59");
}

#[test]
fn grade_point_lookup_handles_boundaries() {
    for (grade, grade_point) in [
        (100, 4.0),
        (90, 4.0),
        (89, 3.7),
        (85, 3.7),
        (60, 1.0),
        (59, 0.0),
        (0, 0.0),
    ] {
        assert_eq!(GradeScale::grade_point(grade), Some(grade_point));
    }
}

#[test]
fn grade_point_lookup_rejects_out_of_range_grades() {
    assert_eq!(GradeScale::grade_point(101), None);
    assert_eq!(GradeScale::grade_point(-1), None);
}

#[test]
fn every_range_resolves_to_its_own_point() {
    for scale in GradeScale::STANDARD {
        assert_eq!(
            GradeScale::grade_point(scale.min_grade),
            Some(scale.grade_point)
        );
        assert_eq!(
            GradeScale::grade_point(scale.max_grade),
            Some(scale.grade_point)
        );
    }
}

#[test]
fn scale_lookup_by_grade_point() {
    for (grade_point, label) in [
        (3.7, "85-89"),
        (4.0, "90-100"),
        (0.0, "≤59"),
        (3.70005, "85-89"),
    ] {
        assert_eq!(
            GradeScale::scale_for_grade_point(grade_point)
                .unwrap()
                .display_range(),
            label
        );
    }
    assert_eq!(GradeScale::scale_for_grade_point(5.0), None);
    assert_eq!(GradeScale::scale_for_grade_point(2.5), None);
}
