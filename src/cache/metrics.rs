use metrics::{counter, histogram};
use std::time::Duration;

/// 監控指標命名空間
pub const METRIC_NAMESPACE: &str = "backtest_cache";

/// 監控指標類型
#[derive(Debug, Clone, Copy)]
pub enum MetricType {
    Hit {
        layer: &'static str,
    },
    Miss,
    Latency {
        operation: &'static str,
    },
    Error {
        operation: &'static str,
    },
    BatchOperation {
        operation: &'static str,
        count: usize,
    },
}

/// 快取監控指標記錄器
pub struct CacheMetrics;

impl CacheMetrics {
    /// 記錄快取指標
    ///
    /// # Arguments
    /// * `data_type` - 資料類型名稱 (如 "minute_bars", "ticks")
    /// * `metric_type` - 指標類型
    /// * `duration` - 可選的持續時間，用於延遲指標
    pub fn record(data_type: &'static str, metric_type: MetricType, duration: Option<Duration>) {
        match metric_type {
            MetricType::Hit { layer } => {
                counter!(
                    format!("{}.hit", METRIC_NAMESPACE),
                    "layer" => layer,
                    "type" => data_type
                )
                .increment(1);
            }
            MetricType::Miss => {
                counter!(
                    format!("{}.miss", METRIC_NAMESPACE),
                    "type" => data_type
                )
                .increment(1);
            }
            MetricType::Latency { operation } => {
                if let Some(dur) = duration {
                    histogram!(
                        format!("{}.latency_ns", METRIC_NAMESPACE),
                        "operation" => operation
                    )
                    .record(dur.as_nanos() as f64);
                }
            }
            MetricType::Error { operation } => {
                counter!(
                    format!("{}.error", METRIC_NAMESPACE),
                    "operation" => operation,
                    "type" => data_type
                )
                .increment(1);
            }
            MetricType::BatchOperation { operation, count } => {
                counter!(
                    format!("{}.batch_{}", METRIC_NAMESPACE, operation),
                    "type" => data_type
                )
                .increment(count as u64);

                if let Some(dur) = duration {
                    histogram!(
                        format!("{}.batch_latency_ns", METRIC_NAMESPACE),
                        "operation" => operation,
                        "type" => data_type
                    )
                    .record(dur.as_nanos() as f64);
                }
            }
        }
    }

    /// 記錄快取設定操作
    pub fn record_set(data_type: &'static str) {
        counter!(
            format!("{}.set", METRIC_NAMESPACE),
            "type" => data_type
        )
        .increment(1);
    }

    /// 記錄刪除操作
    pub fn record_delete(success: bool) {
        counter!(
            format!("{}.delete", METRIC_NAMESPACE),
            "result" => if success { "success" } else { "not_found" }
        )
        .increment(1);
    }

    /// 記錄快取清理操作
    pub fn record_cache_clear(cache_type: &'static str, count: u64) {
        counter!("cache_clear", "type" => cache_type).increment(1);
        counter!("cache_cleared_entries", "type" => cache_type).increment(count);
    }

    /// 記錄智能驅逐操作
    pub fn record_smart_eviction(evicted_size: u64) {
        counter!("cache_smart_eviction").increment(1);
        histogram!("cache_eviction_size").record(evicted_size as f64);
    }

    /// 記錄快取不一致清理
    pub fn record_inconsistent_cleanup() {
        counter!("cache_inconsistent_cleanup").increment(1);
    }

    /// 記錄統計信息請求
    pub fn record_stats_request() {
        counter!("cache_stats_requested").increment(1);
    }

    /// 記錄快取大小指標
    pub fn record_cache_size(cache_type: &'static str, size: usize, mapping_size: Option<usize>) {
        histogram!("cache_memory_entries", "type" => cache_type).record(size as f64);

        if let Some(mapping_sz) = mapping_size {
            histogram!("cache_mapping_size").record(mapping_sz as f64);
        }
    }

    /// 記錄批量設定操作指標
    pub fn record_batch_set(data_type: &'static str, count: usize, duration: Duration) {
        counter!("cache_batch_set", "type" => data_type).increment(count as u64);
        histogram!("cache_batch_set_duration", "type" => data_type).record(duration);
    }

    /// 記錄批量設定錯誤
    pub fn record_batch_set_error(data_type: &'static str) {
        counter!("cache_batch_set_error", "type" => data_type).increment(1);
    }

    /// 記錄 Pipeline 設定操作指標
    pub fn record_pipeline_set(data_type: &'static str, count: usize, duration: Duration) {
        counter!("cache_pipeline_set", "type" => data_type).increment(count as u64);
        histogram!("cache_pipeline_set_duration", "type" => data_type).record(duration);
    }

    /// 記錄 Pipeline 設定錯誤
    pub fn record_pipeline_set_error(data_type: &'static str) {
        counter!("cache_pipeline_set_error", "type" => data_type).increment(1);
    }
}
