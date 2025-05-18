use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

/// 基礎事件特徵
pub trait Event {
    /// 獲取事件名稱
    fn event_name() -> &'static str;
}

//
// 回測相關事件
//

/// 回測任務狀態
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BacktestStatus {
    /// 已創建，尚未開始
    Created,
    /// 正在初始化
    Initializing,
    /// 正在運行
    Running,
    /// 已暫停
    Paused,
    /// 已取消
    Cancelled,
    /// 已完成
    Completed,
    /// 發生錯誤
    Error,
}

/// 回測任務創建事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestCreatedEvent {
    /// 回測任務ID
    pub backtest_id: String,
    /// 策略ID
    pub strategy_id: String,
    /// 策略版本
    pub strategy_version: Option<String>,
    /// 創建時間
    pub created_at: DateTime<Utc>,
    /// 狀態
    pub status: BacktestStatus,
    /// 回測參數
    pub parameters: HashMap<String, serde_json::Value>,
}

impl Event for BacktestCreatedEvent {
    fn event_name() -> &'static str {
        "backtest_created"
    }
}

/// 回測任務狀態變更事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestStatusChangedEvent {
    /// 回測任務ID
    pub backtest_id: String,
    /// 舊狀態
    pub old_status: BacktestStatus,
    /// 新狀態
    pub new_status: BacktestStatus,
    /// 變更時間
    pub changed_at: DateTime<Utc>,
    /// 附加信息
    pub message: Option<String>,
}

impl Event for BacktestStatusChangedEvent {
    fn event_name() -> &'static str {
        "backtest_status_changed"
    }
}

/// 回測任務進度更新事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestProgressEvent {
    /// 回測任務ID
    pub backtest_id: String,
    /// 進度百分比 (0-100)
    pub progress: f64,
    /// 當前處理的時間點
    pub current_date: DateTime<Utc>,
    /// 剩餘預估時間（秒）
    pub estimated_time_remaining: Option<f64>,
    /// 已處理的資料點數量
    pub processed_data_points: u64,
    /// 已完成的交易數量
    pub completed_trades: u64,
    /// 更新時間
    pub updated_at: DateTime<Utc>,
}

impl Event for BacktestProgressEvent {
    fn event_name() -> &'static str {
        "backtest_progress"
    }
}

/// 回測任務完成事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestCompletedEvent {
    /// 回測任務ID
    pub backtest_id: String,
    /// 完成時間
    pub completed_at: DateTime<Utc>,
    /// 運行時間（秒）
    pub execution_time: f64,
    /// 總收益率
    pub total_return: f64,
    /// 年化收益率
    pub annualized_return: f64,
    /// 最大回撤
    pub max_drawdown: f64,
    /// 夏普比率
    pub sharpe_ratio: f64,
    /// 交易數量
    pub trade_count: u64,
    /// 結果摘要
    pub summary: HashMap<String, serde_json::Value>,
}

impl Event for BacktestCompletedEvent {
    fn event_name() -> &'static str {
        "backtest_completed"
    }
}

/// 回測錯誤事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestErrorEvent {
    /// 回測任務ID
    pub backtest_id: String,
    /// 錯誤時間
    pub error_time: DateTime<Utc>,
    /// 錯誤代碼
    pub error_code: String,
    /// 錯誤訊息
    pub error_message: String,
    /// 錯誤詳情
    pub error_details: Option<serde_json::Value>,
}

impl Event for BacktestErrorEvent {
    fn event_name() -> &'static str {
        "backtest_error"
    }
}

//
// 策略相關事件
//

/// 策略創建事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyCreatedEvent {
    /// 策略ID
    pub strategy_id: String,
    /// 策略名稱
    pub name: String,
    /// 策略版本
    pub version: String,
    /// 創建時間
    pub created_at: DateTime<Utc>,
    /// 創建者
    pub created_by: Option<String>,
}

impl Event for StrategyCreatedEvent {
    fn event_name() -> &'static str {
        "strategy_created"
    }
}

/// 策略更新事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyUpdatedEvent {
    /// 策略ID
    pub strategy_id: String,
    /// 策略名稱
    pub name: String,
    /// 策略版本
    pub version: String,
    /// 更新時間
    pub updated_at: DateTime<Utc>,
    /// 更新者
    pub updated_by: Option<String>,
    /// 版本變更描述
    pub change_description: Option<String>,
}

impl Event for StrategyUpdatedEvent {
    fn event_name() -> &'static str {
        "strategy_updated"
    }
}

/// 策略刪除事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyDeletedEvent {
    /// 策略ID
    pub strategy_id: String,
    /// 刪除時間
    pub deleted_at: DateTime<Utc>,
    /// 刪除者
    pub deleted_by: Option<String>,
}

impl Event for StrategyDeletedEvent {
    fn event_name() -> &'static str {
        "strategy_deleted"
    }
}

//
// 數據相關事件
//

/// 數據導入狀態
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataImportStatus {
    /// 已開始導入
    Started,
    /// 正在進行
    InProgress,
    /// 已完成
    Completed,
    /// 失敗
    Failed,
}

/// 數據導入事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataImportEvent {
    /// 導入任務ID
    pub import_id: String,
    /// 資產代碼
    pub asset_code: String,
    /// 數據類型
    pub data_type: String,
    /// 頻率
    pub frequency: String,
    /// 狀態
    pub status: DataImportStatus,
    /// 進度 (0-100)
    pub progress: Option<f64>,
    /// 導入的數據點數量
    pub data_points: Option<u64>,
    /// 開始時間
    pub start_date: Option<DateTime<Utc>>,
    /// 結束時間
    pub end_date: Option<DateTime<Utc>>,
    /// 消息
    pub message: Option<String>,
    /// 更新時間
    pub updated_at: DateTime<Utc>,
}

impl Event for DataImportEvent {
    fn event_name() -> &'static str {
        "data_import"
    }
}

/// 數據更新事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataUpdatedEvent {
    /// 資產代碼
    pub asset_code: String,
    /// 數據類型
    pub data_type: String,
    /// 頻率
    pub frequency: String,
    /// 開始時間
    pub start_date: DateTime<Utc>,
    /// 結束時間
    pub end_date: DateTime<Utc>,
    /// 更新的數據點數量
    pub data_points: u64,
    /// 更新時間
    pub updated_at: DateTime<Utc>,
}

impl Event for DataUpdatedEvent {
    fn event_name() -> &'static str {
        "data_updated"
    }
} 