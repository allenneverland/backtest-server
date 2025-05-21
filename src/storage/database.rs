use crate::config::{self, DatabaseConfig};
use anyhow::Result;
use sqlx::ConnectOptions;
use sqlx::{
    pool::PoolConnection,
    postgres::{PgConnectOptions, PgPool, PgPoolOptions},
    Postgres,
};
use tokio::sync::OnceCell;

/// 全局數據庫連接池
static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// 初始化數據庫連接池
pub async fn init_db_pool(config: &DatabaseConfig) -> Result<PgPool> {
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

/// 獲取數據庫連接池，如果不存在則初始化
pub async fn get_db_pool(force_init: bool) -> Result<&'static PgPool> {
    if force_init || DB_POOL.get().is_none() {
        // 獲取全局配置
        let app_config = config::get_config();
        let pool = init_db_pool(&app_config.database).await?;
        let pool = DB_POOL.get_or_init(|| async { pool }).await;
        return Ok(pool);
    }

    Ok(DB_POOL.get().unwrap())
}

/// 獲取單個數據庫連接
pub async fn get_connection() -> Result<PoolConnection<Postgres>> {
    let pool = get_db_pool(false).await?;
    let conn = pool.acquire().await?;
    Ok(conn)
}

/// 健康檢查
pub async fn health_check() -> Result<bool> {
    let pool = get_db_pool(false).await?;
    sqlx::query("SELECT 1").fetch_one(pool).await?;

    Ok(true)
}
