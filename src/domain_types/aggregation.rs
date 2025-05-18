use serde::{Deserialize, Serialize};

use crate::domain_types::frequency::{Frequency, AggregationOp};

/// 資料聚合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    pub target_frequency: Frequency,            // 目標頻率
    pub open_op: AggregationOp,                 // 開盤價聚合操作
    pub high_op: AggregationOp,                 // 最高價聚合操作
    pub low_op: AggregationOp,                  // 最低價聚合操作
    pub close_op: AggregationOp,                // 收盤價聚合操作
    pub volume_op: AggregationOp,               // 成交量聚合操作
    pub include_partial_bars: bool,             // 是否包含不完整的bar
    pub fill_value: Option<f64>,                // 缺失值填充值，默認為 None
} 

impl AggregationConfig {
    /// 創建一個新的聚合配置，使用標準的 OHLCV 聚合操作
    pub fn new(target_frequency: Frequency) -> Self {
        Self {
            target_frequency,
            open_op: AggregationOp::First,      // 開盤價取第一個值
            high_op: AggregationOp::Max,        // 最高價取最大值
            low_op: AggregationOp::Min,         // 最低價取最小值
            close_op: AggregationOp::Last,      // 收盤價取最後一個值
            volume_op: AggregationOp::Sum,      // 成交量累加
            include_partial_bars: false,        // 默認不包含不完整的bar
            fill_value: None,                  // 默認不填充缺失值
        }
    }
    
    /// 創建一個包含部分柱的聚合配置
    pub fn with_partial_bars(target_frequency: Frequency) -> Self {
        let mut config = Self::new(target_frequency);
        config.include_partial_bars = true;
        config
    }
    
    /// 創建一個使用自定義填充值的聚合配置
    pub fn with_fill_value(target_frequency: Frequency, fill_value: f64) -> Self {
        let mut config = Self::new(target_frequency);
        config.fill_value = Some(fill_value);
        config
    }
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            target_frequency: Frequency::Day,   // 默認為日線
            open_op: AggregationOp::First,
            high_op: AggregationOp::Max,
            low_op: AggregationOp::Min,
            close_op: AggregationOp::Last,
            volume_op: AggregationOp::Sum,
            include_partial_bars: false,
            fill_value: None,
        }
    }
}