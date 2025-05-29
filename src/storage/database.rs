use crate::config::{self, BacktestDatabaseConfig, MarketDatabaseConfig};
use anyhow::Result;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::ConnectOptions;
use tokio::sync::OnceCell;

/// 全局市場數據資料庫連接池
static MARKET_DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// 全局回測資料庫連接池
static BACKTEST_DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// 資料庫池類型
pub type DatabasePool = PgPool;

/// 初始化市場數據資料庫連接池（唯讀）
pub async fn init_market_data_pool(config: &MarketDatabaseConfig) -> Result<PgPool> {
    let mut options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .username(&config.username)
        .password(&config.password)
        .database(&config.database);

    options = options.disable_statement_logging();

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .max_lifetime(config.max_lifetime())
        .acquire_timeout(config.acquire_timeout())
        .idle_timeout(config.idle_timeout())
        .connect_with(options)
        .await?;

    // 測試連接
    sqlx::query("SELECT 1").execute(&pool).await?;

    Ok(pool)
}

/// 初始化回測資料庫連接池
pub async fn init_backtest_pool(config: &BacktestDatabaseConfig) -> Result<PgPool> {
    let mut options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .username(&config.username)
        .password(&config.password)
        .database(&config.database);

    options = options.disable_statement_logging();

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .max_lifetime(config.max_lifetime())
        .acquire_timeout(config.acquire_timeout())
        .idle_timeout(config.idle_timeout())
        .connect_with(options)
        .await?;

    // 測試連接
    sqlx::query("SELECT 1").execute(&pool).await?;

    Ok(pool)
}

/// 獲取市場數據資料庫連接池
pub async fn get_market_data_pool(force_init: bool) -> Result<&'static PgPool> {
    if force_init || MARKET_DB_POOL.get().is_none() {
        let app_config = config::get_config();
        let pool = init_market_data_pool(&app_config.market_database).await?;
        let pool = MARKET_DB_POOL.get_or_init(|| async { pool }).await;
        return Ok(pool);
    }

    Ok(MARKET_DB_POOL.get().unwrap())
}

/// 獲取回測資料庫連接池
pub async fn get_backtest_pool(force_init: bool) -> Result<&'static PgPool> {
    if force_init || BACKTEST_DB_POOL.get().is_none() {
        let app_config = config::get_config();
        let pool = init_backtest_pool(&app_config.backtest_database).await?;
        let pool = BACKTEST_DB_POOL.get_or_init(|| async { pool }).await;
        return Ok(pool);
    }

    Ok(BACKTEST_DB_POOL.get().unwrap())
}

/// 市場數據資料庫包裝器（唯讀）
pub struct MarketDataDatabase {
    pool: PgPool,
}

impl MarketDataDatabase {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 執行唯讀查詢
    pub async fn execute_read_query(&self, query: &str) -> Result<()> {
        sqlx::query(query).execute(&self.pool).await?;
        Ok(())
    }

    /// 執行寫入查詢（應該失敗）
    pub async fn execute_write_query(&self, _query: &str) -> Result<()> {
        // 市場數據資料庫是唯讀的，寫入操作應該被拒絕
        anyhow::bail!("Write operations are not allowed on market data database")
    }

    /// 獲取連接池
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// 回測資料庫包裝器（讀寫）
pub struct BacktestDatabase {
    pool: PgPool,
}

impl BacktestDatabase {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 執行唯讀查詢
    pub async fn execute_read_query(&self, query: &str) -> Result<()> {
        sqlx::query(query).execute(&self.pool).await?;
        Ok(())
    }

    /// 執行寫入查詢
    pub async fn execute_write_query(&self, query: &str) -> Result<()> {
        sqlx::query(query).execute(&self.pool).await?;
        Ok(())
    }

    /// 獲取連接池
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// 資料庫池管理器
pub struct DatabasePoolManager {
    market_pool: Option<PgPool>,
    backtest_pool: Option<PgPool>,
}

impl DatabasePoolManager {
    /// 創建新的資料庫池管理器
    pub async fn new(
        market_config: MarketDatabaseConfig,
        backtest_config: BacktestDatabaseConfig,
    ) -> Result<Self> {
        let market_pool = init_market_data_pool(&market_config).await.ok();
        let backtest_pool = init_backtest_pool(&backtest_config).await.ok();

        Ok(Self {
            market_pool,
            backtest_pool,
        })
    }

    /// 獲取市場數據池
    pub fn market_data_pool(&self) -> Option<&PgPool> {
        self.market_pool.as_ref()
    }

    /// 獲取回測池
    pub fn backtest_pool(&self) -> Option<&PgPool> {
        self.backtest_pool.as_ref()
    }

    /// 健康檢查
    pub async fn health_check(&self) -> Result<HealthCheckResult> {
        let mut market_data_healthy = false;
        let mut backtest_healthy = false;

        if let Some(pool) = &self.market_pool {
            market_data_healthy = sqlx::query("SELECT 1").fetch_one(pool).await.is_ok();
        }

        if let Some(pool) = &self.backtest_pool {
            backtest_healthy = sqlx::query("SELECT 1").fetch_one(pool).await.is_ok();
        }

        Ok(HealthCheckResult {
            market_data_healthy,
            backtest_healthy,
        })
    }
}

/// 健康檢查結果
pub struct HealthCheckResult {
    pub market_data_healthy: bool,
    pub backtest_healthy: bool,
}
