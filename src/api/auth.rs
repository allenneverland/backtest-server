// src/api/auth.rs
use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
    body::to_bytes,
};
use sha2::{Sha256, Digest};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct ApiAuth {
    pub api_key: String,
    pub secret_key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Missing {0} header")]
    MissingHeader(String),
    
    #[error("Invalid {0}")]
    Invalid(String),
    
    #[error("Request expired")]
    Expired,
    
    #[error("Invalid signature")]
    InvalidSignature,
}

impl ApiAuth {
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self { api_key, secret_key }
    }
    
    pub fn verify_request(
        &self,
        method: &str,
        path: &str,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<(), AuthError> {
        // 1. 檢查 API Key
        let api_key = headers
            .get("X-API-Key")
            .ok_or(AuthError::MissingHeader("X-API-Key".to_string()))?
            .to_str()
            .map_err(|_| AuthError::Invalid("API Key".to_string()))?;
        
        if api_key != self.api_key {
            return Err(AuthError::Invalid("API Key".to_string()));
        }
        
        // 2. 檢查時間戳
        let timestamp_str = headers
            .get("X-Timestamp")
            .ok_or(AuthError::MissingHeader("X-Timestamp".to_string()))?
            .to_str()
            .map_err(|_| AuthError::Invalid("timestamp".to_string()))?;
        
        let timestamp = timestamp_str
            .parse::<i64>()
            .map_err(|_| AuthError::Invalid("timestamp format".to_string()))?;
        
        let now = Utc::now().timestamp();
        if (now - timestamp).abs() > 300 { // 5分鐘有效期
            return Err(AuthError::Expired);
        }
        
        // 3. 驗證簽名
        let signature = headers
            .get("X-Signature")
            .ok_or(AuthError::MissingHeader("X-Signature".to_string()))?
            .to_str()
            .map_err(|_| AuthError::Invalid("signature".to_string()))?;
        
        let computed_signature = self.compute_signature(method, path, timestamp, body);
        
        if signature != computed_signature {
            return Err(AuthError::InvalidSignature);
        }
        
        Ok(())
    }
    
    fn compute_signature(&self, method: &str, path: &str, timestamp: i64, body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}{}", method, path, timestamp, self.secret_key));
        hasher.update(body);
        hex::encode(hasher.finalize())
    }
}

// 確保 auth_middleware 是公開的
pub async fn auth_middleware(
    State(auth): State<ApiAuth>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {    
    // 提取請求信息
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    
    // 讀取 body（如果有）
    let (parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // 驗證請求
    auth.verify_request(method.as_str(), &path, &headers, &body_bytes)
        .map_err(|e| {
            tracing::warn!("Auth failed: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;
    
    // 重建請求並繼續處理
    let request = Request::from_parts(parts, axum::body::Body::from(body_bytes));
    Ok(next.run(request).await)
}