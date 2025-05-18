use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;

// 用於將視圖資料轉換為API響應的特性
pub trait ViewToSymbolInfo {
    fn to_symbol_info(&self) -> crate::api::handlers::data::SymbolInfo;
}

/// 股票特定屬性模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Stock {
    pub instrument_id: i32,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<Decimal>,
    pub shares_outstanding: Option<i64>,
    pub free_float: Option<i64>,
    pub listing_date: Option<NaiveDate>,
    pub delisting_date: Option<NaiveDate>,
    pub dividend_yield: Option<Decimal>,
    pub pe_ratio: Option<Decimal>,
}

/// 期貨特定屬性模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Future {
    pub instrument_id: i32,
    pub underlying_asset: String,
    pub contract_size: Decimal,
    pub contract_unit: Option<String>,
    pub delivery_date: NaiveDate,
    pub first_notice_date: Option<NaiveDate>,
    pub last_trading_date: NaiveDate,
    pub settlement_type: String,
    pub initial_margin: Option<Decimal>,
    pub maintenance_margin: Option<Decimal>,
    pub price_quotation: Option<String>,
}

/// 選擇權特定屬性模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OptionContract {
    pub instrument_id: i32,
    pub underlying_instrument_id: Option<i32>,
    pub option_type: String, // 'CALL', 'PUT'
    pub strike_price: Decimal,
    pub expiration_date: NaiveDate,
    pub exercise_style: String, // 'AMERICAN', 'EUROPEAN', 'ASIAN'
    pub contract_size: i32,
    pub implied_volatility: Option<Decimal>,
    pub delta: Option<Decimal>,
    pub gamma: Option<Decimal>,
    pub theta: Option<Decimal>,
    pub vega: Option<Decimal>,
    pub rho: Option<Decimal>,
}

/// 外匯特定屬性模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Forex {
    pub instrument_id: i32,
    pub base_currency: String,
    pub quote_currency: String,
    pub pip_value: Decimal,
    pub typical_spread: Option<Decimal>,
    pub margin_requirement: Option<Decimal>,
    pub trading_hours: Option<Json<serde_json::Value>>,
}

/// 虛擬貨幣特定屬性模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Crypto {
    pub instrument_id: i32,
    pub blockchain_network: Option<String>,
    pub total_supply: Option<Decimal>,
    pub circulating_supply: Option<Decimal>,
    pub max_supply: Option<Decimal>,
    pub mining_algorithm: Option<String>,
    pub consensus_mechanism: Option<String>,
    pub website_url: Option<String>,
    pub whitepaper_url: Option<String>,
    pub github_url: Option<String>,
}

/// 金融商品插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentInsert {
    pub symbol: String,
    pub exchange_id: Option<i32>,
    pub instrument_type: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub tick_size: Option<Decimal>,
    pub lot_size: Option<i32>,
    pub is_active: bool,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
}

impl InstrumentInsert {
    /// 轉換為實體模型，用於資料庫插入
    pub fn into_entity(self) -> Instrument {
        Instrument {
            instrument_id: 0, // 新建時由資料庫生成
            symbol: self.symbol,
            exchange_id: self.exchange_id,
            instrument_type: self.instrument_type,
            name: self.name,
            description: self.description,
            currency: self.currency,
            tick_size: self.tick_size,
            lot_size: self.lot_size,
            is_active: self.is_active,
            trading_start_date: self.trading_start_date,
            trading_end_date: self.trading_end_date,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// 股票插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInsert {
    pub instrument_id: i32,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<Decimal>,
    pub shares_outstanding: Option<i64>,
    pub free_float: Option<i64>,
    pub listing_date: Option<NaiveDate>,
    pub delisting_date: Option<NaiveDate>,
    pub dividend_yield: Option<Decimal>,
    pub pe_ratio: Option<Decimal>,
}

impl StockInsert {
    /// 轉換為實體模型，用於資料庫插入
    pub fn into_entity(self) -> Stock {
        Stock {
            instrument_id: self.instrument_id,
            sector: self.sector,
            industry: self.industry,
            market_cap: self.market_cap,
            shares_outstanding: self.shares_outstanding,
            free_float: self.free_float,
            listing_date: self.listing_date,
            delisting_date: self.delisting_date,
            dividend_yield: self.dividend_yield,
            pe_ratio: self.pe_ratio,
        }
    }
}

/// 期貨插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureInsert {
    pub instrument_id: i32,
    pub underlying_asset: String,
    pub contract_size: Decimal,
    pub contract_unit: Option<String>,
    pub delivery_date: NaiveDate,
    pub first_notice_date: Option<NaiveDate>,
    pub last_trading_date: NaiveDate,
    pub settlement_type: String,
    pub initial_margin: Option<Decimal>,
    pub maintenance_margin: Option<Decimal>,
    pub price_quotation: Option<String>,
}

/// 選擇權插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionInsert {
    pub instrument_id: i32,
    pub underlying_instrument_id: Option<i32>,
    pub option_type: String,
    pub strike_price: Decimal,
    pub expiration_date: NaiveDate,
    pub exercise_style: String,
    pub contract_size: i32,
    pub implied_volatility: Option<Decimal>,
    pub delta: Option<Decimal>,
    pub gamma: Option<Decimal>,
    pub theta: Option<Decimal>,
    pub vega: Option<Decimal>,
    pub rho: Option<Decimal>,
}

