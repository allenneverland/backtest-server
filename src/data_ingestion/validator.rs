//! 資料驗證器模組
//!
//! 提供數據驗證功能，確保導入的市場數據符合質量標準。
//!
//! # 主要功能
//!
//! - **數據完整性檢查**：驗證必要欄位是否存在
//! - **數值範圍驗證**：確保價格、成交量等在合理範圍內
//! - **時間序列驗證**：檢查時間順序、重複和間隔
//! - **業務邏輯驗證**：如 OHLCV 的價格一致性檢查
//!
//! # 使用範例
//!
//! ```rust,ignore
//! use backtest_server::data_ingestion::validator::{
//!     OhlcvValidator, ValidationConfig, Validator
//! };
//!
//! // 創建 OHLCV 驗證器
//! let validator = OhlcvValidator::new()
//!     .with_price_range(0.0, 1_000_000.0)
//!     .with_max_gap(3600); // 1 小時
//!
//! // 驗證單筆記錄
//! let result = validator.validate_record(&ohlcv_record);
//!
//! // 批次驗證
//! let batch_result = validator.validate_batch(&records);
//! ```

pub mod error;
pub mod ohlcv_validator;
pub mod registry;
pub mod report;
pub mod tick_validator;
pub mod time_series_validator;
pub mod traits;

// 重新導出常用類型
pub use error::{ValidationError, ValidationErrors, ValidationResult};
pub use ohlcv_validator::{OhlcvRecord, OhlcvValidator};
pub use registry::{
    create_ohlcv_registry, create_tick_registry, ValidatorChain, ValidatorRegistry, ValidatorType,
};
pub use report::{ReportFormatter, ValidationReport};
pub use tick_validator::{TickRecord, TickValidator};
pub use time_series_validator::{TimeSeriesRecord, TimeSeriesStats, TimeSeriesValidator};
pub use traits::{ComposableValidator, ValidationConfig, ValidationRule, Validator};

/// 創建預設的 OHLCV 驗證器鏈
pub fn create_default_ohlcv_chain() -> ValidatorChain<OhlcvRecord> {
    let registry = create_ohlcv_registry();

    ValidatorChain::new()
        .add_from_registry(&registry, ValidatorType::Ohlcv, ValidationConfig::default())
        .expect("OHLCV validator should be registered")
        .add_from_registry(
            &registry,
            ValidatorType::TimeSeries,
            ValidationConfig::default(),
        )
        .expect("TimeSeries validator should be registered")
}

/// 創建預設的 Tick 驗證器鏈
pub fn create_default_tick_chain() -> ValidatorChain<TickRecord> {
    let registry = create_tick_registry();

    ValidatorChain::new()
        .add_from_registry(&registry, ValidatorType::Tick, ValidationConfig::default())
        .expect("Tick validator should be registered")
        .add_from_registry(
            &registry,
            ValidatorType::TimeSeries,
            ValidationConfig::default(),
        )
        .expect("TimeSeries validator should be registered")
}

/// 執行完整的數據驗證流程
pub async fn validate_data<T>(
    validator_chain: &ValidatorChain<T>,
    data: &[T],
    validator_name: &str,
) -> Result<ValidationReport, ValidationErrors>
where
    T: Send + Sync + 'static,
{
    let mut report = ValidationReport::new(validator_name).with_detailed_errors();

    match validator_chain.validate_batch(data) {
        Ok(()) => {
            report.total_records = data.len();
            report.valid_records = data.len();
            report.invalid_records = 0;
            Ok(report.finish())
        }
        Err(errors_by_type) => {
            let mut all_errors = ValidationErrors::new();

            for (_validator_type, errors) in errors_by_type {
                all_errors.merge(errors);
            }

            report.total_records = data.len();
            report.invalid_records = all_errors.error_count();
            report.valid_records = data.len().saturating_sub(all_errors.error_count());

            for (line, error) in all_errors.iter() {
                report.add_error(*line, error.clone());
            }

            Err(all_errors)
        }
    }
}
