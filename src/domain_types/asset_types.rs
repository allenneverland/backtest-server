use serde::{Deserialize, Serialize};
use std::fmt;

/// 資產類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum AssetType {
    Stock,          // 股票
    Future,         // 期貨
    OptionContract, // 期權
    Forex,          // 外匯
    Crypto,         // 加密貨幣
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetType::Stock => write!(f, "Stock"),
            AssetType::Future => write!(f, "Future"),
            AssetType::OptionContract => write!(f, "OptionContract"),
            AssetType::Forex => write!(f, "Forex"),
            AssetType::Crypto => write!(f, "Crypto"),
        }
    }
}

/// 數據類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum DataType {
    OHLCV,          // 開高低收成交量
    Tick,           // 逐筆成交
    OrderBook,      // 訂單簿
    Trade,          // 成交
    Quote,          // 報價
    Indicator(u32), // 技術指標，存儲指標ID
} 

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::OHLCV => write!(f, "OHLCV"),
            DataType::Tick => write!(f, "Tick"),
            DataType::OrderBook => write!(f, "OrderBook"),
            DataType::Trade => write!(f, "Trade"),
            DataType::Quote => write!(f, "Quote"),
            DataType::Indicator(id) => write!(f, "Indicator({})", id),
        }
    }
} 