/// 外匯插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForexInsert {
    pub instrument_id: i32,
    pub base_currency: String,
    pub quote_currency: String,
    pub pip_value: Decimal,
    pub typical_spread: Option<Decimal>,
    pub margin_requirement: Option<Decimal>,
    pub trading_hours: Option<Json<serde_json::Value>>,
}

/// 虛擬貨幣插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoInsert {
    pub instrument_id: i32,
    pub blockchain_network: Option<String>,
    pub total_supply: Option<Decimal>,
    pub circulating_supply: Option<Decimal>,
    pub max_supply: Option<Decimal>,
    pub mining_algorithm: Option<String>,
    pub consensus_mechanism: Option<String>,
    pub website_url: Option<String>,
    pub whitepaper_url: Option<String>,
    pub github_url: Option<String>,
}

/// 金融商品模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Instrument {
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_id: Option<i32>,
    pub instrument_type: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub tick_size: Option<Decimal>,
    pub lot_size: Option<i32>,
    pub is_active: bool,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 完整視圖資料結構
#[derive(Debug, Clone, FromRow)]
pub struct FutureComplete {
    // 繼承自 Instrument 的欄位
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_id: Option<i32>,
    pub instrument_type: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub tick_size: Option<Decimal>,
    pub lot_size: Option<i32>,
    pub is_active: bool,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 交易所相關欄位
    pub exchange_code: String,
    pub exchange_name: String,
    pub exchange_country: String,

    // Future 特定欄位
    pub underlying_asset: Option<String>,
    pub contract_size: Option<Decimal>,
    pub contract_unit: Option<String>,
    pub delivery_date: Option<NaiveDate>,
    pub first_notice_date: Option<NaiveDate>,
    pub last_trading_date: Option<NaiveDate>,
    pub settlement_type: Option<String>,
    pub initial_margin: Option<Decimal>,
    pub maintenance_margin: Option<Decimal>,
    pub price_quotation: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct StockComplete {
    // 繼承自 Instrument 的欄位
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_id: Option<i32>,
    pub instrument_type: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub tick_size: Option<Decimal>,
    pub lot_size: Option<i32>,
    pub is_active: bool,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 交易所相關欄位
    pub exchange_code: String,
    pub exchange_name: String,
    pub exchange_country: String,

    // Stock 特定欄位
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<Decimal>,
    pub shares_outstanding: Option<i64>,
    pub free_float: Option<i64>,
    pub listing_date: Option<NaiveDate>,
    pub delisting_date: Option<NaiveDate>,
    pub dividend_yield: Option<Decimal>,
    pub pe_ratio: Option<Decimal>,
}

#[derive(Debug, Clone, FromRow)]
pub struct OptionComplete {
    // 繼承自 Instrument 的欄位
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_id: Option<i32>,
    pub instrument_type: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub tick_size: Option<Decimal>,
    pub lot_size: Option<i32>,
    pub is_active: bool,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 交易所相關欄位
    pub exchange_code: String,
    pub exchange_name: String,
    pub exchange_country: String,

