pub mod database;
pub mod repository;
pub mod models;
pub mod migrations;

// 只匯出必要的數據庫功能
pub use database::*;

// 匯出主要的模型
pub use models::{
    Exchange,
    Instrument,
    MarketEvent,
    FinancialReport,
};
pub use models::instrument::{Stock, Future, OptionContract, Forex, Crypto};

// 匯出主要的倉儲接口和實現
pub use repository::{
    DbExecutor,
    Page,
    PageQuery,
    TimeRange,
    // 具體倉儲實現
    ExchangeRepository,
    MarketDataRepository,
    StrategyVersionRepository,
    StrategyRepository
};


// 匯出遷移功能
pub use migrations::*;
