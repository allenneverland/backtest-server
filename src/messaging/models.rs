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
pub use commands::{
    CreateBacktestCommand, CancelBacktestCommand, GetBacktestResultCommand,
};
pub use events::{
    BacktestStatus, BacktestCreatedEvent, BacktestStatusChangedEvent,
    BacktestProgressEvent, BacktestCompletedEvent, BacktestErrorEvent,
};
pub use responses::BacktestResultResponse;

// 重新導出策略相關類型
pub use commands::{
    CreateStrategyCommand, UpdateStrategyCommand, DeleteStrategyCommand, GetStrategyCommand,
};
pub use events::{
    StrategyCreatedEvent, StrategyUpdatedEvent, StrategyDeletedEvent,
};
pub use responses::{StrategyResponse, StrategyVersionsResponse};

// 重新導出數據相關類型
pub use commands::{
    GetMarketDataCommand, ImportMarketDataCommand, GetAssetsCommand,
};
pub use events::{
    DataImportStatus, DataImportEvent, DataUpdatedEvent,
};
pub use responses::{MarketDataResponse, AssetsResponse}; 