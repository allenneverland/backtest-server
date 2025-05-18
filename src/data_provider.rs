pub mod lazy_loader;
pub mod smart_distribution;
pub mod resampler;
pub mod precalculator;
pub mod extensions;

// Re-export key public interfaces from lazy_loader
pub use lazy_loader::{
    LazyDataManager,
    LazyOHLCVLoader,
    LazyTickLoader,
    LazyLoader,
    LazyDataKey,
    LazyDataSource,
    LazyLoadStrategy,
    LazyLoadState,
    LazyTimeSeries
};

// Re-export key public interfaces from extensions
pub use extensions::{
    DataRequest,
    HistoryBulkRequest,
};

// Re-export key public interfaces from smart_distribution
pub use smart_distribution::{
    SmartDistributionSystem,
    DataDependencyGraph, // From smart_distribution::dependency_graph
    // Config structs from smart_distribution::types
    SmartDistributionConfig,
    DependencyGraphConfig,
    ResourceMonitorConfig,
    CacheConfig,
    RequestAnalyzerConfig,
    SmartSchedulerConfig,
    DistributionEngineConfig,
    PredictionConfig,
    // Core data structures from smart_distribution::types
    DataKey as SmartDataKey, // Alias to avoid conflict if other DataKey exists
    DataRequest as SmartDataRequest, // Alias for DataRequest
    DataResponse as SmartDataResponse, // Alias for DataResponse
    DataBatchKey,
    BatchRequest,
    AnalyzedRequest,
    PreloadCandidate,
    DataProcessTask,
    // Status structs from smart_distribution::types
    SmartDistributionStatus,
    ResourceStatus,
    CacheStatus,
    // CacheItemStats, // This was commented out in the types.rs, decide if needed
    SchedulerStatus,
    DependencyGraphStats,
    PredictionStats,
    // Error type
    DataDistributionError
};

// Re-export key public interfaces from resampler
pub use resampler::TimeSeriesResampler;

// Re-export key public interfaces from precalculator
pub use precalculator::{
    StockDataPrecalculator,
    BatchPrecalculator,
    IndicatorType,
    PrecalculatedIndicatorOutput
};

// Add other re-exports as modules are implemented 