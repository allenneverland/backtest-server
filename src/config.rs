use anyhow::{Result, Error as AnyError};
use config::{Config, ConfigError, Environment, File};
use once_cell::sync::OnceCell;
use serde::{Deserialize};
use std::time::Duration;
use crate::utils::empty_string_as_none;

// 全局配置實例
static CONFIG: OnceCell<AppConfig> = OnceCell::new();

/// 應用程式配置結構
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// 數據庫配置
    pub database: DatabaseConfig,
    /// 日誌配置
    pub log: LogConfig,
    /// 應用配置
    pub app: ApplicationConfig,
    /// 數據源配置
    pub data_source: DataSourceConfig,
    /// 策略配置
    pub strategy: StrategyConfig,
    /// 伺服器配置
    pub server: ServerConfig,
    /// REST API 配置
    pub rest_api: RestApiConfig,
}

/// 數據庫配置
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub max_lifetime_secs: Option<u64>,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: Option<u64>,
}

/// 日誌配置
#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
}

/// 應用配置
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationConfig {
    pub threads: u8,
}

/// 數據源類型枚舉
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum DataSourceType {
    File,
    Database,
    Api,
}

/// 數據格式枚舉
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum DataFormat {
    Csv,
    Json,
    Parquet,
    Database,
}

/// 策略配置
#[derive(Debug, Clone, Deserialize)]
pub struct StrategyConfig {
    /// 策略目錄路徑
    pub directory: String,
    /// 是否啟用熱更新
    #[serde(default = "default_hot_update_enabled")]
    pub hot_update_enabled: bool,
    /// 熱更新檢查間隔（秒）
    #[serde(default = "default_hot_update_interval")]
    pub hot_update_interval_secs: u64,
    /// 配置文件監控間隔（秒）
    #[serde(default = "default_config_watch_interval")]
    pub config_watch_interval_secs: u64,
    /// 最大並行更新數量
    #[serde(default = "default_max_parallel_updates")]
    pub max_parallel_updates: u32,
    /// 配置更新自動重載
    #[serde(default = "default_auto_reload")]
    pub auto_reload_on_config_change: bool,
}

/// 伺服器配置
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// 伺服器監聽主機名
    pub host: String,
    /// 伺服器監聽端口
    pub port: u16,
    /// 工作線程數
    pub worker_threads: usize,
    /// 請求超時設定(秒)
    pub request_timeout: u64,
    /// 最大並發連接數
    pub max_connections: usize,
    /// 是否啟用HTTPS
    pub use_https: bool,
    /// HTTPS證書路徑(選填)
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub cert_path: Option<String>,
    /// HTTPS私鑰路徑(選填)
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub key_path: Option<String>,
    /// 是否啟用Gzip壓縮
    pub enable_compression: bool,
    /// 請求主體大小限制(bytes)
    pub max_body_size: usize,
    /// 是否啟用跨域請求
    pub enable_cors: bool,
    /// 跨域允許的來源網址
    pub cors_allowed_origins: Vec<String>,
    /// 靜態文件目錄
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub static_files_dir: Option<String>,
}

/// REST API 配置
#[derive(Debug, Clone, Deserialize)]
pub struct RestApiConfig {
    /// API基礎路徑
    pub base_path: String,
    /// API密鑰
    pub api_key: String,
    /// API密鑰(密碼)
    pub secret_key: String,
    /// 請求超時(秒)
    pub request_timeout: u64,
    /// 是否允許所有跨域請求
    pub cors_allow_all: bool,
    /// 允許的跨域來源
    pub cors_origins: Vec<String>,
}

impl ServerConfig {
    /// 獲取請求超時Duration
    pub fn request_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.request_timeout)
    }
    
    /// 構建伺服器地址字符串
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl RestApiConfig {
    /// 獲取API請求超時Duration
    pub fn request_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.request_timeout)
    }
}

fn default_hot_update_enabled() -> bool {
    true
}

fn default_hot_update_interval() -> u64 {
    30
}

fn default_config_watch_interval() -> u64 {
    15
}

fn default_max_parallel_updates() -> u32 {
    5
}

fn default_auto_reload() -> bool {
    true
}

/// 數據源配置
#[derive(Debug, Clone, Deserialize)]
pub struct DataSourceConfig {
    /// 數據源類型
    pub source_type: DataSourceType,
    /// 數據文件路徑（當 source_type 為 File 時使用）
    pub path: Option<String>,
    /// 數據格式
    pub format: DataFormat,
    /// API 端點（當 source_type 為 Api 時使用）
    pub api_endpoint: Option<String>,
    /// API 認證令牌（當 source_type 為 Api 時使用）
    pub api_token: Option<String>,
}

// 定義配置相關錯誤
#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("無效的環境: {0}，只支持 development 或 production")]
    InvalidEnvironment(String),
    
    #[error("配置檔案讀取錯誤: {0}")]
    ConfigError(#[from] ConfigError),
    
    #[error("驗證錯誤: {0}")]
    ValidationError(String),
    
    #[error("其他錯誤: {0}")]
    Other(#[from] AnyError),
}

impl DatabaseConfig {
    /// 獲取 max_lifetime Duration
    pub fn max_lifetime(&self) -> Option<Duration> {
        self.max_lifetime_secs.map(Duration::from_secs)
    }
    
    /// 獲取 acquire_timeout Duration
    pub fn acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout_secs)
    }
    
    /// 獲取 idle_timeout Duration
    pub fn idle_timeout(&self) -> Option<Duration> {
        self.idle_timeout_secs.map(Duration::from_secs)
    }
    
    /// 獲取完整的數據庫連接 URL
    pub fn connection_url(&self) -> String {
        format!("postgres://{}:{}@{}:{}/{}", 
            self.username, self.password, self.host, self.port, self.database)
    }
}

