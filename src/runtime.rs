// 策略隔離運行時模組
//
// 本模組實現了策略的隔離運行環境，確保不同策略之間互不干擾，
// 且一個策略的故障不會影響其他策略或整個系統的運行。

pub mod error;
pub mod error_boundary;
pub mod fault_safety;
pub mod message_router;
pub mod resource;
pub mod sandbox;

pub use error::RuntimeError;
pub use error_boundary::{
    ErrorBoundary, ErrorBoundaryConfig, ErrorClassifier, ErrorHandlingPolicy, ErrorRecord,
    ErrorSeverity,
};
pub use fault_safety::{
    FaultDetectionMode, FaultSafetyManager, FaultThresholdConfig, HealthCheckResult, HealthManager,
    RecoveryStrategy,
};
pub use message_router::MessageRouter;
pub use resource::{
    quota::{OverQuotaBehavior, QuotaManager, QuotaPriority, QuotaWarningType, ResourceQuota},
    ResourceLimit, ResourceMonitor, ResourceType,
};
pub use sandbox::{
    CommunicationPolicy, Sandbox, SandboxBuilder, SandboxCommand, SandboxConfig, SandboxContext,
    SandboxEvent, SandboxState,
};
