use sqlx::PgPool;

pub async fn setup_test_db() -> PgPool {
    // Use test database URL from environment or default to match development config
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        // Check if we're in Docker environment (has .dockerenv file)
        if std::path::Path::new("/.dockerenv").exists() {
            // In Docker environment, use service name "db"
            "postgresql://backtest_server:backtest_server@db:5432/backtest_server".to_string()
        } else {
            // In CI or local testing, use localhost
            "postgresql://backtest_server:backtest_server@localhost:5432/backtest_server"
                .to_string()
        }
    });

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}