    // Option 特定欄位
    pub underlying_instrument_id: Option<i32>,
    pub underlying_symbol: Option<String>,
    pub underlying_name: Option<String>,
    pub option_type: String,
    pub strike_price: Decimal,
    pub expiration_date: NaiveDate,
    pub exercise_style: String,
    pub contract_size: i32,
    pub implied_volatility: Option<Decimal>,
    pub delta: Option<Decimal>,
    pub gamma: Option<Decimal>,
    pub theta: Option<Decimal>,
    pub vega: Option<Decimal>,
    pub rho: Option<Decimal>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ForexComplete {
    // 繼承自 Instrument 的欄位
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_id: Option<i32>,
    pub instrument_type: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub tick_size: Option<Decimal>,
    pub lot_size: Option<i32>,
    pub is_active: bool,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 交易所相關欄位
    pub exchange_code: String,
    pub exchange_name: String,
    pub exchange_country: String,

    // Forex 特定欄位
    pub base_currency: String,
    pub quote_currency: String,
    pub pip_value: Decimal,
    pub typical_spread: Option<Decimal>,
    pub margin_requirement: Option<Decimal>,
    pub trading_hours: Option<Json<serde_json::Value>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CryptoComplete {
    // 繼承自 Instrument 的欄位
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_id: Option<i32>,
    pub instrument_type: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub tick_size: Option<Decimal>,
    pub lot_size: Option<i32>,
    pub is_active: bool,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 交易所相關欄位
    pub exchange_code: String,
    pub exchange_name: String,
    pub exchange_country: String,

    // Crypto 特定欄位
    pub blockchain_network: Option<String>,
    pub total_supply: Option<Decimal>,
    pub circulating_supply: Option<Decimal>,
    pub max_supply: Option<Decimal>,
    pub mining_algorithm: Option<String>,
    pub consensus_mechanism: Option<String>,
    pub website_url: Option<String>,
    pub whitepaper_url: Option<String>,
    pub github_url: Option<String>,
}

// 為 FutureComplete 實現視圖轉換特性
impl ViewToSymbolInfo for FutureComplete {
    fn to_symbol_info(&self) -> crate::api::handlers::data::SymbolInfo {
        crate::api::handlers::data::SymbolInfo {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            asset_type: self.instrument_type.clone(),
            exchange: self.exchange_name.clone(),
            sector: None,
            industry: None,
            listing_date: self.trading_start_date.map(|d| d.to_string()),
            is_active: self.is_active,
        }
    }
}

// 為 StockComplete 實現視圖轉換特性
impl ViewToSymbolInfo for StockComplete {
    fn to_symbol_info(&self) -> crate::api::handlers::data::SymbolInfo {
        crate::api::handlers::data::SymbolInfo {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            asset_type: self.instrument_type.clone(),
            exchange: self.exchange_name.clone(),
            sector: self.sector.clone(),
            industry: self.industry.clone(),
            listing_date: self.listing_date.map(|d| d.to_string()),
            is_active: self.is_active,
        }
    }
}

// 為 OptionComplete 實現視圖轉換特性
impl ViewToSymbolInfo for OptionComplete {
    fn to_symbol_info(&self) -> crate::api::handlers::data::SymbolInfo {
        crate::api::handlers::data::SymbolInfo {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            asset_type: self.instrument_type.clone(),
            exchange: self.exchange_name.clone(),
            sector: None,
            industry: None,
            listing_date: Some(self.expiration_date.to_string()),
            is_active: self.is_active,
        }
    }
}

// 為 ForexComplete 實現視圖轉換特性
impl ViewToSymbolInfo for ForexComplete {
    fn to_symbol_info(&self) -> crate::api::handlers::data::SymbolInfo {
        crate::api::handlers::data::SymbolInfo {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            asset_type: self.instrument_type.clone(),
            exchange: self.exchange_name.clone(),
            sector: Some("Currencies".to_string()),
            industry: Some("Forex".to_string()),
            listing_date: None,
            is_active: self.is_active,
        }
    }
}

// 為 CryptoComplete 實現視圖轉換特性
impl ViewToSymbolInfo for CryptoComplete {
    fn to_symbol_info(&self) -> crate::api::handlers::data::SymbolInfo {
        crate::api::handlers::data::SymbolInfo {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            asset_type: self.instrument_type.clone(),
            exchange: self.exchange_name.clone(),
            sector: Some("Cryptocurrencies".to_string()),
            industry: None,
            listing_date: None,
            is_active: self.is_active,
        }
    }
} 