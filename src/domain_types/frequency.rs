//! 頻率定義模組 - 從 TOML 檔案動態生成所有頻率相關的程式碼
//! 
//! 這個模組是所有頻率定義的單一來源，所有頻率相關的程式碼都從 config/frequencies.toml 自動生成

use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
use std::path::Path;

/// 頻率配置 - 從 TOML 檔案載入
#[derive(Debug, Deserialize, Clone)]
pub struct FrequencyConfig {
    pub frequency: Vec<FrequencyDef>,
}

/// 單個頻率定義
#[derive(Debug, Deserialize, Clone)]
pub struct FrequencyDef {
    pub name: String,
    pub enum_name: String,
    pub struct_name: String,
    pub seconds: u64,
    pub milliseconds: u64,
    pub polars_string: String,
    pub display_name: String,
    pub alias_suffix: String,
    pub is_ohlcv: bool,
}

impl FrequencyConfig {
    /// 從檔案載入頻率配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
    
    /// 從預設位置載入
    pub fn load_default() -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_from_file("config/frequencies.toml")
    }
    
    /// 獲取所有 OHLCV 頻率
    pub fn ohlcv_frequencies(&self) -> Vec<&FrequencyDef> {
        self.frequency.iter().filter(|f| f.is_ohlcv).collect()
    }
    
    /// 獲取所有頻率名稱
    pub fn frequency_names(&self) -> Vec<&str> {
        self.frequency.iter().map(|f| f.name.as_str()).collect()
    }
}

// 包含由 build.rs 生成的頻率宏定義
include!(concat!(env!("OUT_DIR"), "/frequencies_generated.rs"));

/// 生成頻率枚舉的內部宏
macro_rules! generate_frequency_enum {
    ($(($variant:ident, $alias:literal, $is_ohlcv:literal, $seconds:literal, $milliseconds:literal, $polars:literal, $display:literal)),*) => {
        /// 數據頻率定義
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum Frequency {
            $(
                $variant,
            )*
        }
        
        impl Frequency {
            /// 轉換為表示該頻率的 std::time::Duration
            pub fn to_std_duration(&self) -> StdDuration {
                match self {
                    $(
                        Frequency::$variant => StdDuration::from_secs($seconds),
                    )*
                }
            }

            /// 轉換為表示該頻率的 Polars Duration
            pub fn to_duration(&self) -> polars::prelude::Duration {
                match self {
                    $(
                        Frequency::$variant => polars::prelude::Duration::parse(&format!("{}i", $milliseconds)),
                    )*
                }
            }

            /// 轉換為 Polars 可識別的時間字串
            pub fn to_polars_duration_string(&self) -> String {
                match self {
                    $(
                        Frequency::$variant => $polars.to_string(),
                    )*
                }
            }
            
            /// 獲取頻率的秒數
            pub fn seconds(&self) -> u64 {
                match self {
                    $(
                        Frequency::$variant => $seconds,
                    )*
                }
            }
            
            /// 獲取頻率的毫秒數
            pub fn milliseconds(&self) -> u64 {
                match self {
                    $(
                        Frequency::$variant => $milliseconds,
                    )*
                }
            }
            
            /// 檢查是否為 OHLCV 頻率
            pub fn is_ohlcv(&self) -> bool {
                match self {
                    $(
                        Frequency::$variant => $is_ohlcv,
                    )*
                }
            }
            
            /// 獲取顯示名稱
            pub fn display_name(&self) -> &'static str {
                match self {
                    $(
                        Frequency::$variant => $display,
                    )*
                }
            }
            
            /// 獲取別名後綴
            pub fn alias_suffix(&self) -> &'static str {
                match self {
                    $(
                        Frequency::$variant => $alias,
                    )*
                }
            }
            
            /// 獲取所有頻率列表
            pub fn all() -> Vec<Frequency> {
                vec![
                    $(
                        Frequency::$variant,
                    )*
                ]
            }
            
            /// 獲取所有 OHLCV 頻率
            pub fn all_ohlcv() -> Vec<Frequency> {
                Self::all().into_iter().filter(|f| f.is_ohlcv()).collect()
            }
        }
    };
}

