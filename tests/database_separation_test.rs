use anyhow::Result;
use backtest_server::config::{BacktestDatabaseConfig, MarketDatabaseConfig};
use backtest_server::storage::database::{
    init_backtest_pool, init_market_data_pool, BacktestDatabase, MarketDataDatabase,
};

/// 測試雙資料庫配置初始化
#[tokio::test]
async fn test_dual_database_initialization() -> Result<()> {
    // 創建市場數據資料庫配置
    let market_data_config = MarketDatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "market_user".to_string(),
        password: "market_pass".to_string(),
        database: "marketdata".to_string(),
        connection_pool_size: 10,
        max_connections: 10,
        min_connections: 1,
        max_lifetime_secs: 3600,
        acquire_timeout_secs: 10,
        idle_timeout_secs: 600,
    };

    // 創建回測資料庫配置
    let backtest_config = BacktestDatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "backtest_user".to_string(),
        password: "backtest_pass".to_string(),
        database: "backtest".to_string(),
        connection_pool_size: 10,
        max_connections: 10,
        min_connections: 1,
        max_lifetime_secs: 3600,
        acquire_timeout_secs: 10,
        idle_timeout_secs: 600,
    };

    // 測試資料庫池創建（會失敗因為尚未實作）
    let market_pool_result = init_market_data_pool(&market_data_config).await;
    assert!(
        market_pool_result.is_ok(),
        "Market data pool initialization should succeed"
    );

    let backtest_pool_result = init_backtest_pool(&backtest_config).await;
    assert!(
        backtest_pool_result.is_ok(),
        "Backtest pool initialization should succeed"
    );

    Ok(())
}

/// 測試市場數據資料庫唯讀存取
#[tokio::test]
async fn test_market_data_read_only_access() -> Result<()> {
    let config = MarketDatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "market_user".to_string(),
        password: "market_pass".to_string(),
        database: "marketdata".to_string(),
        connection_pool_size: 10,
        max_connections: 10,
        min_connections: 1,
        max_lifetime_secs: 3600,
        acquire_timeout_secs: 10,
        idle_timeout_secs: 600,
    };

    let pool = init_market_data_pool(&config).await?;
    let db = MarketDataDatabase::new(pool);

    // 測試讀取操作應該成功
    let read_result = db.execute_read_query("SELECT 1").await;
    assert!(read_result.is_ok(), "Read query should succeed");

    // 測試寫入操作應該失敗
    let write_result = db
        .execute_write_query("INSERT INTO test_table VALUES (1)")
        .await;
    assert!(
        write_result.is_err(),
        "Write query should fail on read-only database"
    );

    Ok(())
}

/// 測試回測資料庫讀寫存取
#[tokio::test]
async fn test_backtest_read_write_access() -> Result<()> {
    let config = BacktestDatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "backtest_user".to_string(),
        password: "backtest_pass".to_string(),
        database: "backtest".to_string(),
        connection_pool_size: 10,
        max_connections: 10,
        min_connections: 1,
        max_lifetime_secs: 3600,
        acquire_timeout_secs: 10,
        idle_timeout_secs: 600,
    };

    let pool = init_backtest_pool(&config).await?;
    let db = BacktestDatabase::new(pool);

    // 測試讀取操作應該成功
    let read_result = db.execute_read_query("SELECT 1").await;
    assert!(read_result.is_ok(), "Read query should succeed");

    // 測試寫入操作應該成功
    let write_result = db
        .execute_write_query("CREATE TEMP TABLE test_table (id INT)")
        .await;
    assert!(
        write_result.is_ok(),
        "Write query should succeed on backtest database"
    );

    Ok(())
}

/// 測試資料庫池管理器
#[tokio::test]
async fn test_database_pool_manager() -> Result<()> {
    use backtest_server::storage::database::DatabasePoolManager;

    let market_config = MarketDatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "market_user".to_string(),
        password: "market_pass".to_string(),
        database: "marketdata".to_string(),
        connection_pool_size: 10,
        max_connections: 10,
        min_connections: 1,
        max_lifetime_secs: 3600,
        acquire_timeout_secs: 10,
        idle_timeout_secs: 600,
    };

    let backtest_config = BacktestDatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "backtest_user".to_string(),
        password: "backtest_pass".to_string(),
        database: "backtest".to_string(),
        connection_pool_size: 10,
        max_connections: 10,
        min_connections: 1,
        max_lifetime_secs: 3600,
        acquire_timeout_secs: 10,
        idle_timeout_secs: 600,
    };

    // 初始化池管理器
    let manager = DatabasePoolManager::new(market_config, backtest_config).await?;

    // 測試獲取市場數據池
    let market_pool = manager.market_data_pool();
    assert!(market_pool.is_some(), "Should return market data pool");

    // 測試獲取回測池
    let backtest_pool = manager.backtest_pool();
    assert!(backtest_pool.is_some(), "Should return backtest pool");

    // 測試健康檢查
    let health = manager.health_check().await?;
    assert!(
        health.market_data_healthy,
        "Market data database should be healthy"
    );
    assert!(
        health.backtest_healthy,
        "Backtest database should be healthy"
    );

    Ok(())
}

/// 測試配置驗證
#[cfg(test)]
mod config_tests {
    use backtest_server::config::validation::Validator;
    use backtest_server::config::{BacktestDatabaseConfig, MarketDatabaseConfig};

    #[test]
    fn test_dual_database_config_validation() {
        // 測試市場數據配置
        let market_config = MarketDatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "market_user".to_string(),
            password: "market_pass".to_string(),
            database: "marketdata".to_string(),
            connection_pool_size: 10,
            max_connections: 10,
            min_connections: 1,
            max_lifetime_secs: 3600,
            acquire_timeout_secs: 10,
            idle_timeout_secs: 600,
        };

        // 測試回測配置
        let backtest_config = BacktestDatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "backtest_user".to_string(),
            password: "backtest_pass".to_string(),
            database: "backtest".to_string(),
            connection_pool_size: 10,
            max_connections: 10,
            min_connections: 1,
            max_lifetime_secs: 3600,
            acquire_timeout_secs: 10,
            idle_timeout_secs: 600,
        };

        // 驗證配置應該通過
        assert!(
            market_config.validate().is_ok(),
            "Valid market database config should pass validation"
        );
        assert!(
            backtest_config.validate().is_ok(),
            "Valid backtest database config should pass validation"
        );
    }

    #[test]
    fn test_dual_database_config_invalid() {
        // 測試無效的市場數據配置
        let market_config = MarketDatabaseConfig {
            host: "".to_string(), // 空主機名
            port: 5432,
            username: "market_user".to_string(),
            password: "market_pass".to_string(),
            database: "marketdata".to_string(),
            connection_pool_size: 10,
            max_connections: 10,
            min_connections: 1,
            max_lifetime_secs: 3600,
            acquire_timeout_secs: 10,
            idle_timeout_secs: 600,
        };

        // 測試無效的回測配置
        let backtest_config = BacktestDatabaseConfig {
            host: "localhost".to_string(),
            port: 0, // 無效端口
            username: "backtest_user".to_string(),
            password: "backtest_pass".to_string(),
            database: "backtest".to_string(),
            connection_pool_size: 10,
            max_connections: 10,
            min_connections: 1,
            max_lifetime_secs: 3600,
            acquire_timeout_secs: 10,
            idle_timeout_secs: 600,
        };

        // 驗證配置應該失敗
        assert!(
            market_config.validate().is_err(),
            "Invalid market database config should fail validation"
        );
        assert!(
            backtest_config.validate().is_err(),
            "Invalid backtest database config should fail validation"
        );
    }
}
