// monitor/metrics.rs - 監控指標收集模組

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crate::monitor::MonitorResult;

/// 指標類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    /// 計數器（只增不減）
    Counter,
    /// 計量器（可增可減）
    Gauge,
    /// 直方圖（分布情況）
    Histogram,
    /// 摘要（百分位數等統計量）
    Summary,
    /// 計時器（測量時間）
    Timer,
}

/// 度量指標
#[derive(Debug, Clone)]
pub struct Metric {
    /// 指標名稱
    name: String,
    
    /// 指標類型
    #[allow(dead_code)]
    metric_type: MetricType,
    
    /// 指標描述
    #[allow(dead_code)]
    description: String,
    
    /// 標籤/標記
    labels: HashMap<String, String>,
    
    /// 數值（根據類型不同有不同含義）
    value: f64,
    
    /// 最後更新時間
    last_updated: Instant,
    
    /// 創建時間
    #[allow(dead_code)]
    created_at: Instant,
    
    /// 是否啟用
    #[allow(dead_code)]
    pub enabled: bool,
}

impl Metric {
    /// 創建新指標
    /// 
    /// 參數:
    ///     name: 指標名稱
    ///     metric_type: 指標類型
    ///     description: 指標描述
    /// 
    /// 返回:
    ///     Self: 新建的指標
    pub fn new(name: &str, metric_type: MetricType, description: &str) -> Self {
        Self {
            name: name.to_string(),
            metric_type,
            description: description.to_string(),
            labels: HashMap::new(),
            value: 0.0,
            last_updated: Instant::now(),
            created_at: Instant::now(),
            enabled: true,
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
    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }
    
    /// 設置指標值
    /// 
    /// 參數:
    ///     value: 新的指標值
    /// 
    /// 返回:
    ///     &mut Self: 可鏈式調用
    pub fn set_value(&mut self, value: f64) -> &mut Self {
        self.value = value;
        self.last_updated = Instant::now();
        self
    }
    
    /// 增加指標值
    /// 
    /// 參數:
    ///     amount: 增加量
    /// 
    /// 返回:
    ///     &mut Self: 可鏈式調用
    pub fn increment(&mut self, amount: f64) -> &mut Self {
        self.value += amount;
        self.last_updated = Instant::now();
        self
    }
    
    /// 獲取指標值
    /// 
    /// 返回:
    ///     f64: 當前指標值
    pub fn value(&self) -> f64 {
        self.value
    }
    
    /// 獲取指標名稱
    /// 
    /// 返回:
    ///     &str: 指標名稱
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// 獲取自上次更新以來的時間
    /// 
    /// 返回:
    ///     Duration: 經過的時間
    pub fn time_since_update(&self) -> Duration {
        self.last_updated.elapsed()
    }
}

/// 指標配置結構
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// 是否啟用指標收集
    pub enabled: bool,
    
    /// 指標上報間隔（秒）
    pub reporting_interval_secs: u64,
    
    /// 指標存儲持續時間（秒）
    pub retention_period_secs: u64,
    
    /// 直方圖桶數量
    pub histogram_buckets: usize,
    
    /// 是否啟用系統指標
    pub enable_system_metrics: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reporting_interval_secs: 60,
            retention_period_secs: 86400, // 24小時
            histogram_buckets: 10,
            enable_system_metrics: true,
        }
    }
}

/// 指標收集器結構
#[derive(Debug)]
pub struct MetricsCollector {
    /// 指標配置
    config: MetricsConfig,
    
    /// 指標存儲
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    
    /// 是否初始化
    initialized: bool,
}

