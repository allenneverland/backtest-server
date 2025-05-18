// 事件處理系統模組
//
// 本模組提供事件驅動架構，實現系統中各個組件之間的消息傳遞和事件處理。
// 支持事件發佈/訂閱、事件佇列管理、事件路由以及過濾機制。

pub mod broadcast_optimizer;
pub mod event_debugger;
pub mod event_recorder;
pub mod event_replayer;
pub mod high_performance_bus;
pub mod high_performance_queue;
pub mod types;

// 重新導出核心類型
pub use broadcast_optimizer::{BroadcastConfig, BroadcastOptimizer, BroadcastStats};
pub use event_debugger::{
    Breakpoint, BreakpointCondition, BreakpointOperator, BreakpointTarget,
    DebugEvent, DebugEventType, DebuggerState, EventDebugger, EventDebuggerConfig,
    TraceLevel, Watchpoint, WatchType,
};
pub use event_recorder::{EventRecorder, RecordedEvent, RecordingSessionConfig, RecordingSessionState};
pub use event_replayer::{EventReplayer, ReplayConfig, ReplayProgress, ReplayState};
pub use high_performance_bus::{
    HighPerformanceBus as EventBus, // 重命名為 EventBus 以保持 API 兼容
    HighPerformanceBus,
    HighPerformanceBusConfig as EventBusConfig,
    HighPerformanceBusConfig,
    HighPerformanceSubscription as EventSubscription,
    HighPerformanceSubscription,
};
pub use high_performance_queue::{
    EventQueueStats, HighPerformanceQueue as EventQueue, HighPerformanceQueue,
};
pub use types::{Event, EventPriority, EventType, MarketPhase};
