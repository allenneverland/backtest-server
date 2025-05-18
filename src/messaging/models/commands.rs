use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

/// 基礎命令特徵
pub trait Command {
    /// 獲取命令名稱
    fn command_name() -> &'static str;
}

//
// 回測相關命令
//

/// 創建回測任務命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateBacktestCommand {
    /// 回測任務ID
    pub backtest_id: String,
    /// 策略ID
    pub strategy_id: String,
    /// 策略版本
    pub strategy_version: Option<String>,
    /// 回測開始時間
    pub start_date: DateTime<Utc>,
    /// 回測結束時間
    pub end_date: DateTime<Utc>,
    /// 回測資金
    pub initial_capital: f64,
    /// 回測資產列表
    pub assets: Vec<String>,
    /// 回測參數
    pub parameters: HashMap<String, serde_json::Value>,
}

impl Command for CreateBacktestCommand {
    fn command_name() -> &'static str {
        "create_backtest"
    }
}

/// 取消回測任務命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CancelBacktestCommand {
    /// 回測任務ID
    pub backtest_id: String,
    /// 取消原因
    pub reason: Option<String>,
}

impl Command for CancelBacktestCommand {
    fn command_name() -> &'static str {
        "cancel_backtest"
    }
}

/// 獲取回測結果命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetBacktestResultCommand {
    /// 回測任務ID
    pub backtest_id: String,
    /// 是否包含詳細交易記錄
    pub include_trades: bool,
    /// 是否包含詳細倉位記錄
    pub include_positions: bool,
}

impl Command for GetBacktestResultCommand {
    fn command_name() -> &'static str {
        "get_backtest_result"
    }
}

//
// 策略相關命令
//

/// 創建策略命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateStrategyCommand {
    /// 策略ID（如果為空則自動生成）
    pub strategy_id: Option<String>,
    /// 策略名稱
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 策略代碼
    pub code: String,
    /// 策略參數
    pub parameters: HashMap<String, serde_json::Value>,
    /// 策略標籤
    pub tags: Vec<String>,
}

impl Command for CreateStrategyCommand {
    fn command_name() -> &'static str {
        "create_strategy"
    }
}

/// 更新策略命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateStrategyCommand {
    /// 策略ID
    pub strategy_id: String,
    /// 新策略名稱（可選）
    pub name: Option<String>,
    /// 新策略描述（可選）
    pub description: Option<String>,
    /// 新策略代碼（可選）
    pub code: Option<String>,
    /// 新策略參數（可選）
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    /// 新策略標籤（可選）
    pub tags: Option<Vec<String>>,
}

impl Command for UpdateStrategyCommand {
    fn command_name() -> &'static str {
        "update_strategy"
    }
}

/// 刪除策略命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteStrategyCommand {
    /// 策略ID
    pub strategy_id: String,
}

impl Command for DeleteStrategyCommand {
    fn command_name() -> &'static str {
        "delete_strategy"
    }
}

/// 獲取策略命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetStrategyCommand {
    /// 策略ID
    pub strategy_id: String,
    /// 版本（可選，如果為空則獲取最新版本）
    pub version: Option<String>,
    /// 是否包含代碼
    pub include_code: bool,
}

impl Command for GetStrategyCommand {
    fn command_name() -> &'static str {
        "get_strategy"
    }
}

//
// 數據相關命令
//

/// 獲取市場數據命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetMarketDataCommand {
    /// 資產代碼
    pub asset_code: String,
    /// 數據類型（如 OHLCV, TICK 等）
    pub data_type: String,
    /// 頻率（如 1m, 1h, 1d 等）
    pub frequency: String,
    /// 開始時間
    pub start_date: DateTime<Utc>,
    /// 結束時間
    pub end_date: DateTime<Utc>,
    /// 是否調整（如股票復權）
    pub adjusted: bool,
    /// 最大數據點數量（分頁用）
    pub limit: Option<usize>,
    /// 分頁標記（分頁用）
    pub page_token: Option<String>,
}

impl Command for GetMarketDataCommand {
    fn command_name() -> &'static str {
        "get_market_data"
    }
}

/// 導入市場數據命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportMarketDataCommand {
    /// 數據來源
    pub source: String,
    /// 資產代碼
    pub asset_code: String,
    /// 數據類型（如 OHLCV, TICK 等）
    pub data_type: String,
    /// 頻率（如 1m, 1h, 1d 等）
    pub frequency: String,
    /// 文件路徑或URL
    pub path: String,
    /// 覆蓋現有數據
    pub overwrite: bool,
    /// 檔案格式（如 CSV, JSON 等）
    pub format: String,
    /// 格式選項
    pub format_options: Option<HashMap<String, String>>,
}

impl Command for ImportMarketDataCommand {
    fn command_name() -> &'static str {
        "import_market_data"
    }
}

/// 獲取資產列表命令
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetAssetsCommand {
    /// 資產類型篩選
    pub asset_types: Option<Vec<String>>,
    /// 交易所篩選
    pub exchanges: Option<Vec<String>>,
    /// 搜索關鍵字
    pub search: Option<String>,
    /// 最大返回數量
    pub limit: Option<usize>,
    /// 分頁標記
    pub page_token: Option<String>,
}

impl Command for GetAssetsCommand {
    fn command_name() -> &'static str {
        "get_assets"
    }
} 