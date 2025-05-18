// monitor/logger.rs - 日誌記錄模組

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::monitor::MonitorResult;

use chrono::{DateTime, Utc};

/// 日誌級別枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// 跟踪級別 - 最詳細的日誌
    Trace,
    /// 調試級別 - 詳細的調試信息
    Debug,
    /// 信息級別 - 一般信息
    Info,
    /// 警告級別 - 警告但不影響程序運行
    Warn,
    /// 錯誤級別 - 運行時錯誤
    Error,
    /// 致命級別 - 嚴重錯誤，可能導致程序終止
    Fatal,
}

impl LogLevel {
    /// 將日誌級別轉換為字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 日誌條目結構
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// 時間戳
    timestamp: DateTime<Utc>,
    
    /// 日誌級別
    level: LogLevel,
    
    /// 日誌消息
    message: String,
    
    /// 來源模塊
    module: String,
    
    /// 標籤/額外信息
    metadata: HashMap<String, String>,
}

impl LogEntry {
    /// 創建新的日誌條目
    /// 
    /// 參數:
    ///     level: 日誌級別
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     Self: 新的日誌條目
    pub fn new(level: LogLevel, message: &str, module: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message: message.to_string(),
            module: module.to_string(),
            metadata: HashMap::new(),
        }
    }
    
    /// 添加元數據
    /// 
    /// 參數:
    ///     key: 鍵
    ///     value: 值
    /// 
    /// 返回:
    ///     &mut Self: 可鏈式調用
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
    
    /// 格式化日誌條目為字符串
    /// 
    /// 返回:
    ///     String: 格式化的日誌字符串
    pub fn format(&self) -> String {
        let mut formatted = format!(
            "[{}] {} [{}]: {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.level,
            self.module,
            self.message
        );
        
        if !self.metadata.is_empty() {
            formatted.push_str(" {");
            let metadata_str = self.metadata
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            formatted.push_str(&metadata_str);
            formatted.push('}');
        }
        
        formatted
    }
}

/// 日誌配置結構
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// 最小日誌級別
    pub min_level: LogLevel,
    
    /// 是否啟用控制台輸出
    pub console_output: bool,
    
    /// 是否啟用文件輸出
    pub file_output: bool,
    
    /// 日誌文件路徑
    pub log_file_path: String,
    
    /// 日誌文件最大大小（字節）
    pub max_file_size: usize,
    
    /// 保留的日誌文件數量
    pub max_files: usize,
    
    /// 是否顯示模塊名
    pub show_module: bool,
    
    /// 是否顯示時間戳
    pub show_timestamp: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            console_output: true,
            file_output: true,
            log_file_path: "logs/finrust.log".to_string(),
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            show_module: true,
            show_timestamp: true,
        }
    }
}

/// 日誌記錄器結構
#[derive(Debug)]
pub struct Logger {
    /// 日誌配置
    config: LoggerConfig,
    
    /// 日誌緩衝區
    buffer: Arc<RwLock<Vec<LogEntry>>>,
    
    /// 是否初始化
    initialized: bool,
}

impl Logger {
    /// 創建新的日誌記錄器
    /// 
    /// 參數:
    ///     config: 日誌配置
    /// 
    /// 返回:
    ///     Self: 新的日誌記錄器
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(RwLock::new(Vec::new())),
            initialized: false,
        }
    }
    
    /// 使用默認配置創建日誌記錄器
    /// 
    /// 返回:
    ///     Self: 新的日誌記錄器
    pub fn with_defaults() -> Self {
        Self::new(LoggerConfig::default())
    }
    
    /// 初始化日誌記錄器
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn initialize(&mut self) -> MonitorResult<()> {
        if self.initialized {
            return Ok(());
        }
        
        // 如果啟用了文件輸出，確保日誌目錄存在
        if self.config.file_output {
            let path = std::path::Path::new(&self.config.log_file_path);
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        crate::monitor::MonitorError::LoggerError(
                            format!("Failed to create log directory: {}", e)
                        )
                    })?;
                }
            }
        }
        
        self.initialized = true;
        
        // 記錄初始化成功日誌
        self.log(LogLevel::Info, "Logger initialized", "monitor::logger").await?;
        
        Ok(())
    }
    
    /// 記錄日誌
    /// 
    /// 參數:
    ///     level: 日誌級別
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn log(&self, level: LogLevel, message: &str, module: &str) -> MonitorResult<()> {
        // 檢查日誌級別
        if level < self.config.min_level {
            return Ok(());
        }
        
        // 創建日誌條目
        let entry = LogEntry::new(level, message, module);
        
        // 輸出到控制台
        if self.config.console_output {
            println!("{}", entry.format());
        }
        
        // 存儲到緩衝區
        let mut buffer = self.buffer.write().await;
        buffer.push(entry.clone());
        
        // 如果啟用了文件輸出，將日誌寫入文件
        if self.config.file_output {
            // 實際項目中，這裡應該使用非阻塞的方式寫入文件
            // 這裡簡化處理
            let formatted = entry.format();
            tokio::spawn(async move {
                if let Ok(mut file) = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&"logs/finrust.log")
                    .await
                {
                    // 在實際項目中，這裡應使用非阻塞寫入
                    let _ = tokio::io::AsyncWriteExt::write_all(&mut file, formatted.as_bytes()).await;
                    let _ = tokio::io::AsyncWriteExt::write_all(&mut file, b"\n").await;
                }
            });
        }
        
        Ok(())
    }
    
    /// 記錄跟踪級別日誌
    /// 
    /// 參數:
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn trace(&self, message: &str, module: &str) -> MonitorResult<()> {
        self.log(LogLevel::Trace, message, module).await
    }
    
    /// 記錄調試級別日誌
    /// 
    /// 參數:
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn debug(&self, message: &str, module: &str) -> MonitorResult<()> {
        self.log(LogLevel::Debug, message, module).await
    }
    
    /// 記錄信息級別日誌
    /// 
    /// 參數:
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn info(&self, message: &str, module: &str) -> MonitorResult<()> {
        self.log(LogLevel::Info, message, module).await
    }
    
    /// 記錄警告級別日誌
    /// 
    /// 參數:
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn warn(&self, message: &str, module: &str) -> MonitorResult<()> {
        self.log(LogLevel::Warn, message, module).await
    }
    
    /// 記錄錯誤級別日誌
    /// 
    /// 參數:
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn error(&self, message: &str, module: &str) -> MonitorResult<()> {
        self.log(LogLevel::Error, message, module).await
    }
    
    /// 記錄致命級別日誌
    /// 
    /// 參數:
    ///     message: 日誌消息
    ///     module: 來源模塊
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn fatal(&self, message: &str, module: &str) -> MonitorResult<()> {
        self.log(LogLevel::Fatal, message, module).await
    }
    
    /// 獲取日誌緩衝區
    /// 
    /// 返回:
    ///     Vec<LogEntry>: 日誌條目列表
    pub async fn get_logs(&self) -> Vec<LogEntry> {
        let buffer = self.buffer.read().await;
        buffer.clone()
    }
    
    /// 清空日誌緩衝區
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn clear_logs(&self) -> MonitorResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.clear();
        Ok(())
    }
    
    /// 設置最小日誌級別
    /// 
    /// 參數:
    ///     level: 新的最小日誌級別
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.config.min_level = level;
    }
} 