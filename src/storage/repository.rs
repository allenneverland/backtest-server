use crate::utils::time_utils::{
    datetime_range_to_timestamp_range, timestamp_range_to_datetime_range,
};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

// 重新導出子模塊
pub mod aggregate;
pub mod exchange;
pub mod execution_log;
pub mod execution_run;
pub mod indicator;
pub mod instrument;
pub mod instrument_reference;
pub mod market_data;

// 重新導出常用類型
pub use aggregate::AggregateRepository;
pub use exchange::ExchangeRepository;
pub use execution_log::ExecutionLogRepository;
pub use execution_run::ExecutionRunRepository;
pub use indicator::IndicatorRepository;
pub use instrument::InstrumentRepository;
pub use instrument_reference::InstrumentReferenceRepository;
pub use market_data::MarketDataRepository;

// 重新導出具體實現
pub use market_data::PgMarketDataRepository;
/// 分頁結果
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> Page<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = (total as f64 / page_size as f64).ceil() as i64;
        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }

    pub fn empty(page: i64, page_size: i64) -> Self {
        Self::new(Vec::new(), 0, page, page_size)
    }
}

/// 查詢分頁參數
#[derive(Debug, Clone, Copy)]
pub struct PageQuery {
    pub page: i64,
    pub page_size: i64,
}

impl PageQuery {
    pub fn new(page: i64, page_size: i64) -> Self {
        Self { page, page_size }
    }
}

impl Default for PageQuery {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

/// 時間範圍查詢
#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// 從i64毫秒時間戳創建TimeRange
    ///
    /// 這個方法用於將計算核心層的時間戳轉換為資料庫層使用的TimeRange
    pub fn from_timestamps(start_ts: i64, end_ts: i64) -> Self {
        let (start, end) = timestamp_range_to_datetime_range(start_ts, end_ts);
        Self { start, end }
    }

    /// 將TimeRange轉換為i64毫秒時間戳元組
    ///
    /// 這個方法用於將TimeRange轉換為計算核心層使用的時間戳
    pub fn to_timestamps(&self) -> (i64, i64) {
        datetime_range_to_timestamp_range(&self.start, &self.end)
    }

    pub fn from_date_range(start: NaiveDate, end: NaiveDate) -> Self {
        use chrono::offset::TimeZone;
        Self {
            start: Utc.from_utc_datetime(&start.and_hms_opt(0, 0, 0).unwrap()),
            end: Utc.from_utc_datetime(&end.and_hms_opt(23, 59, 59).unwrap()),
        }
    }

    /// 返回一個表示無限時間範圍的 TimeRange
    pub fn all_time() -> Self {
        Self {
            start: DateTime::<Utc>::MIN_UTC,
            end: DateTime::<Utc>::MAX_UTC,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self {
            start: DateTime::<Utc>::MIN_UTC,
            end: DateTime::<Utc>::MAX_UTC,
        }
    }
}

/// 通用的數據庫操作特性
pub trait DbExecutor {
    fn get_pool(&self) -> &PgPool;
}

/// 市場數據 Repository 實現
pub struct MarketDataRepositoryImpl {
    pool: PgPool,
}

impl MarketDataRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl DbExecutor for MarketDataRepositoryImpl {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}
