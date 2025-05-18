use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

// 重新導出子模塊
pub mod market_data;
pub mod strategy;
pub mod instrument;
pub mod exchange;
pub mod portfolio;
pub mod strategy_version;
pub mod backtest;
pub mod aggregate;
pub mod indicator;

// 重新導出常用類型
pub use strategy::StrategyRepository; 
pub use exchange::ExchangeRepository;
pub use instrument::InstrumentRepository;
pub use market_data::MarketDataRepository;
pub use portfolio::PortfolioRepository;
pub use strategy_version::StrategyVersionRepository;
pub use backtest::BacktestRepository;
pub use aggregate::AggregateRepository;
pub use indicator::IndicatorRepository;

/// 分頁結果
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = (total as f64 / page_size as f64).ceil() as i64;
        Self {
            items,
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