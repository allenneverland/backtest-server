// 消息模型模組
// 定義系統中使用的消息模型，包括命令、事件和回應

// 導出子模組
pub mod commands;
pub mod events;
pub mod responses;

// 重新導出常用類型
pub use commands::Command;
pub use events::Event;
pub use responses::{ErrorCode, ErrorResponse, SuccessResponse};

// 重新導出回測相關類型
pub use commands::{CancelBacktestCommand, CreateBacktestCommand, GetBacktestResultCommand};
pub use events::{
    BacktestCompletedEvent, BacktestCreatedEvent, BacktestErrorEvent, BacktestProgressEvent,
    BacktestStatus, BacktestStatusChangedEvent,
};
pub use responses::BacktestResultResponse;

// 重新導出策略相關類型
pub use commands::{
    CreateStrategyCommand, DeleteStrategyCommand, GetStrategyCommand, UpdateStrategyCommand,
};
pub use events::{StrategyCreatedEvent, StrategyDeletedEvent, StrategyUpdatedEvent};
pub use responses::{StrategyResponse, StrategyVersionsResponse};

// 重新導出數據相關類型
pub use commands::{GetAssetsCommand, GetMarketDataCommand, ImportMarketDataCommand};
pub use events::{DataImportEvent, DataImportStatus, DataUpdatedEvent};
pub use responses::{AssetsResponse, MarketDataResponse};
