use once_cell::sync::OnceCell;
use config::ConfigError;
use tracing::{warn, debug};
use crate::config::types::ApplicationConfig;
use crate::config::loader::{ConfigLoader, Environment};

// 全局配置實例
static CONFIG: OnceCell<ApplicationConfig> = OnceCell::new();

/// 獲取應用程序配置實例
pub fn get_config() -> &'static ApplicationConfig {
    CONFIG.get_or_init(|| {
        ApplicationConfig::load_from_env().expect("無法加載應用程序配置")
    })
}

/// 初始化配置（在應用程序啟動時調用）
pub fn init_config() -> Result<(), ConfigError> {
    let app_config = ApplicationConfig::load_from_env()?;
    
    // 嘗試初始化全局配置
    if CONFIG.set(app_config).is_err() {
        warn!("配置已經被初始化，跳過重複初始化");
    } else {
        debug!("配置初始化成功，環境：{:?}", Environment::from_env());
    }
    
    Ok(())
}

/// ApplicationConfig 加載方法實現
impl ApplicationConfig {
    /// 從環境變數指定的環境加載配置
    pub fn load_from_env() -> Result<Self, ConfigError> {
        let env = Environment::from_env();
        debug!("從環境加載配置: {:?}", env);
        Self::load(env)
    }
    
    /// 從指定環境加載配置
    pub fn load(env: Environment) -> Result<Self, ConfigError> {
        let config_source = ConfigLoader::load(env)?;
        
        // 使用 serde 反序列化配置
        let app_config: ApplicationConfig = config_source.try_deserialize()?;
        
        // 驗證配置（可選）
        if let Err(err) = app_config.validate() {
            warn!("配置驗證失敗: {}", err);
        } else {
            debug!("配置驗證通過");
        }
        
        Ok(app_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_environment_configuration() {
        // 設置測試環境
        env::set_var("BACKTEST_ENV", "development");
        
        // 測試加載配置
        let config = ApplicationConfig::load_from_env().expect("無法加載測試配置");
        
        // 驗證測試特定配置
        assert_eq!(config.server.port, 3001);
        assert_eq!(config.app.threads, 2);
        
        // 測試驗證
        assert!(config.validate().is_ok());
        
        // 清理環境變數
        env::remove_var("BACKTEST_ENV");
    }
} 