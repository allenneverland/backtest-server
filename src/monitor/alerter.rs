// monitor/alerter.rs - 警報系統模組

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crate::monitor::MonitorResult;

use chrono::{DateTime, Utc};

/// 警報級別枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    /// 信息級別
    Info,
    /// 警告級別
    Warning,
    /// 錯誤級別
    Error,
    /// 嚴重級別
    Critical,
}

impl AlertLevel {
    /// 將警報級別轉換為字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertLevel::Info => "INFO",
            AlertLevel::Warning => "WARNING",
            AlertLevel::Error => "ERROR",
            AlertLevel::Critical => "CRITICAL",
        }
    }
}

/// 警報結構
#[derive(Debug, Clone)]
pub struct Alert {
    /// 警報 ID
    id: String,
    
    /// 警報發生時間
    timestamp: DateTime<Utc>,
    
    /// 警報級別
    level: AlertLevel,
    
    /// 警報標題
    title: String,
    
    /// 警報詳細描述
    description: String,
    
    /// 警報來源
    source: String,
    
    /// 相關標籤/元數據
    tags: HashMap<String, String>,
    
    /// 是否已處理
    resolved: bool,
    
    /// 處理時間（如果已處理）
    resolved_at: Option<DateTime<Utc>>,
    
    /// 處理者（如果已處理）
    resolved_by: Option<String>,
    
    /// 處理備註
    resolution_notes: Option<String>,
}

impl Alert {
    /// 創建新的警報
    /// 
    /// 參數:
    ///     id: 警報ID
    ///     level: 警報級別
    ///     title: 警報標題
    ///     description: 警報詳細描述
    ///     source: 警報來源
    /// 
    /// 返回:
    ///     Self: 新的警報
    pub fn new(
        id: &str,
        level: AlertLevel,
        title: &str,
        description: &str,
        source: &str
    ) -> Self {
        Self {
            id: id.to_string(),
            timestamp: Utc::now(),
            level,
            title: title.to_string(),
            description: description.to_string(),
            source: source.to_string(),
            tags: HashMap::new(),
            resolved: false,
            resolved_at: None,
            resolved_by: None,
            resolution_notes: None,
        }
    }
    
    /// 添加標籤
    /// 
    /// 參數:
    ///     key: 標籤鍵
    ///     value: 標籤值
    /// 
    /// 返回:
    ///     &mut Self: 可鏈式調用
    pub fn with_tag(mut self, key: &str, value: &str) -> Self {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }
    
    /// 標記警報為已處理
    /// 
    /// 參數:
    ///     resolved_by: 處理者
    ///     notes: 處理備註
    /// 
    /// 返回:
    ///     &mut Self: 可鏈式調用
    pub fn resolve(&mut self, resolved_by: &str, notes: Option<&str>) -> &mut Self {
        self.resolved = true;
        self.resolved_at = Some(Utc::now());
        self.resolved_by = Some(resolved_by.to_string());
        self.resolution_notes = notes.map(|n| n.to_string());
        self
    }
    
    /// 獲取警報ID
    /// 
    /// 返回:
    ///     &str: 警報ID
    pub fn id(&self) -> &str {
        &self.id
    }
    
    /// 檢查警報是否已處理
    /// 
    /// 返回:
    ///     bool: 是否已處理
    pub fn is_resolved(&self) -> bool {
        self.resolved
    }
    
    /// 獲取警報級別
    /// 
    /// 返回:
    ///     AlertLevel: 警報級別
    pub fn level(&self) -> AlertLevel {
        self.level
    }
    
    /// 獲取警報標題
    /// 
    /// 返回:
    ///     &str: 警報標題
    pub fn title(&self) -> &str {
        &self.title
    }
    
    /// 格式化警報為字符串
    /// 
    /// 返回:
    ///     String: 格式化的警報字符串
    pub fn format(&self) -> String {
        let status = if self.resolved {
            format!(
                "RESOLVED by {} at {}", 
                self.resolved_by.as_ref().unwrap_or(&"unknown".to_string()),
                self.resolved_at.unwrap().format("%Y-%m-%d %H:%M:%S")
            )
        } else {
            "ACTIVE".to_string()
        };
        
        format!(
            "[{}] {} - {} [{}]: {} (Source: {})",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.level.as_str(),
            self.title,
            status,
            self.description,
            self.source
        )
    }
}

