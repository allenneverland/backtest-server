use sqlx::PgPool;

pub async fn setup_test_db() -> PgPool {
    // Use backtest database URL from environment or default
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        // Check if we're in Docker environment (has .dockerenv file)
        if std::path::Path::new("/.dockerenv").exists() {
            // In Docker environment, use service name "backtest-db"
            "postgresql://backtest_user:backtest_pass@backtest-db:5432/backtest".to_string()
        } else {
            // In CI or local testing, use localhost
            "postgresql://backtest_user:backtest_pass@localhost:5432/backtest".to_string()
        }
    });

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Setup market data database connection for tests
pub async fn setup_market_data_db() -> Option<PgPool> {
    // Check if market data database URL is available
    let database_url = std::env::var("MARKET_DATABASE_URL").unwrap_or_else(|_| {
        // Check if we're in Docker environment (has .dockerenv file)
        if std::path::Path::new("/.dockerenv").exists() {
            // In Docker environment, use service name "marketdata-center-market-db-1"
            "postgresql://market_reader:market_reader_password@marketdata-center-market-db-1:5432/marketdata".to_string()
        } else {
            // In CI or local testing, use localhost with correct port
            // Try both connection strings for compatibility
            if std::env::var("DATABASE_URL").is_ok() {
                // In CI environment
                "postgresql://market_reader:market_reader_password@localhost:5431/marketdata".to_string()
            } else {
                // Local development with default credentials
                "postgresql://market_reader:market_reader_password@localhost:5431/marketdata".to_string()
            }
        }
    });

    // Try to connect to market data database
    match PgPool::connect(&database_url).await {
        Ok(pool) => Some(pool),
        Err(e) => {
            eprintln!("Failed to connect to market database: {}", e);
            None
        }
    }
}
