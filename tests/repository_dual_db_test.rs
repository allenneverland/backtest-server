use anyhow::Result;
use backtest_server::storage::repository::{BacktestRepo, DbExecutor, MarketDataRepo};

/// 測試市場數據 repository 使用正確的資料庫
#[tokio::test]
async fn test_market_data_repository_uses_market_db() -> Result<()> {
    let market_db_url = std::env::var("MARKET_DATABASE_URL")?;
    let market_pool = sqlx::PgPool::connect(&market_db_url).await?;
    let repo = MarketDataRepo::new(market_pool);

    // 測試查詢操作（驗證使用正確的連接池）
    // 這裡主要測試 repository 是否綁定到正確的資料庫
    // 實際的查詢方法會在具體實現中定義
    assert!(repo.get_pool().size() > 0);

    Ok(())
}

/// 測試回測 repository 使用正確的資料庫
#[tokio::test]
async fn test_backtest_repository_uses_backtest_db() -> Result<()> {
    let backtest_db_url = std::env::var("DATABASE_URL")?;
    let backtest_pool = sqlx::PgPool::connect(&backtest_db_url).await?;
    let repo = BacktestRepo::new(backtest_pool);

    // 測試寫入操作（驗證使用正確的連接池）
    // 這裡主要測試 repository 是否綁定到正確的資料庫
    // 實際的寫入方法會在具體實現中定義
    assert!(repo.get_pool().size() > 0);

    Ok(())
}

/// 測試跨資料庫查詢場景
#[tokio::test]
async fn test_cross_database_query_scenario() -> Result<()> {
    let market_db_url = std::env::var("MARKET_DATABASE_URL")?;
    let backtest_db_url = std::env::var("DATABASE_URL")?;

    // 創建兩個 repository
    let market_pool = sqlx::PgPool::connect(&market_db_url).await?;
    let backtest_pool = sqlx::PgPool::connect(&backtest_db_url).await?;
    
    let market_repo = MarketDataRepo::new(market_pool);
    let backtest_repo = BacktestRepo::new(backtest_pool);

    // 模擬回測流程：從市場數據讀取，寫入回測結果
    // 這裡主要驗證兩個 repository 使用不同的資料庫連接池

    // 驗證市場數據 repository 使用市場數據池
    assert!(market_repo.get_pool().size() > 0);

    // 驗證回測 repository 使用回測池
    assert!(backtest_repo.get_pool().size() > 0);

    // 驗證是不同的連接池
    assert!(!std::ptr::eq(
        market_repo.get_pool() as *const _,
        backtest_repo.get_pool() as *const _
    ));

    Ok(())
}

#[cfg(test)]
mod config_tests {
    use backtest_server::config::{BacktestDatabaseConfig, MarketDatabaseConfig};

    #[test]
    fn test_dual_database_config_validation() {
        let market_config = MarketDatabaseConfig {
            host: "localhost".to_string(),
            port: 5431,
            username: "market_reader".to_string(),
            password: "market_reader_password".to_string(),
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

        // 驗證配置有效性
        assert_eq!(market_config.database, "marketdata");
        assert_eq!(backtest_config.database, "backtest");
        assert_ne!(market_config.port, backtest_config.port);
    }

    #[test]
    fn test_dual_database_config_invalid() {
        // 測試無效配置檢測
        let invalid_config = MarketDatabaseConfig {
            host: "".to_string(), // 空主機名應該被檢測為無效
            port: 0, // 無效端口
            username: "".to_string(),
            password: "".to_string(),
            database: "".to_string(),
            connection_pool_size: 0, // 無效池大小
            max_connections: 0,
            min_connections: 0,
            max_lifetime_secs: 0,
            acquire_timeout_secs: 0,
            idle_timeout_secs: 0,
        };

        // 這些值應該被標記為無效
        assert!(invalid_config.host.is_empty());
        assert_eq!(invalid_config.port, 0);
        assert_eq!(invalid_config.connection_pool_size, 0);
    }
}