/// 警報配置結構
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// 最小警報級別（低於此級別不觸發警報）
    pub min_level: AlertLevel,
    
    /// 是否啟用控制台輸出
    pub console_output: bool,
    
    /// 是否啟用郵件通知
    pub email_notifications: bool,
    
    /// 郵件接收人列表
    pub email_recipients: Vec<String>,
    
    /// 是否啟用應用內通知
    pub in_app_notifications: bool,
    
    /// 警報去重時間窗口（秒）
    pub deduplication_window_secs: u64,
    
    /// 警報保留時間（秒）
    pub retention_period_secs: u64,
    
    /// 自動處理相同警報的數量限制
    pub auto_resolve_same_count: u32,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            min_level: AlertLevel::Warning,
            console_output: true,
            email_notifications: false,
            email_recipients: Vec::new(),
            in_app_notifications: true,
            deduplication_window_secs: 300, // 5分鐘
            retention_period_secs: 604800, // 7天
            auto_resolve_same_count: 0, // 不自動處理
        }
    }
}

/// 警報引擎結構
#[derive(Debug)]
pub struct Alerter {
    /// 警報配置
    config: AlertConfig,
    
    /// 活躍警報
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    
    /// 歷史警報
    historical_alerts: Arc<RwLock<Vec<Alert>>>,
    
    /// 警報計數器
    alert_counter: Arc<RwLock<HashMap<String, u32>>>,
    
    /// 最後警報時間
    last_alert_time: Arc<RwLock<HashMap<String, Instant>>>,
    
    /// 是否初始化
    initialized: bool,
}

