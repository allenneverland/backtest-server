use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 資料頻率類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Frequency {
    Tick,
    Second(u32),   // 秒線, e.g. Second(1) for 1-second bar
    Minute(u32),   // 分線, e.g. Minute(1) for 1-minute bar
    Hour(u32),     // 小時線
    Day,           // 日線
    Week,          // 週線
    Month,         // 月線
    Quarter,       // 季線
    Year,          // 年線
}

impl Frequency {
    /// 將頻率轉換為 Duration
    /// 注意：對於 Tick, Day 及以上的頻率，會返回預設值
    pub fn to_duration(&self) -> Duration {
        match self {
            Frequency::Tick => Duration::from_millis(1), // Tick 沒有固定的 Duration，返回一個較小的值
            Frequency::Second(s) => Duration::from_secs(*s as u64),
            Frequency::Minute(m) => Duration::from_secs(*m as u64 * 60),
            Frequency::Hour(h) => Duration::from_secs(*h as u64 * 3600),
            Frequency::Day => Duration::from_secs(86400), // 24小時
            Frequency::Week => Duration::from_secs(86400 * 7), // 7天
            Frequency::Month => Duration::from_secs(86400 * 30), // 約30天
            Frequency::Quarter => Duration::from_secs(86400 * 91), // 約91天
            Frequency::Year => Duration::from_secs(86400 * 365), // 約365天
        }
    }
}

/// 聚合操作類型 (用於將較高頻率數據轉換為較低頻率)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationOp {
    First,         // 第一個值
    Last,          // 最後一個值
    Max,           // 最大值
    Min,           // 最小值
    Sum,           // 求和
    Mean,          // 平均值
    Median,        // 中位數
    Count,         // 計數
    Variance,      // 方差
    StandardDev,   // 標準差
} 