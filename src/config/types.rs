use serde::{Serialize, Deserialize};
use crate::config::validation::{ValidationError, ValidationUtils, Validator};

/// 應用程序配置結構
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub database: DatabaseConfig,
    pub log: LogConfig,
    pub app: AppConfig,
    pub strategy: StrategyConfig,
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub rabbitmq: RabbitMQConfig,
}

impl Validator for ApplicationConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證各個部分的配置
        self.database.validate()?;
        self.log.validate()?;
        self.app.validate()?;
        self.strategy.validate()?;
        self.server.validate()?;
        self.redis.validate()?;
        self.rabbitmq.validate()?;
        
        Ok(())
    }
}

/// 數據庫配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub connection_pool_size: u32,
    pub max_connections: u32,
    pub min_connections: u32,
    pub max_lifetime_secs: u64,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl Validator for DatabaseConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證數據庫配置
        ValidationUtils::not_empty(&self.host, "database.host")?;
        ValidationUtils::not_empty(&self.username, "database.username")?;
        ValidationUtils::not_empty(&self.database, "database.database")?;
        ValidationUtils::in_range(self.port, 1, 65535, "database.port")?;
        ValidationUtils::in_range(self.max_connections, self.min_connections, 1000, "database.max_connections")?;
        
        Ok(())
    }
}

impl DatabaseConfig {
    /// 獲取最大生命週期持續時間
    pub fn max_lifetime(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.max_lifetime_secs)
    }
    
    /// 獲取獲取連接超時持續時間
    pub fn acquire_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.acquire_timeout_secs)
    }
    
    /// 獲取閒置超時持續時間
    pub fn idle_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.idle_timeout_secs)
    }
}

/// 日誌配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
}

impl Validator for LogConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證日誌級別
        ValidationUtils::one_of(
            &self.level.to_lowercase(), 
            &["trace", "debug", "info", "warn", "error"].iter().map(|s| s.to_string()).collect::<Vec<String>>(), 
            "log.level"
        )?;
        
        // 驗證日誌格式
        ValidationUtils::one_of(
            &self.format.to_lowercase(),
            &["pretty", "json"].iter().map(|s| s.to_string()).collect::<Vec<String>>(),
            "log.format"
        )?;
        
        Ok(())
    }
}

/// 應用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub threads: u32,
}

impl Validator for AppConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證線程數
        ValidationUtils::in_range(self.threads, 1, 256, "app.threads")?;
        
        Ok(())
    }
}

/// 策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub directory: String,
    pub hot_update_enabled: bool,
    pub hot_update_interval_secs: u64,
    pub config_watch_interval_secs: u64,
    pub max_parallel_updates: u32,
    pub auto_reload_on_config_change: bool,
}

impl Validator for StrategyConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證策略配置
        ValidationUtils::not_empty(&self.directory, "strategy.directory")?;
        
        if self.hot_update_enabled {
            ValidationUtils::in_range(
                self.hot_update_interval_secs, 
                1, 
                3600, 
                "strategy.hot_update_interval_secs"
            )?;
        }
        
        ValidationUtils::in_range(
            self.max_parallel_updates,
            1,
            50,
            "strategy.max_parallel_updates"
        )?;
        
        Ok(())
    }
}

/// 伺服器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub worker_threads: u32,
    pub request_timeout: u64,
    pub max_connections: u32,
    pub use_https: bool,
    pub cert_path: String,
    pub key_path: String,
    pub enable_compression: bool,
    pub max_body_size: u64,
    pub enable_cors: bool,
    pub cors_allowed_origins: Vec<String>,
    pub static_files_dir: String,
}

impl Validator for ServerConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證服務器配置
        ValidationUtils::not_empty(&self.host, "server.host")?;
        ValidationUtils::in_range(self.port, 1, 65535, "server.port")?;
        ValidationUtils::in_range(self.worker_threads, 1, 256, "server.worker_threads")?;
        
        // HTTPS設定驗證
        if self.use_https {
            ValidationUtils::not_empty(&self.cert_path, "server.cert_path")?;
            ValidationUtils::not_empty(&self.key_path, "server.key_path")?;
        }
        
        // CORS設定驗證
        if self.enable_cors && self.cors_allowed_origins.is_empty() {
            return Err(ValidationError::InvalidValue(
                "啟用CORS但未指定允許的來源".to_string()
            ));
        }
        
        Ok(())
    }
}