impl MetricsCollector {
    /// 創建新的指標收集器
    /// 
    /// 參數:
    ///     config: 指標配置
    /// 
    /// 返回:
    ///     Self: 新的指標收集器
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(HashMap::new())),
            initialized: false,
        }
    }
    
    /// 使用默認配置創建指標收集器
    /// 
    /// 返回:
    ///     Self: 新的指標收集器
    pub fn with_defaults() -> Self {
        Self::new(MetricsConfig::default())
    }
    
    /// 初始化指標收集器
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn initialize(&mut self) -> MonitorResult<()> {
        if self.initialized {
            return Ok(());
        }
        
        // 初始化系統指標
        if self.config.enable_system_metrics {
            self.register_system_metrics().await?;
        }
        
        self.initialized = true;
        Ok(())
    }
    
    /// 註冊新指標
    /// 
    /// 參數:
    ///     metric: 指標實例
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn register(&self, metric: Metric) -> MonitorResult<()> {
        let mut metrics = self.metrics.write().await;
        metrics.insert(metric.name().to_string(), metric);
        Ok(())
    }
    
    /// 註冊系統指標
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    async fn register_system_metrics(&self) -> MonitorResult<()> {
        // 註冊CPU使用率指標
        let cpu_metric = Metric::new(
            "system_cpu_usage", 
            MetricType::Gauge, 
            "System CPU usage percentage"
        );
        self.register(cpu_metric).await?;
        
        // 註冊記憶體使用率指標
        let memory_metric = Metric::new(
            "system_memory_usage", 
            MetricType::Gauge, 
            "System memory usage percentage"
        );
        self.register(memory_metric).await?;
        
        // 註冊磁盤使用率指標
        let disk_metric = Metric::new(
            "system_disk_usage", 
            MetricType::Gauge, 
            "System disk usage percentage"
        );
        self.register(disk_metric).await?;
        
        // 註冊网絡流量指標
        let network_in_metric = Metric::new(
            "system_network_in", 
            MetricType::Counter, 
            "Network incoming traffic in bytes"
        );
        self.register(network_in_metric).await?;
        
        let network_out_metric = Metric::new(
            "system_network_out", 
            MetricType::Counter, 
            "Network outgoing traffic in bytes"
        );
        self.register(network_out_metric).await?;
        
        // 註冊系統負載指標
        let load_metric = Metric::new(
            "system_load_average", 
            MetricType::Gauge, 
            "System load average (1 minute)"
        );
        self.register(load_metric).await?;
        
        Ok(())
    }
    
    /// 獲取指標
    /// 
    /// 參數:
    ///     name: 指標名稱
    /// 
    /// 返回:
    ///     Option<Metric>: 指標實例或None
    pub async fn get_metric(&self, name: &str) -> Option<Metric> {
        let metrics = self.metrics.read().await;
        metrics.get(name).cloned()
    }
    
    /// 更新指標值
    /// 
    /// 參數:
    ///     name: 指標名稱
    ///     value: 新值
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn update_metric(&self, name: &str, value: f64) -> MonitorResult<()> {
        let mut metrics = self.metrics.write().await;
        
        if let Some(metric) = metrics.get_mut(name) {
            metric.set_value(value);
            Ok(())
        } else {
            Err(crate::monitor::MonitorError::MetricsError(
                format!("Metric '{}' not found", name)
            ))
        }
    }
    
    /// 增加指標值
    /// 
    /// 參數:
    ///     name: 指標名稱
    ///     amount: 增加量
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    pub async fn increment_metric(&self, name: &str, amount: f64) -> MonitorResult<()> {
        let mut metrics = self.metrics.write().await;
        
        if let Some(metric) = metrics.get_mut(name) {
            metric.increment(amount);
            Ok(())
        } else {
            Err(crate::monitor::MonitorError::MetricsError(
                format!("Metric '{}' not found", name)
            ))
        }
    }
    
    /// 獲取所有指標
    /// 
    /// 返回:
    ///     HashMap<String, Metric>: 所有指標的副本
    pub async fn get_all_metrics(&self) -> HashMap<String, Metric> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
    
    /// 開始計時器
    /// 
    /// 返回:
    ///     Instant: 開始時間點
    pub fn start_timer(&self) -> Instant {
        Instant::now()
    }
    
    /// 停止計時器並更新指標
    /// 
    /// 參數:
    ///     name: 指標名稱
    ///     start_time: 開始時間點
    /// 
    /// 返回:
    ///     MonitorResult<Duration>: 經過的時間
    pub async fn stop_timer(&self, name: &str, start_time: Instant) -> MonitorResult<Duration> {
        let duration = start_time.elapsed();
        self.update_metric(name, duration.as_secs_f64()).await?;
        Ok(duration)
    }
}

