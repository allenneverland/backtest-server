//! 集中化的 Redis 測試配置
//!
//! 提供一致的測試環境配置，支援不同部署環境（本地開發、Docker 容器）

use crate::config::types::RedisConfig;
use crate::redis::pool::{ConnectionPool, RedisPool, RedisPoolError};
use std::sync::Arc;

/// Redis 測試配置建構器
pub struct RedisTestConfig;

impl RedisTestConfig {
    /// 獲取測試用 Redis URL
    ///
    /// 優先級：
    /// 1. REDIS_TEST_URL 環境變數
    /// 2. 檢測 Docker 環境使用 redis:6379
    /// 3. 預設 localhost:6379
    pub fn get_test_url() -> String {
        // 首先檢查環境變數
        if let Ok(url) = std::env::var("REDIS_TEST_URL") {
            return url;
        }

        // 檢測是否在 Docker 環境中
        if Self::is_docker_environment() {
            "redis://redis:6379".to_string()
        } else {
            "redis://localhost:6379".to_string()
        }
    }

    /// 檢測是否在 Docker 環境中執行
    fn is_docker_environment() -> bool {
        // 檢查常見的 Docker 環境指標
        std::env::var("DOCKER_CONTAINER").is_ok()
            || std::path::Path::new("/.dockerenv").exists()
            || std::env::var("HOSTNAME")
                .map(|h| h.starts_with("backtest-server"))
                .unwrap_or(false)
    }

    /// 建立標準測試 Redis 配置
    pub fn create_test_config() -> RedisConfig {
        RedisConfig {
            url: Self::get_test_url(),
            pool_size: 3,
            connection_timeout_secs: 5,
            read_timeout_secs: 5,
            write_timeout_secs: 5,
            reconnect_attempts: 1,
            reconnect_delay_secs: 1,
        }
    }

    /// 建立測試用 Redis 連接池
    pub async fn create_test_pool() -> Result<Arc<ConnectionPool>, RedisPoolError> {
        let config = Self::create_test_config();
        let pool = ConnectionPool::new(config).await?;
        Ok(Arc::new(pool))
    }

    /// 檢查 Redis 是否可用於測試
    ///
    /// 如果 REDIS_TEST_AVAILABLE 環境變數未設置為 "true"，
    /// 或者無法連接到 Redis，返回 false
    pub async fn is_redis_available() -> bool {
        // 檢查環境變數
        let redis_available =
            std::env::var("REDIS_TEST_AVAILABLE").unwrap_or_else(|_| "false".to_string());

        if redis_available != "true" {
            return false;
        }

        // 嘗試建立連接
        match Self::create_test_pool().await {
            Ok(pool) => pool.check_health().await,
            Err(_) => false,
        }
    }

    /// 跳過 Redis 測試的輔助巨集
    ///
    /// 在測試開始時呼叫，如果 Redis 不可用則跳過測試
    pub async fn skip_if_redis_unavailable(test_name: &str) -> Option<()> {
        if !Self::is_redis_available().await {
            println!("跳過 Redis 測試 '{}' - Redis 環境不可用", test_name);
            return None;
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_test_url_with_env_var() {
        // 設置環境變數
        std::env::set_var("REDIS_TEST_URL", "redis://custom:1234");

        let url = RedisTestConfig::get_test_url();
        assert_eq!(url, "redis://custom:1234");

        // 清理
        std::env::remove_var("REDIS_TEST_URL");
    }

    #[test]
    fn test_create_test_config() {
        let config = RedisTestConfig::create_test_config();
        assert!(config.url.starts_with("redis://"));
        assert_eq!(config.pool_size, 3);
        assert_eq!(config.connection_timeout_secs, 5);
    }

    #[tokio::test]
    async fn test_redis_availability_check() {
        // 此測試不需要實際 Redis 連接，只測試邏輯
        std::env::set_var("REDIS_TEST_AVAILABLE", "false");
        assert!(!RedisTestConfig::is_redis_available().await);

        std::env::remove_var("REDIS_TEST_AVAILABLE");
    }
}
