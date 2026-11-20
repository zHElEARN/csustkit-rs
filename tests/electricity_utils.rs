use chrono::{DateTime, TimeDelta, Utc};
use csustkit::utils::electricity::{
    ElectricityRecord, ValueRangeOptions, downsample, predict_exhaustion_date, value_range,
};

const BASE: i64 = 1_700_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

fn readings(values: &[f64]) -> Vec<ElectricityRecord> {
    values
        .iter()
        .enumerate()
        .map(|(index, &electricity)| ElectricityRecord {
            date: DateTime::<Utc>::from_timestamp(BASE + index as i64 * SECONDS_PER_DAY, 0)
                .unwrap(),
            electricity,
        })
        .collect()
}

fn predicted_days(records: &[ElectricityRecord]) -> f64 {
    (predict_exhaustion_date(records).unwrap() - records.last().unwrap().date).as_seconds_f64()
        / SECONDS_PER_DAY as f64
}

#[test]
fn predict_returns_none_without_enough_readings() {
    assert_eq!(predict_exhaustion_date(&[]), None);
    assert_eq!(predict_exhaustion_date(&readings(&[10.0])), None);
}

#[test]
fn predict_returns_none_when_not_declining() {
    assert_eq!(predict_exhaustion_date(&readings(&[10.0, 10.0])), None);
    assert_eq!(predict_exhaustion_date(&readings(&[10.0, 12.0])), None);
}

#[test]
fn predict_returns_none_when_span_is_too_short() {
    let mut records = readings(&[10.0, 9.9]);
    records[1].date = records[0].date + TimeDelta::hours(1);
    assert_eq!(predict_exhaustion_date(&records), None);
}

#[test]
fn predict_linear_decline() {
    let values = (0..=14)
        .map(|day| 24.0 - f64::from(day))
        .collect::<Vec<_>>();
    assert!((predicted_days(&readings(&values)) - 10.0).abs() < 0.01);
}

#[test]
fn predict_only_uses_recent_window() {
    let values = (0..=13)
        .map(|day| 100.0 - f64::from(day) * 5.0)
        .chain((0..=13).map(|day| 35.0 - f64::from(day)))
        .collect::<Vec<_>>();
    assert!((predicted_days(&readings(&values)) - 22.0 / (13.0 / 14.0)).abs() < 0.05);
}

#[test]
fn predict_starts_after_last_charge() {
    let records = readings(&[20.0, 19.0, 18.0, 30.0, 29.0, 28.0]);
    assert!((predicted_days(&records) - 28.0).abs() < 0.05);
}

#[test]
fn predict_returns_none_when_segment_has_single_reading() {
    assert_eq!(predict_exhaustion_date(&readings(&[10.0, 9.0, 20.0])), None);
}

#[test]
fn downsample_keeps_input_when_not_enough() {
    let input = readings(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    for threshold in [5, 10, 0, -3] {
        assert_eq!(downsample(&input, threshold), input);
    }
    assert!(downsample(&[], 10).is_empty());
}

#[test]
fn downsample_keeps_endpoints_at_small_thresholds() {
    let input = readings(&(0..10).map(f64::from).collect::<Vec<_>>());
    for threshold in [1, 2] {
        assert_eq!(
            downsample(&input, threshold),
            [input[0], input[input.len() - 1]]
        );
    }
}

#[test]
fn downsample_hits_threshold_and_keeps_endpoints() {
    let input = readings(
        &(0..1000)
            .map(|day| 1000.0 - f64::from(day))
            .collect::<Vec<_>>(),
    );
    let output = downsample(&input, 80);
    assert_eq!(output.len(), 80);
    assert_eq!(output.first(), input.first());
    assert_eq!(output.last(), input.last());
    assert!(
        output
            .windows(2)
            .all(|pair| pair[0].electricity > pair[1].electricity)
    );
}

#[test]
fn downsample_retains_a_peak_between_the_endpoints() {
    let input = readings(&[5.0, 5.0, 95.0, 5.0, 5.0]);
    assert_eq!(downsample(&input, 3), [input[0], input[2], input[4]]);
}

#[test]
fn value_range_falls_back_for_empty_input() {
    assert_eq!(value_range(&[], ValueRangeOptions::default()), 0.0..=2.0);
    let options = ValueRangeOptions {
        fallback: 0.0..=5.0,
        ..ValueRangeOptions::default()
    };
    assert_eq!(value_range(&[], options), 0.0..=5.0);
}

#[test]
fn value_range_applies_padding() {
    assert_eq!(
        value_range(&readings(&[10.0, 20.0]), ValueRangeOptions::default()),
        8.0..=22.0
    );
}

#[test]
fn value_range_never_goes_below_zero() {
    assert_eq!(
        value_range(&readings(&[0.5, 1.0]), ValueRangeOptions::default()),
        0.0..=3.0
    );
}

#[test]
fn value_range_honours_minimum_span() {
    let options = ValueRangeOptions {
        padding: 0.0,
        minimum_span: 1.0,
        ..ValueRangeOptions::default()
    };
    assert_eq!(value_range(&readings(&[10.0]), options), 10.0..=11.0);
}
