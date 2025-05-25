// time_utils.rs
//
// 提供時間轉換相關的工具函數，用於在系統不同層之間轉換時間格式。
// 主要功能：
// 1. 在資料庫層和領域模型層之間轉換時間格式
// 2. 在領域模型層和計算核心層之間轉換時間格式
// 3. 提供各種時間格式的互相轉換

use chrono::{DateTime, TimeZone, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

//
// 基礎時間轉換函數
//

/// 將 DateTime<Utc> 轉換為毫秒時間戳
pub fn datetime_to_timestamp_ms(dt: &DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

/// 將毫秒時間戳轉換為 DateTime<Utc>
pub fn timestamp_ms_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(ts)
        .single()
        .unwrap_or_else(|| Utc::now())
}

/// 將 i64 時間戳數組轉換為 DateTime<Utc> 數組
pub fn timestamps_to_datetimes(timestamps: &[i64]) -> Vec<DateTime<Utc>> {
    timestamps
        .iter()
        .map(|&ts| timestamp_ms_to_datetime(ts))
        .collect()
}

/// 將 DateTime<Utc> 數組轉換為 i64 時間戳數組
pub fn datetimes_to_timestamps(datetimes: &[DateTime<Utc>]) -> Vec<i64> {
    datetimes
        .iter()
        .map(|dt| datetime_to_timestamp_ms(dt))
        .collect()
}

/// 獲取當前系統時間的毫秒時間戳
pub fn current_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

//
// 資料庫層 <-> 領域模型層轉換函數
//

/// 將可選的 DateTime<Utc> 轉換為可選的毫秒時間戳
/// 用於從領域模型轉換到資料庫參數
pub fn opt_datetime_to_opt_timestamp_ms(dt: &Option<DateTime<Utc>>) -> Option<i64> {
    dt.as_ref().map(datetime_to_timestamp_ms)
}

/// 將可選的毫秒時間戳轉換為可選的 DateTime<Utc>
/// 用於從資料庫結果轉換到領域模型
pub fn opt_timestamp_ms_to_opt_datetime(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.map(timestamp_ms_to_datetime)
}

//
// 領域模型層 <-> 計算核心層轉換函數
//

/// 將時間範圍從 DateTime<Utc> 轉換為 i64 毫秒時間戳
/// 用於從領域模型傳遞到計算核心
pub fn datetime_range_to_timestamp_range(start: &DateTime<Utc>, end: &DateTime<Utc>) -> (i64, i64) {
    (
        datetime_to_timestamp_ms(start),
        datetime_to_timestamp_ms(end),
    )
}

/// 將時間範圍從 i64 毫秒時間戳轉換為 DateTime<Utc>
/// 用於從計算核心返回到領域模型
pub fn timestamp_range_to_datetime_range(
    start_ts: i64,
    end_ts: i64,
) -> (DateTime<Utc>, DateTime<Utc>) {
    (
        timestamp_ms_to_datetime(start_ts),
        timestamp_ms_to_datetime(end_ts),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_datetime_timestamp_conversion() {
        let now = Utc::now();
        let ts = datetime_to_timestamp_ms(&now);
        let dt = timestamp_ms_to_datetime(ts);

        // 由於浮點數精度損失，轉換後可能有幾毫秒的差異
        let diff = (now - dt).num_milliseconds().abs();
        assert!(diff < 2, "時間轉換差異應小於2毫秒，實際差異: {}", diff);
    }

    #[test]
    fn test_optional_datetime_conversion() {
        let now = Utc::now();
        let opt_now = Some(now);
        let opt_none: Option<DateTime<Utc>> = None;

        // 測試 Some 值的轉換
        let ts = opt_datetime_to_opt_timestamp_ms(&opt_now);
        let dt = opt_timestamp_ms_to_opt_datetime(ts);
        assert!(dt.is_some());

        // 測試 None 值的轉換
        let ts_none = opt_datetime_to_opt_timestamp_ms(&opt_none);
        let dt_none = opt_timestamp_ms_to_opt_datetime(ts_none);
        assert!(dt_none.is_none());
    }

    #[test]
    fn test_datetime_range_conversion() {
        let now = Utc::now();
        let tomorrow = now + Duration::days(1);

        let (start_ts, end_ts) = datetime_range_to_timestamp_range(&now, &tomorrow);
        let (start_dt, end_dt) = timestamp_range_to_datetime_range(start_ts, end_ts);

        // 檢查時間差異
        let start_diff = (now - start_dt).num_milliseconds().abs();
        let end_diff = (tomorrow - end_dt).num_milliseconds().abs();

        assert!(
            start_diff < 2,
            "開始時間差異應小於2毫秒，實際差異: {}",
            start_diff
        );
        assert!(
            end_diff < 2,
            "結束時間差異應小於2毫秒，實際差異: {}",
            end_diff
        );
    }
}
