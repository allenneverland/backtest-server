pub mod error;
pub mod traits;
pub mod ohlcv_validator;
pub mod tick_validator;
pub mod time_series_validator;
pub mod report;
pub mod registry;

// Re-export key components
pub use self::{
    error::{DataValidationError, ValidationResult},
    traits::DataValidator,
    ohlcv_validator::OHLCVValidator,
    tick_validator::TickValidator,
    time_series_validator::{TimeSeriesValidator, create_ohlcv_validator, create_tick_validator, HasTimestamp},
    report::{DataValidationReport, ValidationIssue, ValidationWarning, DataStatistics},
    registry::{
        ValidationRegistry,
        get_registry_arc,
        get_validator_for_type,
        register_custom_validator_async
    }
}; 