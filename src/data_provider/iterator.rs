//! Market data iterator for streaming historical data
//!
//! This module provides efficient iterators for processing large amounts of historical
//! market data without loading everything into memory at once.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
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

/// Trait for data types that have a timestamp
pub trait Timestamped {
    /// Get the timestamp of this data point
    fn timestamp(&self) -> DateTime<Utc>;
}

impl Timestamped for MinuteBar {
    fn timestamp(&self) -> DateTime<Utc> {
        self.time
    }
}

impl Timestamped for Tick {
    fn timestamp(&self) -> DateTime<Utc> {
        self.time
    }
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

/// Helper struct for priority queue ordering
struct TimestampedItem<T> {
    item: T,
    source_index: usize,
    timestamp: DateTime<Utc>,
}

impl<T> PartialEq for TimestampedItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && self.source_index == other.source_index
    }
}

impl<T> Eq for TimestampedItem<T> {}

impl<T> PartialOrd for TimestampedItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for TimestampedItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap behavior
        other
            .timestamp
            .cmp(&self.timestamp)
            .then_with(|| other.source_index.cmp(&self.source_index))
    }
}

/// Synchronizes multiple data iterators by timestamp
pub struct MultiSourceIterator<T: Send + Timestamped + Unpin + 'static> {
    iterators: Vec<Box<dyn MarketDataIterator<Item = T>>>,
}

impl<T: Send + Timestamped + Unpin + 'static> MultiSourceIterator<T> {
    /// Create a new multi-source iterator
    pub fn new(iterators: Vec<Box<dyn MarketDataIterator<Item = T>>>) -> Self {
        Self { iterators }
    }
}

#[async_trait]
impl<T: Send + Timestamped + Unpin + 'static> MarketDataIterator for MultiSourceIterator<T> {
    type Item = Vec<T>;

    fn stream(&self) -> Pin<Box<dyn Stream<Item = Result<Self::Item, IteratorError>> + Send>> {
        let streams: Vec<_> = self.iterators.iter().map(|iter| iter.stream()).collect();

        Box::pin(MultiSourceStream::new(streams))
    }
}

/// Type alias for a boxed stream of results
type BoxedResultStream<T> = Pin<Box<dyn Stream<Item = Result<T, IteratorError>> + Send>>;

/// Stream implementation for synchronized multi-source data
pub struct MultiSourceStream<T: Send + Timestamped> {
    /// Active streams with their indices
    streams: Vec<(usize, BoxedResultStream<T>)>,
    /// Priority queue of items ready to be yielded
    heap: BinaryHeap<TimestampedItem<T>>,
    /// Buffer for collecting items with the same timestamp
    buffer: Vec<T>,
    /// Current timestamp being processed
    current_timestamp: Option<DateTime<Utc>>,
}

impl<T: Send + Timestamped> MultiSourceStream<T> {
    fn new(streams: Vec<BoxedResultStream<T>>) -> Self {
        let indexed_streams = streams.into_iter().enumerate().collect();

        Self {
            streams: indexed_streams,
            heap: BinaryHeap::new(),
            buffer: Vec::new(),
            current_timestamp: None,
        }
    }
}

