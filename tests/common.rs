use sqlx::PgPool;

pub async fn setup_test_db() -> PgPool {
    // Use backtest database URL from environment or default
    let database_url = std::env::var("BACKTEST_DATABASE_URL").unwrap_or_else(|_| {
        // Check if we're in Docker environment (has .dockerenv file)
        if std::path::Path::new("/.dockerenv").exists() {
            // In Docker environment, use service name "backtest-db"
            "postgresql://backtest_user:backtest_pass@backtest-db:5432/backtest".to_string()
        } else {
            // In CI or local testing, use localhost
            "postgresql://backtest_user:backtest_pass@localhost:5432/backtest"
                .to_string()
        }
    });

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}
