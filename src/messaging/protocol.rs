use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 消息封裝，用於所有消息通訊
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message<T> {
    /// 唯一消息ID
    pub message_id: String,
    /// 消息類型
    pub message_type: String,
    /// 相關ID (用於請求-回應)
    pub correlation_id: Option<String>,
    /// 發送時間
    pub timestamp: DateTime<Utc>,
    /// 認證令牌 (可選)
    pub auth_token: Option<String>,
    /// 消息載荷
    pub payload: T,
}

impl<T> Message<T> {
    /// 創建新消息
    pub fn new(message_type: &str, payload: T) -> Self {
        Self {
            message_id: Uuid::new_v4().to_string(),
            message_type: message_type.to_string(),
            correlation_id: None,
            timestamp: Utc::now(),
            auth_token: None,
            payload,
        }
    }
    
    /// 設置認證令牌
    pub fn with_auth(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }
    
    /// 設置相關ID (用於回應)
    pub fn as_response_to(mut self, request_id: &str) -> Self {
        self.correlation_id = Some(request_id.to_string());
        self
    }
}

/// 標準錯誤回應
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}