impl AppConfig {
    /// 根據運行環境載入配置
    pub fn load() -> std::result::Result<Self, ConfigurationError> {
        // 獲取環境，預設為開發環境
        let env = std::env::var("FINRUST_ENV").unwrap_or_else(|_| "development".into());
        
        // 檢查環境是否有效
        if env != "development" && env != "production" {
            return Err(ConfigurationError::InvalidEnvironment(env));
        }
        
        // 構建設定
        let config = Config::builder()
            // 僅載入環境特定配置
            .add_source(File::with_name(&format!("config/{}", env)).required(true))
            // 環境變量（優先級最高）
            .add_source(Environment::with_prefix("FINRUST").separator("__"))
            .build()?;

        // 解析配置到結構體
        let app_config: AppConfig = config.try_deserialize()
            .map_err(|e| ConfigurationError::ConfigError(e))?;
        
        // 驗證配置
        Self::validate(&app_config)?;
        
        Ok(app_config)
    }

    /// 驗證配置，確保配置有效
    fn validate(config: &AppConfig) -> std::result::Result<(), ConfigurationError> {
        // 檢查數據庫主機非空
        if config.database.host.is_empty() {
            return Err(ConfigurationError::ValidationError("數據庫主機不能為空".to_string()));
        }
        
        // 檢查數據庫用戶名非空
        if config.database.username.is_empty() {
            return Err(ConfigurationError::ValidationError("數據庫用戶名不能為空".to_string()));
        }
        
        // 檢查數據庫名稱非空
        if config.database.database.is_empty() {
            return Err(ConfigurationError::ValidationError("數據庫名稱不能為空".to_string()));
        }
        
        // 檢查數據庫端口有效
        if config.database.port == 0 {
            return Err(ConfigurationError::ValidationError("數據庫端口不能為零".to_string()));
        }
        
        // 檢查連接數有效
        if config.database.max_connections == 0 {
            return Err(ConfigurationError::ValidationError("最大連接數不能為零".to_string()));
        }
        
        // 檢查最小連接數不大於最大連接數
        if config.database.min_connections > config.database.max_connections {
            return Err(ConfigurationError::ValidationError("最小連接數不能大於最大連接數".to_string()));
        }

        // 檢查端口號有效
        if config.app.threads == 0 {
            return Err(ConfigurationError::ValidationError("應用線程數不能為零".to_string()));
        }
        
        // 驗證數據源配置
        match config.data_source.source_type {
            DataSourceType::File => {
                if config.data_source.path.is_none() {
                    return Err(ConfigurationError::ValidationError("文件數據源必須指定路徑".to_string()));
                }
            },
            DataSourceType::Api => {
                if config.data_source.api_endpoint.is_none() {
                    return Err(ConfigurationError::ValidationError("API數據源必須指定端點".to_string()));
                }
            },
            _ => {}
        }
        
        // 驗證策略配置
        if config.strategy.directory.is_empty() {
            return Err(ConfigurationError::ValidationError("策略目錄不能為空".to_string()));
        }
        
        if config.strategy.hot_update_interval_secs == 0 {
            return Err(ConfigurationError::ValidationError("熱更新檢查間隔不能為零".to_string()));
        }
        
        if config.strategy.config_watch_interval_secs == 0 {
            return Err(ConfigurationError::ValidationError("配置監控間隔不能為零".to_string()));
        }
        
        if config.strategy.max_parallel_updates == 0 {
            return Err(ConfigurationError::ValidationError("最大並行更新數不能為零".to_string()));
        }
        
        // 驗證伺服器配置
        if config.server.host.is_empty() {
            return Err(ConfigurationError::ValidationError("伺服器主機不能為空".to_string()));
        }
        
        if config.server.port == 0 {
            return Err(ConfigurationError::ValidationError("伺服器端口不能為零".to_string()));
        }
        
        if config.server.worker_threads == 0 {
            return Err(ConfigurationError::ValidationError("伺服器工作線程數不能為零".to_string()));
        }
        
        if config.server.request_timeout == 0 {
            return Err(ConfigurationError::ValidationError("請求超時時間不能為零".to_string()));
        }
        
        if config.server.max_connections == 0 {
            return Err(ConfigurationError::ValidationError("伺服器最大連接數不能為零".to_string()));
        }
        
        if config.server.use_https {
            if config.server.cert_path.is_none() || config.server.cert_path.as_ref().unwrap().is_empty() {
                return Err(ConfigurationError::ValidationError("啟用HTTPS時必須提供證書路徑".to_string()));
            }
            
            if config.server.key_path.is_none() || config.server.key_path.as_ref().unwrap().is_empty() {
                return Err(ConfigurationError::ValidationError("啟用HTTPS時必須提供私鑰路徑".to_string()));
            }
        }
        
        Ok(())
    }
}

/// 獲取全局配置
pub fn get_config() -> Result<&'static AppConfig> {
    CONFIG.get().ok_or_else(|| anyhow::anyhow!("配置未初始化"))
}

/// 初始化全局配置
pub fn init_config() -> Result<&'static AppConfig> {
    let config = AppConfig::load()?;
    CONFIG.set(config).map_err(|_| anyhow::anyhow!("配置已初始化"))?;
    Ok(CONFIG.get().unwrap())
} 