//! Market data iterator for streaming historical data
//!
//! This module provides efficient iterators for processing large amounts of historical
//! market data without loading everything into memory at once.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use std::pin::Pin;
use thiserror::Error;

use crate::domain_types::{Instrument, OhlcvSeries, TickSeries};
use crate::storage::models::market_data::{MinuteBar, Tick};
use crate::storage::repository::market_data::MarketDataRepository;

/// Errors that can occur during iteration
#[derive(Debug, Error)]
pub enum IteratorError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("No data available for the specified time range")]
    NoData,
    
    #[error("Invalid time range: start {start} is after end {end}")]
    InvalidTimeRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

/// Configuration for data iteration
#[derive(Debug, Clone)]
pub struct IteratorConfig {
    /// Number of records to fetch per batch
    pub batch_size: usize,
    /// Buffer size for prefetching
    pub buffer_size: usize,
    /// Time range for iteration
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
}

impl Default for IteratorConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            buffer_size: 5000,
            time_range: (DateTime::UNIX_EPOCH, Utc::now()),
        }
    }
}

/// Trait for market data iterators
#[async_trait]
pub trait MarketDataIterator: Send + Sync {
    /// The type of data this iterator yields
    type Item;
    
    /// Create a stream of market data
    fn stream(&self) -> Pin<Box<dyn Stream<Item = Result<Self::Item, IteratorError>> + Send>>;
}

/// Iterator for OHLCV data
pub struct OhlcvIterator<R: MarketDataRepository> {
    repository: R,
    instrument_id: i32,
    config: IteratorConfig,
}

impl<R: MarketDataRepository> OhlcvIterator<R> {
    /// Create a new OHLCV iterator
    pub fn new(
        repository: R,
        instrument_id: i32,
        config: IteratorConfig,
    ) -> Self {
        Self {
            repository,
            instrument_id,
            config,
        }
    }
}

/// Iterator for tick data
pub struct TickIterator<R: MarketDataRepository> {
    repository: R,
    instrument_id: i32,
    config: IteratorConfig,
}

impl<R: MarketDataRepository> TickIterator<R> {
    /// Create a new tick iterator
    pub fn new(
        repository: R,
        instrument_id: i32,
        config: IteratorConfig,
    ) -> Self {
        Self {
            repository,
            instrument_id,
            config,
        }
    }
}

/// Synchronizes multiple data iterators
pub struct MultiSourceIterator<T> {
    iterators: Vec<Box<dyn MarketDataIterator<Item = T>>>,
}

impl<T> MultiSourceIterator<T> {
    /// Create a new multi-source iterator
    pub fn new(iterators: Vec<Box<dyn MarketDataIterator<Item = T>>>) -> Self {
        Self { iterators }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iterator_config_default() {
        let config = IteratorConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.buffer_size, 5000);
    }

    #[tokio::test]
    async fn test_invalid_time_range() {
        let start = Utc::now();
        let end = start - chrono::Duration::days(1);
        
        let err = IteratorError::InvalidTimeRange { start, end };
        assert!(err.to_string().contains("Invalid time range"));
    }

    #[tokio::test]
    async fn test_ohlcv_iterator_creation() {
        // This test will fail until we implement the iterator
        // Test will be expanded once we implement the stream method
    }

    #[tokio::test]
    async fn test_tick_iterator_creation() {
        // This test will fail until we implement the iterator
        // Test will be expanded once we implement the stream method
    }

    #[tokio::test]
    async fn test_multi_source_iterator() {
        // Test for multi-source synchronization
        // Will be implemented with actual iterator logic
    }
}