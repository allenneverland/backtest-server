//! 回測系統模組
//! 
//! 此模組負責協調各個組件，執行策略回測，並收集和分析回測結果。
//! 包含回測引擎、回測任務管理、結果處理、進度監控等功能。

pub mod engine;
pub mod task;
pub mod results;
pub mod progress;
pub mod executor;
pub mod context;
pub mod metrics;
pub mod storage;

// 重新導出主要類型和結構
pub use engine::BacktestEngine;
pub use task::{BacktestTask, TaskStatus};
pub use results::{BacktestResult, StrategyPerformance};
pub use progress::BacktestProgress;
pub use executor::BacktestExecutor;
pub use context::BacktestContext;
pub use storage::ResultStorage;

// 重新導出回測配置
pub use engine::BacktestConfig;
