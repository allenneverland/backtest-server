//! Market data iterator for streaming historical data
//!
//! This module provides efficient iterators for processing large amounts of historical
//! market data without loading everything into memory at once.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::storage::models::market_data::{MinuteBar, Tick};
use crate::storage::repository::{market_data::MarketDataRepository, TimeRange};

/// Errors that can occur during iteration
#[derive(Debug, Error)]
pub enum IteratorError {
    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),

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
    repository: std::sync::Arc<R>,
    instrument_id: i32,
    config: IteratorConfig,
}

impl<R: MarketDataRepository + 'static> OhlcvIterator<R> {
    /// Create a new OHLCV iterator
    pub fn new(repository: R, instrument_id: i32, config: IteratorConfig) -> Self {
        Self {
            repository: std::sync::Arc::new(repository),
            instrument_id,
            config,
        }
    }

    /// Create a stream that yields OHLCV bars
    pub fn into_stream(self) -> OhlcvStream<R> {
        let (start, end) = self.config.time_range;

        // Validate time range
        if start > end {
            let (tx, rx) = mpsc::channel(1);
            // Send error immediately
            tokio::spawn(async move {
                let _ = tx
                    .send(Err(IteratorError::InvalidTimeRange { start, end }))
                    .await;
            });
            return OhlcvStream {
                receiver: rx,
                _phantom: std::marker::PhantomData,
            };
        }

        let (tx, rx) = mpsc::channel(self.config.buffer_size);
        let repository = std::sync::Arc::clone(&self.repository);
        let instrument_id = self.instrument_id;
        let batch_size = self.config.batch_size;

        // Spawn background task to fetch data
        tokio::spawn(async move {
            let mut current_time = start;

            while current_time < end {
                let time_range = TimeRange {
                    start: current_time,
                    end,
                };

                match repository
                    .get_minute_bars(instrument_id, time_range, Some(batch_size as i64))
                    .await
                {
                    Ok(bars) => {
                        if bars.is_empty() {
                            // No more data
                            break;
                        }

                        let last_time = bars.last().unwrap().time;

                        for bar in bars {
                            if tx.send(Ok(bar)).await.is_err() {
                                // Receiver dropped
                                return;
                            }
                        }

                        // Move to next batch
                        current_time = last_time + chrono::Duration::nanoseconds(1);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(IteratorError::Database(e))).await;
                        break;
                    }
                }
            }

            // Check if we got any data at all
            if current_time == start {
                let _ = tx.send(Err(IteratorError::NoData)).await;
            }
        });

        OhlcvStream {
            receiver: rx,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Stream implementation for OHLCV data
pub struct OhlcvStream<R> {
    receiver: mpsc::Receiver<Result<MinuteBar, IteratorError>>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R> Stream for OhlcvStream<R> {
    type Item = Result<MinuteBar, IteratorError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: we're not moving out of the pinned field
        let receiver = unsafe { &mut self.as_mut().get_unchecked_mut().receiver };
        Pin::new(receiver).poll_recv(cx)
    }
}

#[async_trait]
impl<R: MarketDataRepository + 'static> MarketDataIterator for OhlcvIterator<R> {
    type Item = MinuteBar;

    fn stream(&self) -> Pin<Box<dyn Stream<Item = Result<Self::Item, IteratorError>> + Send>> {
        // Clone self to create a new stream
        let cloned_self = Self {
            repository: std::sync::Arc::clone(&self.repository),
            instrument_id: self.instrument_id,
            config: self.config.clone(),
        };

        Box::pin(cloned_self.into_stream())
    }
}

/// Iterator for tick data
pub struct TickIterator<R: MarketDataRepository> {
    repository: std::sync::Arc<R>,
    instrument_id: i32,
    config: IteratorConfig,
}

impl<R: MarketDataRepository + 'static> TickIterator<R> {
    /// Create a new tick iterator
    pub fn new(repository: R, instrument_id: i32, config: IteratorConfig) -> Self {
        Self {
            repository: std::sync::Arc::new(repository),
            instrument_id,
            config,
        }
    }

    /// Create a stream that yields tick data
    pub fn into_stream(self) -> TickStream<R> {
        let (start, end) = self.config.time_range;

        // Validate time range
        if start > end {
            let (tx, rx) = mpsc::channel(1);
            // Send error immediately
            tokio::spawn(async move {
                let _ = tx
                    .send(Err(IteratorError::InvalidTimeRange { start, end }))
                    .await;
            });
            return TickStream {
                receiver: rx,
                _phantom: std::marker::PhantomData,
            };
        }

        let (tx, rx) = mpsc::channel(self.config.buffer_size);
        let repository = std::sync::Arc::clone(&self.repository);
        let instrument_id = self.instrument_id;
        let batch_size = self.config.batch_size;

        // Spawn background task to fetch data
        tokio::spawn(async move {
            let mut current_time = start;

            while current_time < end {
                let time_range = TimeRange {
                    start: current_time,
                    end,
                };

                match repository
                    .get_ticks(instrument_id, time_range, Some(batch_size as i64))
                    .await
                {
                    Ok(ticks) => {
                        if ticks.is_empty() {
                            // No more data
                            break;
                        }

                        let last_time = ticks.last().unwrap().time;

                        for tick in ticks {
                            if tx.send(Ok(tick)).await.is_err() {
                                // Receiver dropped
                                return;
                            }
                        }

                        // Move to next batch
                        current_time = last_time + chrono::Duration::nanoseconds(1);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(IteratorError::Database(e))).await;
                        break;
                    }
                }
            }

            // Check if we got any data at all
            if current_time == start {
                let _ = tx.send(Err(IteratorError::NoData)).await;
            }
        });

        TickStream {
            receiver: rx,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Stream implementation for tick data
pub struct TickStream<R> {
    receiver: mpsc::Receiver<Result<Tick, IteratorError>>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R> Stream for TickStream<R> {
    type Item = Result<Tick, IteratorError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: we're not moving out of the pinned field
        let receiver = unsafe { &mut self.as_mut().get_unchecked_mut().receiver };
        Pin::new(receiver).poll_recv(cx)
    }
}

#[async_trait]
impl<R: MarketDataRepository + 'static> MarketDataIterator for TickIterator<R> {
    type Item = Tick;

    fn stream(&self) -> Pin<Box<dyn Stream<Item = Result<Self::Item, IteratorError>> + Send>> {
        // Clone self to create a new stream
        let cloned_self = Self {
            repository: std::sync::Arc::clone(&self.repository),
            instrument_id: self.instrument_id,
            config: self.config.clone(),
        };

        Box::pin(cloned_self.into_stream())
    }
}

/// Synchronizes multiple data iterators
pub struct MultiSourceIterator<T: Send + 'static> {
    _iterators: Vec<Box<dyn MarketDataIterator<Item = T>>>,
}

impl<T: Send + 'static> MultiSourceIterator<T> {
    /// Create a new multi-source iterator
    pub fn new(iterators: Vec<Box<dyn MarketDataIterator<Item = T>>>) -> Self {
        Self {
            _iterators: iterators,
        }
    }
}

#[async_trait]
impl<T: Send + 'static> MarketDataIterator for MultiSourceIterator<T> {
    type Item = Vec<T>;

    fn stream(&self) -> Pin<Box<dyn Stream<Item = Result<Self::Item, IteratorError>> + Send>> {
        // For now, return a simple implementation
        // In a real implementation, this would synchronize by timestamp from all iterators
        Box::pin(futures::stream::once(async {
            Err(IteratorError::NoData) // Placeholder implementation
        }))
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
