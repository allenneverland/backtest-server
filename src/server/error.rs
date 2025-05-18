// error.rs - 伺服器級別錯誤處理

use std::error::Error;
use std::fmt;
use std::io;

/// 伺服器錯誤類型
#[derive(Debug)]
pub enum ServerError {
    /// IO錯誤
    Io(io::Error),
    
    /// 配置錯誤
    InvalidConfiguration(String),
    
    /// 初始化錯誤
    InitializationFailed(String),
    
    /// 路由處理錯誤
    RoutingError(String),
    
    /// 請求處理錯誤
    RequestHandlingError(String),
    
    /// 連接錯誤
    ConnectionError(String),
    
    /// 權限錯誤
    PermissionDenied(String),
    
    /// 伺服器已經啟動
    ServerAlreadyRunning,
    
    /// 伺服器未運行
    ServerNotRunning,
    
    /// 其他錯誤
    Other(String),
}

/// 伺服器結果類型
pub type ServerResult<T> = Result<T, ServerError>;

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::Io(err) => write!(f, "IO error: {}", err),
            ServerError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            ServerError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            ServerError::RoutingError(msg) => write!(f, "Routing error: {}", msg),
            ServerError::RequestHandlingError(msg) => write!(f, "Request handling error: {}", msg),
            ServerError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            ServerError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            ServerError::ServerAlreadyRunning => write!(f, "Server is already running"),
            ServerError::ServerNotRunning => write!(f, "Server is not running"),
            ServerError::Other(msg) => write!(f, "Server error: {}", msg),
        }
    }
}

impl Error for ServerError {}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> Self {
        ServerError::Io(err)
    }
}

/// 轉換為HTTP狀態碼
impl ServerError {
    /// 將錯誤轉換為對應的HTTP狀態碼
    /// 
    /// 返回:
    ///     u16: HTTP狀態碼
    pub fn status_code(&self) -> u16 {
        match self {
            ServerError::Io(_) => 500,
            ServerError::InvalidConfiguration(_) => 500,
            ServerError::InitializationFailed(_) => 500,
            ServerError::RoutingError(_) => 404,
            ServerError::RequestHandlingError(_) => 400,
            ServerError::ConnectionError(_) => 503,
            ServerError::PermissionDenied(_) => 403,
            ServerError::ServerAlreadyRunning => 409,
            ServerError::ServerNotRunning => 503,
            ServerError::Other(_) => 500,
        }
    }
    
    /// 獲取錯誤描述
    /// 
    /// 返回:
    ///     String: 伺服器錯誤的詳細描述
    pub fn description(&self) -> String {
        match self {
            ServerError::Io(err) => format!("IO error occurred: {}", err),
            ServerError::InvalidConfiguration(msg) => format!("Configuration error: {}", msg),
            ServerError::InitializationFailed(msg) => format!("Server initialization failed: {}", msg),
            ServerError::RoutingError(msg) => format!("Routing error: {}", msg),
            ServerError::RequestHandlingError(msg) => format!("Invalid request: {}", msg),
            ServerError::ConnectionError(msg) => format!("Connection error: {}", msg),
            ServerError::PermissionDenied(msg) => format!("Access denied: {}", msg),
            ServerError::ServerAlreadyRunning => "Server is already running".to_string(),
            ServerError::ServerNotRunning => "Server is not running".to_string(),
            ServerError::Other(msg) => format!("Server error: {}", msg),
        }
    }
    
    /// 檢查是否為伺服器內部錯誤
    /// 
    /// 返回:
    ///     bool: 是否為內部錯誤
    pub fn is_internal(&self) -> bool {
        matches!(
            self,
            ServerError::Io(_) | 
            ServerError::InvalidConfiguration(_) | 
            ServerError::InitializationFailed(_) |
            ServerError::Other(_)
        )
    }
} 