/// REST API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestApiConfig {
    pub base_path: String,
    pub api_key: String,
    pub secret_key: String,
    pub request_timeout: u64,
    pub cors_allow_all: bool,
    pub cors_origins: Vec<String>,
}

impl Validator for RestApiConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證API配置
        ValidationUtils::not_empty(&self.base_path, "rest_api.base_path")?;
        ValidationUtils::not_empty(&self.api_key, "rest_api.api_key")?;
        ValidationUtils::not_empty(&self.secret_key, "rest_api.secret_key")?;
        
        // 如果不允許所有來源，必須指定允許的來源
        if !self.cors_allow_all && self.cors_origins.is_empty() {
            return Err(ValidationError::InvalidValue(
                "未指定允許的CORS來源，且未啟用允許所有來源".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Redis配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout_secs: u64,
    pub read_timeout_secs: u64,
    pub write_timeout_secs: u64,
    pub reconnect_attempts: u32,
    pub reconnect_delay_secs: u64,
}

impl Validator for RedisConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證Redis配置
        ValidationUtils::not_empty(&self.url, "redis.url")?;
        ValidationUtils::in_range(self.pool_size, 1, 100, "redis.pool_size")?;
        ValidationUtils::in_range(self.connection_timeout_secs, 1, 60, "redis.connection_timeout_secs")?;
        ValidationUtils::in_range(self.read_timeout_secs, 1, 60, "redis.read_timeout_secs")?;
        ValidationUtils::in_range(self.write_timeout_secs, 1, 60, "redis.write_timeout_secs")?;
        ValidationUtils::in_range(self.reconnect_attempts, 0, 10, "redis.reconnect_attempts")?;
        
        Ok(())
    }
}

/// RabbitMQ配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RabbitMQConfig {
    /// AMQP URL (例如: "amqp://user:pass@localhost:5672/")
    pub url: String,
    /// 連接池大小
    pub pool_size: u32,
    /// 連接超時（秒）
    pub connection_timeout_secs: u64,
    /// 重試間隔（秒）
    pub retry_interval_secs: u64,
    /// 最大重試次數
    pub max_retries: u32,
    /// 預設交換機類型
    pub default_exchange_type: String,
    /// 預設交換機名稱
    pub default_exchange: String,
    /// 是否持久化消息
    pub durable_messages: bool,
    /// 發佈確認
    pub publish_confirm: bool,
    /// 預取計數
    pub prefetch_count: u16,
    /// 消費者標籤前綴
    pub consumer_tag_prefix: String,
}

impl Default for RabbitMQConfig {
    fn default() -> Self {
        Self {
            url: "amqp://guest:guest@localhost:5672/".to_string(),
            pool_size: 10,
            connection_timeout_secs: 5,
            retry_interval_secs: 1,
            max_retries: 5,
            default_exchange_type: "direct".to_string(),
            default_exchange: "backtest.direct".to_string(),
            durable_messages: true,
            publish_confirm: true,
            prefetch_count: 10,
            consumer_tag_prefix: "backtest_server".to_string(),
        }
    }
}

impl Validator for RabbitMQConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // 驗證RabbitMQ配置
        ValidationUtils::not_empty(&self.url, "rabbitmq.url")?;
        ValidationUtils::in_range(self.pool_size, 1, 100, "rabbitmq.pool_size")?;
        ValidationUtils::in_range(self.connection_timeout_secs, 1, 60, "rabbitmq.connection_timeout_secs")?;
        
        // 驗證交換機類型
        ValidationUtils::one_of(
            &self.default_exchange_type.to_lowercase(),
            &["direct", "topic", "fanout", "headers"].iter().map(|s| s.to_string()).collect::<Vec<String>>(),
            "rabbitmq.default_exchange_type"
        )?;
        
        ValidationUtils::not_empty(&self.default_exchange, "rabbitmq.default_exchange")?;
        ValidationUtils::not_empty(&self.consumer_tag_prefix, "rabbitmq.consumer_tag_prefix")?;
        
        Ok(())
    }
} 