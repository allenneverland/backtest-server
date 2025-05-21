pub mod database;
pub mod migrations;
pub mod models;
pub mod repository;

// 只匯出必要的數據庫功能
pub use database::*;

// 匯出主要的模型
pub use models::instrument::{Crypto, Forex, Future, OptionContract, Stock};
pub use models::{Exchange, FinancialReport, Instrument, MarketEvent};

// 匯出主要的倉儲接口和實現
pub use repository::{
    DbExecutor,
    // 具體倉儲實現
    ExchangeRepository,
    MarketDataRepository,
    Page,
    PageQuery,
    StrategyRepository,
    StrategyVersionRepository,
    TimeRange,
};

// 匯出遷移功能
pub use migrations::*;
