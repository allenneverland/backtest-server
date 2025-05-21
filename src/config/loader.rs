use config::{Config, ConfigError, Environment as ConfigEnvironment, File};
use serde::Deserialize;
use std::env;
use std::path::Path;

/// 環境類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    /// 從環境變數取得當前環境設定
    pub fn from_env() -> Self {
        match env::var("BACKTEST_ENV")
            .unwrap_or_else(|_| "development".into())
            .to_lowercase()
            .as_str()
        {
            "production" => Environment::Production,
            _ => Environment::Development,
        }
    }

    /// 轉換為配置文件名
    pub fn as_filename(&self) -> &'static str {
        match self {
            Environment::Development => "development.toml",
            Environment::Production => "production.toml",
        }
    }
}

/// 配置加載器，負責根據環境加載適當的配置
pub struct ConfigLoader;

impl ConfigLoader {
    /// 載入指定環境的配置
    pub fn load(env: Environment) -> Result<Config, ConfigError> {
        let config_dir = env::var("CONFIG_DIR").unwrap_or_else(|_| "config".into());
        let config_path = Path::new(&config_dir).join(env.as_filename());

        let mut config_builder = Config::builder();

        // 加載環境特定配置
        config_builder = config_builder.add_source(File::from(config_path));

        // 從環境變數加載配置（優先級高於文件配置）
        config_builder = config_builder.add_source(
            ConfigEnvironment::with_prefix("BACKTEST")
                .separator("__")
                .try_parsing(true),
        );

        // 構建最終配置
        config_builder.build()
    }

    /// 載入當前環境的配置
    pub fn load_current() -> Result<Config, ConfigError> {
        Self::load(Environment::from_env())
    }
}

/// 配置獲取輔助特性
pub trait ConfigExt {
    /// 從配置中獲取並反序列化指定部分
    fn get_section<'a, T: Deserialize<'a>>(&'a self, section: &str) -> Result<T, ConfigError>;
}

impl ConfigExt for Config {
    fn get_section<'a, T: Deserialize<'a>>(&'a self, section: &str) -> Result<T, ConfigError> {
        self.get(section)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_from_env() {
        // 測試預設值
        assert_eq!(Environment::from_env(), Environment::Development);

        // 測試設定 BACKTEST_ENV
        env::set_var("BACKTEST_ENV", "production");
        assert_eq!(Environment::from_env(), Environment::Production);

        env::set_var("BACKTEST_ENV", "development");
        assert_eq!(Environment::from_env(), Environment::Development);

        // 清理環境變數
        env::remove_var("BACKTEST_ENV");
    }

    #[test]
    fn test_environment_as_filename() {
        assert_eq!(Environment::Development.as_filename(), "development.toml");
        assert_eq!(Environment::Production.as_filename(), "production.toml");
    }
}