impl<T: Send + Timestamped + Unpin> Stream for MultiSourceStream<T> {
    type Item = Result<Vec<T>, IteratorError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First, try to fill the heap from all streams
        let mut i = 0;
        while i < self.streams.len() {
            let (source_index, stream) = &mut self.streams[i];
            let source_idx = *source_index;

            match stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(item))) => {
                    let timestamp = item.timestamp();
                    self.heap.push(TimestampedItem {
                        item,
                        source_index: source_idx,
                        timestamp,
                    });
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Ready(None) => {
                    // Stream exhausted, remove it
                    let _ = self.streams.swap_remove(i);
                    continue;
                }
                Poll::Pending => {
                    // Stream not ready, continue
                }
            }
            i += 1;
        }

        // Process items with the same timestamp
        if let Some(current_ts) = self.current_timestamp {
            // Continue collecting items with the same timestamp
            while let Some(peeked) = self.heap.peek() {
                if peeked.timestamp == current_ts {
                    if let Some(timestamped_item) = self.heap.pop() {
                        self.buffer.push(timestamped_item.item);
                    }
                } else {
                    break;
                }
            }

            // If we have items in the buffer, return them
            if !self.buffer.is_empty() {
                let items = std::mem::take(&mut self.buffer);
                self.current_timestamp = None;
                return Poll::Ready(Some(Ok(items)));
            }
        }

        // Try to get the next timestamp group
        if let Some(timestamped_item) = self.heap.pop() {
            self.current_timestamp = Some(timestamped_item.timestamp);
            self.buffer.push(timestamped_item.item);

            // Collect all items with the same timestamp
            while let Some(peeked) = self.heap.peek() {
                if peeked.timestamp == self.current_timestamp.unwrap() {
                    if let Some(item) = self.heap.pop() {
                        self.buffer.push(item.item);
                    }
                } else {
                    break;
                }
            }

            let items = std::mem::take(&mut self.buffer);
            self.current_timestamp = None;
            return Poll::Ready(Some(Ok(items)));
        }

        // Check if all streams are exhausted
        if self.streams.is_empty() && self.heap.is_empty() {
            Poll::Ready(None)
        } else {
            // We need more data
            cx.waker().wake_by_ref();
            Poll::Pending
        }
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
        use futures::{stream, StreamExt};
        use rust_decimal::Decimal;

        // Create mock data with different timestamps
        let data1 = vec![
            MinuteBar {
                instrument_id: 1,
                time: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: Decimal::from(100),
                high: Decimal::from(101),
                low: Decimal::from(99),
                close: Decimal::from(100),
                volume: Decimal::from(1000),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
            MinuteBar {
                instrument_id: 1,
                time: DateTime::parse_from_rfc3339("2024-01-01T00:02:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: Decimal::from(100),
                high: Decimal::from(102),
                low: Decimal::from(100),
                close: Decimal::from(101),
                volume: Decimal::from(1500),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
        ];

        let data2 = vec![
            MinuteBar {
                instrument_id: 2,
                time: DateTime::parse_from_rfc3339("2024-01-01T00:01:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: Decimal::from(200),
                high: Decimal::from(201),
                low: Decimal::from(199),
                close: Decimal::from(200),
                volume: Decimal::from(2000),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
            MinuteBar {
                instrument_id: 2,
                time: DateTime::parse_from_rfc3339("2024-01-01T00:02:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                open: Decimal::from(200),
                high: Decimal::from(202),
                low: Decimal::from(200),
                close: Decimal::from(201),
                volume: Decimal::from(2500),
                amount: None,
                open_interest: None,
                created_at: Utc::now(),
            },
        ];

        // Create mock iterators
        struct MockIterator {
            data: Vec<MinuteBar>,
        }

        #[async_trait]
        impl MarketDataIterator for MockIterator {
            type Item = MinuteBar;

            fn stream(
                &self,
            ) -> Pin<Box<dyn Stream<Item = Result<Self::Item, IteratorError>> + Send>> {
                let data = self.data.clone();
                Box::pin(stream::iter(data.into_iter().map(Ok)))
            }
        }

        let iter1 = Box::new(MockIterator { data: data1 });
        let iter2 = Box::new(MockIterator { data: data2 });

        let multi_iterator = MultiSourceIterator::new(vec![iter1, iter2]);
        let mut stream = multi_iterator.stream();

        // First batch should have the earliest timestamp (00:00:00)
        let batch1 = stream.next().await.unwrap().unwrap();
        assert_eq!(batch1.len(), 1);
        assert_eq!(
            batch1[0].time,
            DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
        );

        // Second batch should have the next timestamp (00:01:00)
        let batch2 = stream.next().await.unwrap().unwrap();
        assert_eq!(batch2.len(), 1);
        assert_eq!(
            batch2[0].time,
            DateTime::parse_from_rfc3339("2024-01-01T00:01:00Z")
                .unwrap()
                .with_timezone(&Utc)
        );

        // Third batch should have items from both sources with same timestamp (00:02:00)
        let batch3 = stream.next().await.unwrap().unwrap();
        assert_eq!(batch3.len(), 2);
        assert!(batch3.iter().all(|bar| bar.time
            == DateTime::parse_from_rfc3339("2024-01-01T00:02:00Z")
                .unwrap()
                .with_timezone(&Utc)));

        // No more data
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_multi_source_iterator_empty_streams() {
        use futures::StreamExt;

        // Test with empty streams
        let multi_iterator: MultiSourceIterator<MinuteBar> = MultiSourceIterator::new(vec![]);
        let mut stream = multi_iterator.stream();
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_multi_source_iterator_with_errors() {
        use futures::{stream, StreamExt};

        struct ErrorIterator;

        #[async_trait]
        impl MarketDataIterator for ErrorIterator {
            type Item = MinuteBar;

            fn stream(
                &self,
            ) -> Pin<Box<dyn Stream<Item = Result<Self::Item, IteratorError>> + Send>> {
                Box::pin(stream::once(async { Err(IteratorError::NoData) }))
            }
        }

        let iter1 = Box::new(ErrorIterator);
        let multi_iterator = MultiSourceIterator::new(vec![iter1]);
        let mut stream = multi_iterator.stream();

        // Should propagate the error
        let result = stream.next().await.unwrap();
        assert!(result.is_err());
    }
}