/// 指標提供者特徵
pub trait Metrics {
    /// 註冊組件指標
    /// 
    /// 參數:
    ///     collector: 指標收集器引用
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    fn register_metrics(&self, collector: &MetricsCollector) -> MonitorResult<()>;
    
    /// 更新組件指標
    /// 
    /// 參數:
    ///     collector: 指標收集器引用
    /// 
    /// 返回:
    ///     MonitorResult<()>: 成功或錯誤
    fn update_metrics(&self, collector: &MetricsCollector) -> MonitorResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_metric_creation() {
        // 測試創建新的指標
        let metric = Metric::new("test_counter", MetricType::Counter, "Test counter metric");
        
        assert_eq!(metric.name(), "test_counter");
        assert_eq!(metric.metric_type, MetricType::Counter);
        assert_eq!(metric.value(), 0.0);
    }
    
    #[test]
    fn test_metric_with_label() {
        // 測試標籤功能
        let metric = Metric::new("test_gauge", MetricType::Gauge, "Test gauge metric")
            .with_label("service", "backtest")
            .with_label("instance", "test-1");
        
        assert_eq!(metric.labels.len(), 2);
        assert_eq!(metric.labels.get("service"), Some(&"backtest".to_string()));
        assert_eq!(metric.labels.get("instance"), Some(&"test-1".to_string()));
    }
    
    #[test]
    fn test_metric_set_value() {
        // 測試設置值功能
        let mut metric = Metric::new("test_gauge", MetricType::Gauge, "Test gauge metric");
        
        metric.set_value(42.5);
        assert_eq!(metric.value(), 42.5);
        
        // 測試鏈式調用
        metric.set_value(50.0).set_value(60.0);
        assert_eq!(metric.value(), 60.0);
    }
    
    #[test]
    fn test_metric_increment() {
        // 測試增量功能
        let mut metric = Metric::new("test_counter", MetricType::Counter, "Test counter metric");
        
        metric.increment(5.0);
        assert_eq!(metric.value(), 5.0);
        
        metric.increment(10.0);
        assert_eq!(metric.value(), 15.0);
        
        // 測試鏈式調用
        metric.increment(2.5).increment(2.5);
        assert_eq!(metric.value(), 20.0);
    }
    
