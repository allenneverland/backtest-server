// time_utils.rs
use chrono::{DateTime, TimeZone, Utc};

/// 將 DateTime<Utc> 轉換為毫秒時間戳
pub fn datetime_to_timestamp_ms(dt: &DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

/// 將毫秒時間戳轉換為 DateTime<Utc>
pub fn timestamp_ms_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(ts).single().unwrap_or_else(|| Utc::now())
}

/// 將 i64 時間戳數組轉換為 DateTime<Utc> 數組
pub fn timestamps_to_datetimes(timestamps: &[i64]) -> Vec<DateTime<Utc>> {
    timestamps.iter().map(|&ts| timestamp_ms_to_datetime(ts)).collect()
}

/// 將 DateTime<Utc> 數組轉換為 i64 時間戳數組
pub fn datetimes_to_timestamps(datetimes: &[DateTime<Utc>]) -> Vec<i64> {
    datetimes.iter().map(|dt| datetime_to_timestamp_ms(dt)).collect()
}