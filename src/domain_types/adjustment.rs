use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


/// 調整後數據點類型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdjustmentType {
    Split,         // 拆股
    Dividend,      // 分紅
    Rights,        // 配股
}

/// 調整數據結構
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adjustment {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub adjustment_type: AdjustmentType,
    pub ratio: f64,                    // 調整比例
    pub value: f64,                    // 調整值 (如每股分紅)
}

/// 復權類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjustmentMode {
    None,          // 不進行復權
    Forward,       // 前復權 (全部調整為最新狀態)
    Backward,      // 後復權 (全部調整為初始狀態)
} 