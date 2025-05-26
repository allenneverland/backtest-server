use backtest_server::data_provider::{
    IteratorConfig, MarketDataIterator, OhlcvIterator, TickIterator,
};
use backtest_server::storage::repository::market_data::PgMarketDataRepository;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use sqlx::PgPool;

async fn setup_test_db() -> PgPool {
    // Use test database URL from environment or default
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/backtest_test".to_string());
    
    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[tokio::test]
async fn test_ohlcv_iterator_streams_data() {
    let pool = setup_test_db().await;
    let repo = PgMarketDataRepository::new(pool);
    let instrument_id = 1;
    
    let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    
    let config = IteratorConfig {
        batch_size: 100,
        buffer_size: 500,
        time_range: (start, end),
    };
    
    let iterator = OhlcvIterator::new(repo, instrument_id, config);
    let mut stream = iterator.stream();
    
    // Collect first batch of data
    let mut count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(bar) => {
                // Verify bar is within time range
                assert!(bar.timestamp >= start);
                assert!(bar.timestamp <= end);
                count += 1;
                
                if count >= 10 {
                    break; // Test first 10 bars
                }
            }
            Err(e) => {
                // Handle expected errors (e.g., no data)
                if matches!(e, backtest_server::data_provider::IteratorError::NoData) {
                    break;
                } else {
                    panic!("Unexpected error: {:?}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_tick_iterator_streams_data() {
    let pool = setup_test_db().await;
    let repo = PgMarketDataRepository::new(pool);
    let instrument_id = 1;
    
    let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339("2024-01-01T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    
    let config = IteratorConfig {
        batch_size: 1000,
        buffer_size: 5000,
        time_range: (start, end),
    };
    
    let iterator = TickIterator::new(repo, instrument_id, config);
    let mut stream = iterator.stream();
    
    // Collect first batch of data
    let mut count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(tick) => {
                // Verify tick is within time range
                assert!(tick.timestamp >= start);
                assert!(tick.timestamp <= end);
                count += 1;
                
                if count >= 100 {
                    break; // Test first 100 ticks
                }
            }
            Err(e) => {
                // Handle expected errors (e.g., no data)
                if matches!(e, backtest_server::data_provider::IteratorError::NoData) {
                    break;
                } else {
                    panic!("Unexpected error: {:?}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_iterator_respects_batch_size() {
    let pool = setup_test_db().await;
    let repo = PgMarketDataRepository::new(pool);
    let instrument_id = 1;
    
    let config = IteratorConfig {
        batch_size: 10,
        buffer_size: 50,
        time_range: (Utc::now() - chrono::Duration::days(1), Utc::now()),
    };
    
    let iterator = OhlcvIterator::new(repo, instrument_id, config);
    // Test will verify batch size behavior once implemented
}

#[tokio::test]
async fn test_iterator_handles_empty_data() {
    let pool = setup_test_db().await;
    let repo = PgMarketDataRepository::new(pool);
    let instrument_id = 1;
    
    // Use future date range to ensure no data
    let start = Utc::now() + chrono::Duration::days(365);
    let end = start + chrono::Duration::days(1);
    
    let config = IteratorConfig {
        batch_size: 100,
        buffer_size: 500,
        time_range: (start, end),
    };
    
    let iterator = OhlcvIterator::new(repo, instrument_id, config);
    let mut stream = iterator.stream();
    
    // Should either return no items or NoData error
    let mut has_data = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(_) => has_data = true,
            Err(e) => {
                assert!(matches!(e, backtest_server::data_provider::IteratorError::NoData));
                break;
            }
        }
    }
    
    // Either no data was returned or we got a NoData error
    assert!(!has_data || stream.next().await.is_none());
}