/// 生成頻率標記結構的內部宏
macro_rules! generate_frequency_structs {
    ($(($variant:ident, $alias:literal, $is_ohlcv:literal, $seconds:literal, $milliseconds:literal, $polars:literal, $display:literal)),*) => {
        /// 頻率標記 trait，所有頻率類型都必須實現
        pub trait FrequencyMarker: Send + Sync + 'static {
            /// 轉換為對應的頻率枚舉
            fn to_frequency() -> Frequency;
            
            /// 獲取頻率名稱
            fn name() -> &'static str;
        }
        
        $(
            #[doc = concat!($display, " 頻率標記")]
            #[derive(Debug, Clone, Copy)]
            pub struct $variant;

            impl FrequencyMarker for $variant {
                fn to_frequency() -> Frequency { Frequency::$variant }
                fn name() -> &'static str { $display }
            }
        )*
    };
}

/// 生成 import 宏的內部宏
macro_rules! generate_import_macro {
    ($(($variant:ident, $alias:literal, $is_ohlcv:literal, $seconds:literal, $milliseconds:literal, $polars:literal, $display:literal)),*) => {
        /// 匯入所有頻率類型的宏
        #[macro_export]
        macro_rules! import_all_frequency_types {
            () => {
                use $crate::domain_types::frequency::{
                    Frequency, FrequencyMarker,
                    $(
                        $variant,
                    )*
                };
            };
        }
    };
}

/// 生成 OHLCV 宏的內部宏
macro_rules! generate_ohlcv_macro {
    ($(($variant:ident, $alias:literal, $is_ohlcv:literal, $seconds:literal, $milliseconds:literal, $polars:literal, $display:literal)),*) => {
        /// 為每個 OHLCV 頻率執行宏
        #[macro_export]
        macro_rules! for_each_ohlcv_frequency {
            ($macro:ident) => {
                $macro! {
                    $(
                        // 只包含 OHLCV 頻率（is_ohlcv = true）
                        generate_ohlcv_macro!(@filter $macro, $variant, $is_ohlcv);
                    )*
                }
            };
        }
    };
    
    // 過濾輔助宏
    (@filter $macro:ident, $variant:ident, true) => {
        $variant => $variant,
    };
    (@filter $macro:ident, $variant:ident, false) => {
        // 跳過非 OHLCV 頻率
    };
}


/// 生成所有頻率宏的內部宏
macro_rules! generate_all_frequencies_macro {
    ($(($variant:ident, $alias:literal, $is_ohlcv:literal, $seconds:literal, $milliseconds:literal, $polars:literal, $display:literal)),*) => {
        /// 為所有頻率執行宏
        #[macro_export]
        macro_rules! for_all_frequencies {
            ($macro:ident) => {
                $macro! {
                    $(
                        $variant => $alias, $is_ohlcv;
                    )*
                }
            };
        }
    };
}


// 使用主宏生成所有程式碼
frequencies!(generate_frequency_enum);
frequencies!(generate_frequency_structs);
frequencies!(generate_import_macro);
frequencies!(generate_ohlcv_macro);
frequencies!(generate_all_frequencies_macro);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_conversions() {
        assert_eq!(Frequency::Minute.to_std_duration(), StdDuration::from_secs(60));
        assert_eq!(Frequency::Hour.to_std_duration(), StdDuration::from_secs(3600));
        assert_eq!(Frequency::Day.to_std_duration(), StdDuration::from_secs(86400));
    }

    #[test]
    fn test_frequency_marker() {
        assert_eq!(Minute::to_frequency(), Frequency::Minute);
        assert_eq!(Minute::name(), "Minute");
        assert_eq!(Hour::to_frequency(), Frequency::Hour);
        assert_eq!(Hour::name(), "Hour");
    }
}