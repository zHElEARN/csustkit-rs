//! 宿舍电量预测与趋势图数据处理。

use std::{ops::RangeInclusive, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

/// 一次宿舍剩余电量读数。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ElectricityRecord {
    pub date: DateTime<Utc>,
    pub electricity: f64,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ValueRangeOptions {
    pub padding: f64,
    pub minimum_span: f64,
    pub fallback: RangeInclusive<f64>,
}

impl Default for ValueRangeOptions {
    fn default() -> Self {
        Self {
            padding: 2.0,
            minimum_span: 1.0,
            fallback: 0.0..=2.0,
        }
    }
}

#[cfg(feature = "uniffi")]
#[uniffi::export]
fn default_electricity_value_range_options() -> ValueRangeOptions {
    ValueRangeOptions::default()
}

/// 使用最后一次充值后、最近 14 天内的首尾读数预测耗尽时间。
///
/// 读数按时间升序排列。跨度不足一天、当前电量不为正数或电量未下降时返回 `None`。
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn predict_exhaustion_date(records: &[ElectricityRecord]) -> Option<DateTime<Utc>> {
    let last = records.last()?;
    if !last.electricity.is_finite() || last.electricity <= 0.0 {
        return None;
    }

    let mut segment_start = records.len() - 1;
    while segment_start > 0
        && records[segment_start].electricity <= records[segment_start - 1].electricity
    {
        segment_start -= 1;
    }

    let window_start = last
        .date
        .checked_sub_signed(TimeDelta::days(14))
        .unwrap_or(DateTime::<Utc>::MIN_UTC);
    let first = records[segment_start..]
        .iter()
        .find(|record| record.date >= window_start)?;
    let span = last.date - first.date;
    let consumed = first.electricity - last.electricity;
    if span < TimeDelta::days(1) || !consumed.is_finite() || consumed <= 0.0 {
        return None;
    }

    let remaining = last.electricity / consumed * span.as_seconds_f64();
    let remaining = Duration::try_from_secs_f64(remaining).ok()?;
    last.date
        .checked_add_signed(TimeDelta::from_std(remaining).ok()?)
}

/// 使用最大三角面积降采样，保留首尾读数。
///
/// 读数按时间升序排列。目标数量非正数或大于等于读数数量时原样返回。
/// 目标数量为 1 或 2 时，仍保留首尾两个端点。
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn downsample(records: &[ElectricityRecord], threshold: i64) -> Vec<ElectricityRecord> {
    let Ok(threshold) = usize::try_from(threshold) else {
        return records.to_vec();
    };
    if threshold == 0 || records.len() <= threshold {
        return records.to_vec();
    }
    if threshold <= 2 {
        return vec![records[0], records[records.len() - 1]];
    }

    let buckets = threshold - 2;
    let interior_count = records.len() - 2;
    let bucket_start =
        |index: usize| (index as u128 * interior_count as u128 / buckets as u128) as usize + 1;
    let mut sampled = Vec::with_capacity(threshold);
    let mut selected = records[0];
    sampled.push(selected);

    for bucket in 0..buckets {
        let end = bucket_start(bucket + 1);
        let next_end = if bucket + 1 == buckets {
            records.len()
        } else {
            bucket_start(bucket + 2)
        };
        let next = &records[end..next_end];
        let (next_x, next_y) = next.iter().fold((0.0, 0.0), |(x, y), record| {
            (
                x + (record.date - selected.date).as_seconds_f64(),
                y + record.electricity,
            )
        });
        let next_x = next_x / next.len() as f64;
        let next_y = next_y / next.len() as f64;

        let candidates = &records[bucket_start(bucket)..end];
        let mut max_area = -1.0;
        let mut point = candidates[0];
        for candidate in candidates {
            let x = (candidate.date - selected.date).as_seconds_f64();
            let area = (x * (next_y - selected.electricity)
                - next_x * (candidate.electricity - selected.electricity))
                .abs();
            if area > max_area {
                max_area = area;
                point = *candidate;
            }
        }
        selected = point;
        sampled.push(selected);
    }

    sampled.push(records[records.len() - 1]);
    sampled
}

/// 根据读数范围计算纵轴值域。下界不小于 0，空输入使用备用值域。
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn value_range(
    records: &[ElectricityRecord],
    options: ValueRangeOptions,
) -> RangeInclusive<f64> {
    let Some((first, tail)) = records.split_first() else {
        return options.fallback;
    };
    let (minimum, maximum) = tail.iter().fold(
        (first.electricity, first.electricity),
        |(min, max), record| (min.min(record.electricity), max.max(record.electricity)),
    );
    let lower = (minimum - options.padding).max(0.0);
    let upper = (lower + options.minimum_span.max(0.0)).max(maximum + options.padding);
    lower..=upper
}
