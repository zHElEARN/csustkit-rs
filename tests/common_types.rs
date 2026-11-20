use csustkit::{Campus, DayOfWeek, campus_card, edu};

#[test]
fn existing_module_paths_export_the_shared_types() {
    let campus: Campus = campus_card::Campus::Yuntang;
    let same_campus: edu::Campus = campus;
    let day: DayOfWeek = edu::DayOfWeek::Monday;

    assert_eq!(same_campus, Campus::Yuntang);
    assert_eq!(day, DayOfWeek::Monday);
}

#[test]
fn shared_enums_preserve_their_serialized_values() {
    for (campus, json) in Campus::ALL
        .into_iter()
        .zip([r#""yuntang""#, r#""jinpenling""#])
    {
        assert_eq!(serde_json::to_string(&campus).unwrap(), json);
        assert_eq!(serde_json::from_str::<Campus>(json).unwrap(), campus);
    }

    let days = [
        (DayOfWeek::Sunday, r#""sunday""#),
        (DayOfWeek::Monday, r#""monday""#),
        (DayOfWeek::Tuesday, r#""tuesday""#),
        (DayOfWeek::Wednesday, r#""wednesday""#),
        (DayOfWeek::Thursday, r#""thursday""#),
        (DayOfWeek::Friday, r#""friday""#),
        (DayOfWeek::Saturday, r#""saturday""#),
    ];
    for (index, (day, json)) in days.into_iter().enumerate() {
        assert_eq!(usize::from(day as u8), index);
        assert_eq!(serde_json::to_string(&day).unwrap(), json);
        assert_eq!(serde_json::from_str::<DayOfWeek>(json).unwrap(), day);
    }
}