    #[test]
    fn test_metric_time_update() {
        // 測試時間更新功能
        let mut metric = Metric::new("test_timer", MetricType::Timer, "Test timer metric");
        
        // 等待一點時間
        thread::sleep(Duration::from_millis(50));
        assert!(metric.time_since_update().as_millis() >= 50);
        
        // 設置值應該更新時間戳
        metric.set_value(1.0);
        assert!(metric.time_since_update().as_millis() < 50);
    }
    
    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::with_defaults();
        assert!(!collector.initialized);
    }
    
    #[tokio::test]
    async fn test_metrics_collector_initialize() {
        let mut collector = MetricsCollector::with_defaults();
        
        // 初始化
        let result = collector.initialize().await;
        assert!(result.is_ok());
        assert!(collector.initialized);
        
        // 再次初始化不應該出錯
        let result = collector.initialize().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_metrics_collector_register() {
        let collector = MetricsCollector::with_defaults();
        
        // 註冊一個指標
        let metric = Metric::new("test_metric", MetricType::Counter, "Test metric");
        let result = collector.register(metric).await;
        assert!(result.is_ok());
        
        // 驗證指標已被註冊
        let retrieved = collector.get_metric("test_metric").await;
        assert!(retrieved.is_some());
        if let Some(m) = retrieved {
            assert_eq!(m.name(), "test_metric");
        }
    }
    
    #[tokio::test]
    async fn test_metrics_collector_update() {
        let collector = MetricsCollector::with_defaults();
        
        // 註冊一個指標
        let metric = Metric::new("update_test", MetricType::Gauge, "Update test metric");
        collector.register(metric).await.unwrap();
        
        // 更新指標
        let result = collector.update_metric("update_test", 42.0).await;
        assert!(result.is_ok());
        
        // 驗證更新成功
        let retrieved = collector.get_metric("update_test").await.unwrap();
        assert_eq!(retrieved.value(), 42.0);
        
        // 更新不存在的指標應該失敗
        let result = collector.update_metric("nonexistent", 10.0).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_metrics_collector_increment() {
        let collector = MetricsCollector::with_defaults();
        
        // 註冊一個指標
        let metric = Metric::new("increment_test", MetricType::Counter, "Increment test metric");
        collector.register(metric).await.unwrap();
        
        // 增加指標值
        collector.increment_metric("increment_test", 5.0).await.unwrap();
        
        // 驗證增加成功
        let retrieved = collector.get_metric("increment_test").await.unwrap();
        assert_eq!(retrieved.value(), 5.0);
        
        // 再次增加
        collector.increment_metric("increment_test", 7.0).await.unwrap();
        let retrieved = collector.get_metric("increment_test").await.unwrap();
        assert_eq!(retrieved.value(), 12.0);
        
        // 增加不存在的指標應該失敗
        let result = collector.increment_metric("nonexistent", 10.0).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_metrics_collector_timer() {
        let collector = MetricsCollector::with_defaults();
        
        // 註冊計時器指標
        let metric = Metric::new("timer_test", MetricType::Timer, "Timer test metric");
        collector.register(metric).await.unwrap();
        
        // 開始計時
        let start = collector.start_timer();
        
        // 等待一點時間
        thread::sleep(Duration::from_millis(50));
        
        // 停止計時
        let duration = collector.stop_timer("timer_test", start).await.unwrap();
        
        // 驗證時間已記錄
        assert!(duration.as_millis() >= 50);
        
        let retrieved = collector.get_metric("timer_test").await.unwrap();
        assert!(retrieved.value() >= 0.05); // 50毫秒 = 0.05秒
    }
    
    #[tokio::test]
    async fn test_metrics_collector_get_all() {
        let collector = MetricsCollector::new(MetricsConfig {
            enable_system_metrics: false, // 不初始化系統指標
            ..MetricsConfig::default()
        });
        
        // 註冊幾個指標
        collector.register(Metric::new("metric1", MetricType::Counter, "Metric 1")).await.unwrap();
        collector.register(Metric::new("metric2", MetricType::Gauge, "Metric 2")).await.unwrap();
        collector.register(Metric::new("metric3", MetricType::Timer, "Metric 3")).await.unwrap();
        
        // 獲取所有指標
        let all_metrics = collector.get_all_metrics().await;
        
        // 驗證結果
        assert_eq!(all_metrics.len(), 3);
        assert!(all_metrics.contains_key("metric1"));
        assert!(all_metrics.contains_key("metric2"));
        assert!(all_metrics.contains_key("metric3"));
    }
    
    #[tokio::test]
    async fn test_system_metrics_registration() {
        let mut collector = MetricsCollector::with_defaults();
        
        // 初始化（會註冊系統指標）
        collector.initialize().await.unwrap();
        
        // 驗證系統指標已註冊
        let all_metrics = collector.get_all_metrics().await;
        assert!(all_metrics.contains_key("system_cpu_usage"));
        assert!(all_metrics.contains_key("system_memory_usage"));
        assert!(all_metrics.contains_key("system_disk_usage"));
        assert!(all_metrics.contains_key("system_network_in"));
        assert!(all_metrics.contains_key("system_network_out"));
        assert!(all_metrics.contains_key("system_load_average"));
    }
} 