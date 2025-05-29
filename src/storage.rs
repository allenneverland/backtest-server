pub mod database;
pub mod migrations;
pub mod models;
pub mod repository;

// 只匯出必要的數據庫功能
pub use database::*;

// 匯出主要的模型
pub use models::{Exchange, FinancialReport, Instrument, MarketEvent};

// 匯出主要的倉儲接口和實現
pub use repository::{
    DbExecutor,
    // 具體倉儲實現
    ExchangeRepository,
    ExecutionLogRepository,
    ExecutionRunRepository,
    MarketDataRepository,
    Page,
    PageQuery,
    TimeRange,
};

// 匯出遷移功能
pub use migrations::*;

#[cfg(test)]
pub async fn create_test_pool() -> sqlx::PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/test".to_string());

    sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to create test database pool")
}
