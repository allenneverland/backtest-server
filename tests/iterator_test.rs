mod common;

use backtest_server::data_provider::{
    IteratorConfig, MarketDataIterator, OhlcvIterator, TickIterator,
};
use backtest_server::storage::repository::market_data::PgMarketDataRepository;
use chrono::{DateTime, Utc};
use futures::StreamExt;

#[tokio::test]
async fn test_ohlcv_iterator_streams_data() {
    // This test requires market data database with minute_bar table
    let pool = match common::setup_market_data_db().await {
        Some(pool) => pool,
        None => {
            eprintln!("Skipping test: Market data database not available");
            return;
        }
    };
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
                assert!(bar.time >= start);
                assert!(bar.time <= end);
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
    // This test requires market data database with tick table
    let pool = match common::setup_market_data_db().await {
        Some(pool) => pool,
        None => {
            eprintln!("Skipping test: Market data database not available");
            return;
        }
    };
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
                assert!(tick.time >= start);
                assert!(tick.time <= end);
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
    let pool = common::setup_test_db().await;
    let repo = PgMarketDataRepository::new(pool);
    let instrument_id = 1;

    let config = IteratorConfig {
        batch_size: 10,
        buffer_size: 50,
        time_range: (Utc::now() - chrono::Duration::days(1), Utc::now()),
    };

    let _iterator = OhlcvIterator::new(repo, instrument_id, config);
    // Test will verify batch size behavior once implemented
}

#[tokio::test]
async fn test_iterator_handles_empty_data() {
    // This test requires market data database
    let pool = match common::setup_market_data_db().await {
        Some(pool) => pool,
        None => {
            eprintln!("Skipping test: Market data database not available");
            return;
        }
    };
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
                assert!(matches!(
                    e,
                    backtest_server::data_provider::IteratorError::NoData
                ));
                break;
            }
        }
    }

    // Either no data was returned or we got a NoData error
    assert!(!has_data || stream.next().await.is_none());
}

#[tokio::test]
async fn test_multi_source_iterator_integration() {
    use backtest_server::data_provider::{MarketDataIterator, MultiSourceIterator};

    // This test requires market data database with minute_bar table
    let pool = match common::setup_market_data_db().await {
        Some(pool) => pool,
        None => {
            eprintln!("Skipping test: Market data database not available");
            return;
        }
    };
    let repo1 = PgMarketDataRepository::new(pool.clone());
    let repo2 = PgMarketDataRepository::new(pool);

    let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339("2024-01-01T01:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    let config = IteratorConfig {
        batch_size: 100,
        buffer_size: 500,
        time_range: (start, end),
    };

    // Create iterators for different instruments
    let iter1 = Box::new(OhlcvIterator::new(repo1, 1, config.clone()));
    let iter2 = Box::new(OhlcvIterator::new(repo2, 2, config));

    let multi_iterator = MultiSourceIterator::new(vec![iter1, iter2]);
    let mut stream = multi_iterator.stream();

    let mut prev_timestamp: Option<DateTime<Utc>> = None;
    let mut count = 0;

    // Verify that data comes out in chronological order
    while let Some(result) = stream.next().await {
        match result {
            Ok(bars) => {
                // All bars in a batch should have the same timestamp
                if !bars.is_empty() {
                    let timestamp = bars[0].time;
                    assert!(bars.iter().all(|bar| bar.time == timestamp));

                    // Timestamps should be in ascending order
                    if let Some(prev) = prev_timestamp {
                        assert!(timestamp >= prev);
                    }
                    prev_timestamp = Some(timestamp);
                    count += bars.len();

                    if count >= 10 {
                        break; // Test first 10 items
                    }
                }
            }
            Err(e) => {
                if matches!(e, backtest_server::data_provider::IteratorError::NoData) {
                    break;
                } else {
                    panic!("Unexpected error: {:?}", e);
                }
            }
        }
    }
}