impl Alerter {
    /// 創建新的警報引擎
    /// 
    /// 參數:
    ///     config: 警報配置
    /// 
    /// 返回:
    ///     Self: 新的警報引擎
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            historical_alerts: Arc::new(RwLock::new(Vec::new())),
            alert_counter: Arc::new(RwLock::new(HashMap::new())),
            last_alert_time: Arc::new(RwLock::new(HashMap::new())),
            initialized: false,
        }
    }
    
    /// 使用默認配置創建警報引擎
    /// 
    /// 返回:
    ///     Self: 新的警報引擎
    pub fn with_defaults() -> Self {
        Self::new(AlertConfig::default())
    }
    
    /// 初始化警報引擎
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn initialize(&mut self) -> MonitorResult<()> {
        if self.initialized {
            return Ok(());
        }
        
        // 可以在這裡進行其他初始化，如連接外部告警系統
        
        self.initialized = true;
        
        // 啟動後台清理任務
        let config = self.config.clone();
        let historical_alerts = self.historical_alerts.clone();
        
        // 清理過期的歷史警報
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 每小時清理一次
            loop {
                interval.tick().await;
                
                let cutoff = Utc::now() - chrono::Duration::seconds(config.retention_period_secs as i64);
                let mut alerts = historical_alerts.write().await;
                alerts.retain(|alert| alert.timestamp > cutoff);
            }
        });
        
        Ok(())
    }
    
    /// 觸發新的警報
    /// 
    /// 參數:
    ///     level: 警報級別
    ///     title: 警報標題
    ///     description: 警報詳細描述
    ///     source: 警報來源
    ///     key: 去重鍵（可選，如果需要去重）
    /// 
    /// 返回:
    ///     MonitorResult<String>: 成功時返回警報ID，失敗時返回錯誤
    pub async fn trigger(
        &self,
        level: AlertLevel,
        title: &str,
        description: &str,
        source: &str,
        key: Option<&str>
    ) -> MonitorResult<String> {
        // 檢查警報級別
        if level < self.config.min_level {
            return Ok("alert_level_too_low".to_string());
        }
        
        // 檢查去重
        if let Some(dedup_key) = key {
            let last_alert_time = self.last_alert_time.read().await;
            if let Some(time) = last_alert_time.get(dedup_key) {
                let elapsed = time.elapsed();
                if elapsed < Duration::from_secs(self.config.deduplication_window_secs) {
                    // 更新計數器
                    let mut counter = self.alert_counter.write().await;
                    let count = counter.entry(dedup_key.to_string()).or_insert(0);
                    *count += 1;
                    
                    // 如果配置了自動處理相同警報
                    if self.config.auto_resolve_same_count > 0 
                       && *count >= self.config.auto_resolve_same_count {
                        // 找到已存在的警報
                        let mut active_alerts = self.active_alerts.write().await;
                        let mut alert_to_remove = None;
                        
                        // 先查找需要處理的警報
                        for (id, alert) in active_alerts.iter_mut() {
                            if alert.tags.get("dedup_key") == Some(&dedup_key.to_string()) {
                                // 自動處理
                                alert.resolve(
                                    "system",
                                    Some(&format!("Auto-resolved after {} occurrences", count))
                                );
                                
                                // 保存警報 ID 以便後續移除
                                alert_to_remove = Some((id.clone(), alert.clone()));
                                break;
                            }
                        }
                        
                        // 如果找到了需要處理的警報
                        if let Some((id, alert)) = alert_to_remove {
                            // 將警報添加到歷史記錄
                            let mut historical = self.historical_alerts.write().await;
                            historical.push(alert);
                            
                            // 從活躍警報中移除
                            active_alerts.remove(&id);
                        }
                    }
                    
                    return Ok(format!("deduplicated_alert_{}", dedup_key));
                }
            }
        }
        
        // 生成警報ID
        let alert_id = format!("alert_{}", uuid::Uuid::new_v4());
        
        // 創建警報
        let mut alert = Alert::new(
            &alert_id,
            level,
            title,
            description,
            source
        );
        
        // 如果提供了去重鍵，添加為標籤
        if let Some(dedup_key) = key {
            alert = alert.with_tag("dedup_key", dedup_key);
            
            // 更新最後警報時間
            let mut last_alert_time = self.last_alert_time.write().await;
            last_alert_time.insert(dedup_key.to_string(), Instant::now());
            
            // 重置計數器
            let mut counter = self.alert_counter.write().await;
            counter.insert(dedup_key.to_string(), 1);
        }
        
        // 輸出到控制台
        if self.config.console_output {
            println!("ALERT: {}", alert.format());
        }
        
        // 如果啟用了電子郵件通知，在這裡發送
        if self.config.email_notifications && !self.config.email_recipients.is_empty() {
            // 實際項目中，這裡會調用郵件發送系統
            let emails = self.config.email_recipients.join(", ");
            println!("Sending alert email to: {}", emails);
        }
        
        // 存儲警報
        let mut active_alerts = self.active_alerts.write().await;
        active_alerts.insert(alert_id.clone(), alert);
        
        Ok(alert_id)
    }
    
    /// 處理警報
    /// 
    /// 參數:
    ///     alert_id: 警報ID
    ///     resolved_by: 處理者
    ///     notes: 處理備註
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn resolve_alert(
        &self,
        alert_id: &str,
        resolved_by: &str,
        notes: Option<&str>
    ) -> MonitorResult<()> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            // 標記為已處理
            alert.resolve(resolved_by, notes);
            
            // 添加到歷史記錄
            let mut historical = self.historical_alerts.write().await;
            historical.push(alert.clone());
            
            // 從活躍警報中移除
            active_alerts.remove(alert_id);
            
            Ok(())
        } else {
            Err(crate::monitor::MonitorError::AlerterError(
                format!("Alert with ID '{}' not found", alert_id)
            ))
        }
    }
    
    /// 獲取活躍警報
    /// 
    /// 返回:
    ///     HashMap<String, Alert>: 活躍警報映射
    pub async fn get_active_alerts(&self) -> HashMap<String, Alert> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.clone()
    }
    
    /// 獲取歷史警報
    /// 
    /// 參數:
    ///     limit: 返回的最大數量
    /// 
    /// 返回:
    ///     Vec<Alert>: 歷史警報列表
    pub async fn get_historical_alerts(&self, limit: usize) -> Vec<Alert> {
        let historical = self.historical_alerts.read().await;
        let start = if historical.len() > limit {
            historical.len() - limit
        } else {
            0
        };
        historical[start..].to_vec()
    }
    
    /// 獲取特定級別的活躍警報數量
    /// 
    /// 參數:
    ///     level: 警報級別
    /// 
    /// 返回:
    ///     usize: 警報數量
    pub async fn count_active_alerts_by_level(&self, level: AlertLevel) -> usize {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values()
            .filter(|alert| alert.level() == level)
            .count()
    }
    
    /// 檢查是否有嚴重警報
    /// 
    /// 返回:
    ///     bool: 是否有嚴重警報
    pub async fn has_critical_alerts(&self) -> bool {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values()
            .any(|alert| alert.level() == AlertLevel::Critical)
    }
    
    /// 設置最小警報級別
    /// 
    /// 參數:
    ///     level: 新的最小警報級別
    pub fn set_min_level(&mut self, level: AlertLevel) {
        self.config.min_level = level;
    }
} 