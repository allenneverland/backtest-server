use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// 標準錯誤代碼
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    /// 無效請求
    BadRequest,
    /// 未授權
    Unauthorized,
    /// 禁止訪問
    Forbidden,
    /// 資源不存在
    NotFound,
    /// 衝突
    Conflict,
    /// 內部伺服器錯誤
    InternalServerError,
    /// 服務不可用
    ServiceUnavailable,
    /// 超時
    Timeout,
    /// 驗證錯誤
    ValidationError,
    /// 功能未實現
    NotImplemented,
}

impl ToString for ErrorCode {
    fn to_string(&self) -> String {
        match self {
            ErrorCode::BadRequest => "BAD_REQUEST".to_string(),
            ErrorCode::Unauthorized => "UNAUTHORIZED".to_string(),
            ErrorCode::Forbidden => "FORBIDDEN".to_string(),
            ErrorCode::NotFound => "NOT_FOUND".to_string(),
            ErrorCode::Conflict => "CONFLICT".to_string(),
            ErrorCode::InternalServerError => "INTERNAL_SERVER_ERROR".to_string(),
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE".to_string(),
            ErrorCode::Timeout => "TIMEOUT".to_string(),
            ErrorCode::ValidationError => "VALIDATION_ERROR".to_string(),
            ErrorCode::NotImplemented => "NOT_IMPLEMENTED".to_string(),
        }
    }
}

/// 標準錯誤回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 錯誤代碼
    pub code: String,
    /// 錯誤訊息
    pub message: String,
    /// 詳細錯誤資訊
    pub details: Option<serde_json::Value>,
    /// 錯誤時間
    pub timestamp: DateTime<Utc>,
    /// 請求ID
    pub request_id: Option<String>,
}

impl ErrorResponse {
    /// 創建新的錯誤回應
    pub fn new(code: ErrorCode, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            timestamp: Utc::now(),
            request_id: None,
        }
    }
    
    /// 設置詳細錯誤資訊
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    
    /// 設置請求ID
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }
}

/// 標準成功回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    /// 是否成功
    pub success: bool,
    /// 回應數據
    pub data: T,
    /// 回應時間
    pub timestamp: DateTime<Utc>,
    /// 請求ID
    pub request_id: Option<String>,
}

impl<T> SuccessResponse<T> {
    /// 創建新的成功回應
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
            timestamp: Utc::now(),
            request_id: None,
        }
    }
    
    /// 設置請求ID
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }
}

//
// 回測相關回應
//

/// 回測任務結果回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestResultResponse {
    /// 回測任務ID
    pub backtest_id: String,
    /// 策略ID
    pub strategy_id: String,
    /// 策略版本
    pub strategy_version: Option<String>,
    /// 開始時間
    pub start_date: DateTime<Utc>,
    /// 結束時間
    pub end_date: DateTime<Utc>,
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
    /// 勝率
    pub win_rate: f64,
    /// 盈虧比
    pub profit_factor: f64,
    /// 詳細交易記錄
    pub trades: Option<Vec<BacktestTradeRecord>>,
    /// 詳細倉位記錄
    pub positions: Option<Vec<BacktestPositionRecord>>,
    /// 績效指標
    pub metrics: HashMap<String, f64>,
    /// 自定義結果數據
    pub custom_data: Option<serde_json::Value>,
}

/// 回測交易記錄
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestTradeRecord {
    /// 交易ID
    pub trade_id: String,
    /// 資產代碼
    pub asset_code: String,
    /// 交易方向 (buy/sell)
    pub direction: String,
    /// 開倉時間
    pub entry_time: DateTime<Utc>,
    /// 開倉價格
    pub entry_price: f64,
    /// 開倉數量
    pub entry_quantity: f64,
    /// 平倉時間
    pub exit_time: Option<DateTime<Utc>>,
    /// 平倉價格
    pub exit_price: Option<f64>,
    /// 平倉數量
    pub exit_quantity: Option<f64>,
    /// 持倉時間（小時）
    pub holding_period: Option<f64>,
    /// 收益
    pub profit: Option<f64>,
    /// 收益率
    pub return_pct: Option<f64>,
    /// 交易成本
    pub cost: f64,
    /// 交易標籤
    pub tags: Option<Vec<String>>,
}

/// 回測倉位記錄
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestPositionRecord {
    /// 資產代碼
    pub asset_code: String,
    /// 時間點
    pub timestamp: DateTime<Utc>,
    /// 倉位數量
    pub quantity: f64,
    /// 市值
    pub market_value: f64,
    /// 成本基礎
    pub cost_basis: f64,
    /// 未實現收益
    pub unrealized_pnl: f64,
    /// 已實現收益
    pub realized_pnl: f64,
}

//
// 策略相關回應
//

/// 策略信息回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyResponse {
    /// 策略ID
    pub strategy_id: String,
    /// 策略名稱
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 策略版本
    pub version: String,
    /// 創建時間
    pub created_at: DateTime<Utc>,
    /// 最後更新時間
    pub updated_at: DateTime<Utc>,
    /// 策略參數
    pub parameters: HashMap<String, serde_json::Value>,
    /// 策略標籤
    pub tags: Vec<String>,
    /// 策略代碼 (僅當請求時include_code=true)
    pub code: Option<String>,
}

/// 策略版本列表回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyVersionsResponse {
    /// 策略ID
    pub strategy_id: String,
    /// 策略名稱
    pub name: String,
    /// 策略版本列表
    pub versions: Vec<StrategyVersionInfo>,
}

/// 策略版本信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyVersionInfo {
    /// 版本號
    pub version: String,
    /// 創建時間
    pub created_at: DateTime<Utc>,
    /// 創建者
    pub created_by: Option<String>,
    /// 變更描述
    pub change_description: Option<String>,
}

//
// 數據相關回應
//

/// 市場數據回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketDataResponse<T> {
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
    /// 數據點數量
    pub count: usize,
    /// 是否有更多數據
    pub has_more: bool,
    /// 下一頁標記
    pub next_page_token: Option<String>,
    /// 數據點列表
    pub data: Vec<T>,
}

/// 資產列表回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetsResponse {
    /// 資產總數
    pub total_count: usize,
    /// 當前頁資產數量
    pub count: usize,
    /// 是否有更多數據
    pub has_more: bool,
    /// 下一頁標記
    pub next_page_token: Option<String>,
    /// 資產列表
    pub assets: Vec<AssetInfo>,
}

/// 資產信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetInfo {
    /// 資產代碼
    pub code: String,
    /// 資產名稱
    pub name: String,
    /// 資產類型
    pub asset_type: String,
    /// 交易所
    pub exchange: String,
    /// 是否可交易
    pub tradable: bool,
    /// 最小交易單位
    pub min_size: Option<f64>,
    /// 價格精度
    pub price_precision: Option<u8>,
    /// 額外信息
    pub metadata: Option<serde_json::Value>,
} 