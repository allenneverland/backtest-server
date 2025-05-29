/// 配置管理模組
///
/// 本模組負責加載、驗證和管理系統配置。
/// 支持從開發、測試和生產三種環境中加載不同的配置。
// 宣告子模組
pub mod loader;
pub mod manager;
pub mod types;
pub mod validation;

// 重新導出常用組件
pub use loader::{ConfigExt, ConfigLoader, Environment};
pub use manager::{get_config, init_config};
pub use types::*;
pub use validation::{validate_config, ValidationError, ValidationUtils, Validator};

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exports() {
        // 確保重要的導出可用
        let _ = super::Environment::Development;
        let _ = super::ValidationUtils::not_empty("test", "field");

        // 類型檢查
        fn _ensure_config_works(cfg: &super::ApplicationConfig) {
            let _ = &cfg.market_database;
            let _ = &cfg.backtest_database;
            let _ = &cfg.log;
            let _ = &cfg.server;
        }
    }
}
