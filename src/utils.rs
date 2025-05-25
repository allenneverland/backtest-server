// utils.rs - 公共工具模組
//
// 提供各種通用的工具函數和輔助方法，用於簡化系統其他部分的代碼。

pub mod time_utils;

// 重新導出時間工具函數，使其可以通過 utils::function_name 直接訪問
pub use time_utils::{
    current_timestamp_ms,

    // 領域模型層 <-> 計算核心層
    datetime_range_to_timestamp_range,
    // 基礎時間轉換
    datetime_to_timestamp_ms,
    datetimes_to_timestamps,
    // 資料庫層 <-> 領域模型層
    opt_datetime_to_opt_timestamp_ms,
    opt_timestamp_ms_to_opt_datetime,

    timestamp_ms_to_datetime,
    timestamp_range_to_datetime_range,
    timestamps_to_datetimes,
};
