use chrono::{DateTime, Utc};
use sqlx::types::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// 策略版本比較結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCompareResult {
    /// 版本相同
    Equal,
    /// 第一個版本較新
    Newer,
    /// 第一個版本較舊
    Older,
    /// 版本不可比較
    Incomparable,
}

/// 策略版本錯誤
#[derive(Error, Debug)]
pub enum VersionError {
    /// 資料庫錯誤
    #[error("資料庫錯誤: {0}")]
    DatabaseError(String),
    
    /// 版本不存在
    #[error("版本不存在: {0}")]
    VersionNotFound(String),
    
    /// 無效的版本格式
    #[error("無效的版本格式: {0}")]
    InvalidVersionFormat(String),
    
    /// 策略不存在
    #[error("策略不存在: {0}")]
    StrategyNotFound(String),
    
    /// IO錯誤
    #[error("IO錯誤: {0}")]
    IoError(String),
    
    /// 其他錯誤
    #[error("其他錯誤: {0}")]
    Other(String),
}


/// 策略版本模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StrategyVersion {
    pub version_id: i32,
    pub strategy_id: i32,
    pub version: String,
    pub source_path: String,
    pub is_stable: bool,
    pub description: Option<String>,
    pub created_by: String,
    pub metadata: Option<Json<HashMap<String, String>>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 策略版本插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyVersionInsert {
    pub strategy_id: String,
    pub version: String,
    pub source_path: String,
    pub is_stable: bool,
    pub description: Option<String>,
    pub created_by: String,
    pub metadata: Option<Json<HashMap<String, String>>>,
}

/// 策略版本更新模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyVersionUpdate {
    pub version_id: i32,
    pub is_stable: bool,
    pub metadata: Option<Json<HashMap<String, String>>>,
}

/// 策略版本查詢模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyVersionQuery {
    pub strategy_id: Option<String>,
    pub is_stable: Option<bool>,
    pub created_by: Option<String>,
    pub version_pattern: Option<String>,
}