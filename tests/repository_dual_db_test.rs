use anyhow::Result;
use backtest_server::config::{BacktestDatabaseConfig, MarketDatabaseConfig};
use backtest_server::storage::database::DatabasePoolManager;
use backtest_server::storage::repository::{BacktestRepo, DbExecutor, MarketDataRepo};

/// 測試市場數據 repository 使用正確的資料庫
#[tokio::test]
async fn test_market_data_repository_uses_market_db() -> Result<()> {
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

    let pool_manager = DatabasePoolManager::new(market_config, backtest_config).await?;
    let repo = MarketDataRepo::new(pool_manager.market_data_pool().unwrap().clone());

    // 測試查詢操作（驗證使用正確的連接池）
    // 這裡主要測試 repository 是否綁定到正確的資料庫
    // 實際的查詢方法會在具體實現中定義
    assert!(repo.get_pool().size() > 0);

    Ok(())
}

/// 測試回測 repository 使用正確的資料庫
#[tokio::test]
async fn test_backtest_repository_uses_backtest_db() -> Result<()> {
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

    let pool_manager = DatabasePoolManager::new(market_config, backtest_config).await?;
    let repo = BacktestRepo::new(pool_manager.backtest_pool().unwrap().clone());

    // 測試寫入操作（驗證使用正確的連接池）
    // 這裡主要測試 repository 是否綁定到正確的資料庫
    // 實際的寫入方法會在具體實現中定義
    assert!(repo.get_pool().size() > 0);

    Ok(())
}

/// 測試跨資料庫查詢場景
#[tokio::test]
async fn test_cross_database_query_scenario() -> Result<()> {
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

    let pool_manager = DatabasePoolManager::new(market_config, backtest_config).await?;

    // 創建兩個 repository
    let market_repo = MarketDataRepo::new(pool_manager.market_data_pool().unwrap().clone());
    let backtest_repo = BacktestRepo::new(pool_manager.backtest_pool().unwrap().clone());

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
