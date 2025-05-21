//! 金融商品結構定義

use super::types::{AssetType, DomainError, Result};
use crate::utils::time_utils::{
    opt_datetime_to_opt_timestamp_ms, opt_timestamp_ms_to_opt_datetime
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// 金融商品定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub instrument_id: String,
    pub symbol: String,
    pub exchange: String,
    pub asset_type: AssetType,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub currency: String,
    pub listing_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub lot_size: f64,
    pub tick_size: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attributes: serde_json::Value, // 存儲特定資產類型的附加屬性
}

/// 股票特有屬性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockAttributes {
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<f64>,
    pub is_etf: bool,
    pub dividend_yield: Option<f64>,
}

/// 期貨特有屬性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureAttributes {
    pub underlying_symbol: String,
    pub contract_size: f64,
    pub settlement_type: String, // "Physical" or "Cash"
    pub initial_margin: Option<f64>,
    pub maintenance_margin: Option<f64>,
}

/// 選擇權特有屬性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionAttributes {
    pub underlying_symbol: String,
    pub option_type: String, // "Call" or "Put"
    pub strike_price: f64,
    pub contract_size: f64,
    pub exercise_style: String, // "European" or "American"
}

/// 外匯特有屬性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForexAttributes {
    pub base_currency: String,
    pub quote_currency: String,
    pub pip_value: f64,
    pub min_volume: f64,
    pub max_leverage: Option<f64>,
}

/// 加密貨幣特有屬性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoAttributes {
    pub network: Option<String>,
    pub min_withdrawal: Option<f64>,
    pub withdrawal_fee: Option<f64>,
    pub blockchain: Option<String>,
    pub max_leverage: Option<f64>,
}

/// 金融商品構建器 - 提供流暢的API來構建Instrument
#[derive(Debug, Default)]
pub struct InstrumentBuilder {
    instrument_id: Option<String>,
    symbol: Option<String>,
    exchange: Option<String>,
    asset_type: Option<AssetType>,
    name: Option<String>,
    description: Option<String>,
    is_active: Option<bool>,
    currency: Option<String>,
    listing_date: Option<DateTime<Utc>>,
    expiry_date: Option<DateTime<Utc>>,
    lot_size: Option<f64>,
    tick_size: Option<f64>,
    attributes: Option<serde_json::Value>,
}

impl InstrumentBuilder {
    /// 創建一個新的樂器建構器
    pub fn new() -> Self {
        Self::default()
    }

    /// 設置金融商品唯一標識符
    pub fn instrument_id(mut self, instrument_id: impl Into<String>) -> Self {
        self.instrument_id = Some(instrument_id.into());
        self
    }

    /// 設置金融商品交易符號
    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    /// 設置金融商品所屬交易所
    pub fn exchange(mut self, exchange: impl Into<String>) -> Self {
        self.exchange = Some(exchange.into());
        self
    }

    /// 設置金融商品資產類型
    pub fn asset_type(mut self, asset_type: AssetType) -> Self {
        self.asset_type = Some(asset_type);
        self
    }

    /// 設置金融商品名稱
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 設置金融商品描述
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 設置金融商品活躍狀態
    pub fn is_active(mut self, is_active: bool) -> Self {
        self.is_active = Some(is_active);
        self
    }

    /// 設置金融商品交易貨幣
    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = Some(currency.into());
        self
    }

    /// 設置金融商品上市日期
    pub fn listing_date(mut self, listing_date: DateTime<Utc>) -> Self {
        self.listing_date = Some(listing_date);
        self
    }

    /// 設置金融商品到期日期（如適用）
    pub fn expiry_date(mut self, expiry_date: DateTime<Utc>) -> Self {
        self.expiry_date = Some(expiry_date);
        self
    }

    /// 設置金融商品手數大小
    pub fn lot_size(mut self, lot_size: f64) -> Self {
        self.lot_size = Some(lot_size);
        self
    }

    /// 設置金融商品最小價格變動單位
    pub fn tick_size(mut self, tick_size: f64) -> Self {
        self.tick_size = Some(tick_size);
        self
    }

    /// 設置金融商品類型特定屬性
    pub fn attributes(mut self, attributes: serde_json::Value) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// 對股票類型設置特定屬性
    pub fn stock_attributes(self, attrs: StockAttributes) -> Self {
        let json = serde_json::to_value(attrs).unwrap_or(serde_json::Value::Null);
        self.attributes(json)
    }

    /// 對期貨類型設置特定屬性
    pub fn future_attributes(self, attrs: FutureAttributes) -> Self {
        let json = serde_json::to_value(attrs).unwrap_or(serde_json::Value::Null);
        self.attributes(json)
    }

    /// 對選擇權類型設置特定屬性
    pub fn option_attributes(self, attrs: OptionAttributes) -> Self {
        let json = serde_json::to_value(attrs).unwrap_or(serde_json::Value::Null);
        self.attributes(json)
    }

    /// 對外匯類型設置特定屬性
    pub fn forex_attributes(self, attrs: ForexAttributes) -> Self {
        let json = serde_json::to_value(attrs).unwrap_or(serde_json::Value::Null);
        self.attributes(json)
    }

    /// 對加密貨幣類型設置特定屬性
    pub fn crypto_attributes(self, attrs: CryptoAttributes) -> Self {
        let json = serde_json::to_value(attrs).unwrap_or(serde_json::Value::Null);
        self.attributes(json)
    }

    /// 構建金融商品實例
    pub fn build(self) -> Result<Instrument> {
        let now = Utc::now();

        // 檢查必要字段
        let instrument_id = self
            .instrument_id
            .ok_or_else(|| DomainError::MissingRequiredField("instrument_id".to_string()))?;

        let symbol = self
            .symbol
            .ok_or_else(|| DomainError::MissingRequiredField("symbol".to_string()))?;

        let exchange = self
            .exchange
            .ok_or_else(|| DomainError::MissingRequiredField("exchange".to_string()))?;

        let asset_type = self
            .asset_type
            .ok_or_else(|| DomainError::MissingRequiredField("asset_type".to_string()))?;

        // 如果沒有提供名稱，使用交易代碼作為名稱
        let name = self.name.unwrap_or_else(|| symbol.clone());

        Ok(Instrument {
            instrument_id,
            symbol,
            exchange,
            asset_type,
            name,
            description: self.description,
            is_active: self.is_active.unwrap_or(true),
            currency: self.currency.unwrap_or_else(|| "USD".to_string()),
            listing_date: self.listing_date,
            expiry_date: self.expiry_date,
            lot_size: self.lot_size.unwrap_or(1.0),
            tick_size: self.tick_size.unwrap_or(0.01),
            created_at: now,
            updated_at: now,
            attributes: self.attributes.unwrap_or(serde_json::Value::Null),
        })
    }
}

impl Instrument {
    /// 創建一個新的金融商品 (簡便方法)
    pub fn new(
        instrument_id: impl Into<String>,
        symbol: impl Into<String>,
        exchange: impl Into<String>,
        asset_type: AssetType,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            symbol: symbol.into(),
            exchange: exchange.into(),
            asset_type,
            name: String::new(),
            description: None,
            is_active: true,
            currency: "USD".to_string(),
            listing_date: None,
            expiry_date: None,
            lot_size: 1.0,
            tick_size: 0.01,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            attributes: serde_json::Value::Null,
        }
    }

    /// 使用UUID創建一個新的金融商品
    pub fn with_uuid(
        symbol: impl Into<String>,
        exchange: impl Into<String>,
        asset_type: AssetType,
    ) -> Self {
        let uuid = Uuid::new_v4().to_string();
        Self::new(uuid, symbol, exchange, asset_type)
    }

    /// 創建一個新的構建器來構建金融商品
    pub fn builder() -> InstrumentBuilder {
        InstrumentBuilder::new()
    }

    /// 設置金融商品名稱
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 設置金融商品幣種
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    /// 設置金融商品屬性
    pub fn with_attributes(mut self, attributes: serde_json::Value) -> Self {
        self.attributes = attributes;
        self
    }

    /// 檢查金融商品是否處於活躍狀態
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// 檢查金融商品是否已到期
    pub fn is_expired(&self) -> bool {
        match self.expiry_date {
            Some(expiry) => expiry < Utc::now(),
            None => false,
        }
    }

    /// 獲取完整的市場識別符（如 NASDAQ:AAPL）
    pub fn market_id(&self) -> String {
        format!("{}:{}", self.exchange, self.symbol)
    }

    /// 從屬性中獲取特定資產類型的屬性
    pub fn get_stock_attributes(&self) -> Option<StockAttributes> {
        if self.asset_type == AssetType::Stock {
            serde_json::from_value(self.attributes.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取期貨特定屬性
    pub fn get_future_attributes(&self) -> Option<FutureAttributes> {
        if self.asset_type == AssetType::Future {
            serde_json::from_value(self.attributes.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取選擇權特定屬性
    pub fn get_option_attributes(&self) -> Option<OptionAttributes> {
        if self.asset_type == AssetType::Option {
            serde_json::from_value(self.attributes.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取外匯特定屬性
    pub fn get_forex_attributes(&self) -> Option<ForexAttributes> {
        if self.asset_type == AssetType::Forex {
            serde_json::from_value(self.attributes.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取加密貨幣特定屬性
    pub fn get_crypto_attributes(&self) -> Option<CryptoAttributes> {
        if self.asset_type == AssetType::Crypto {
            serde_json::from_value(self.attributes.clone()).ok()
        } else {
            None
        }
    }

    /// 更新最後修改時間
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// 設置金融商品為活躍狀態
    pub fn activate(&mut self) {
        self.is_active = true;
        self.touch();
    }

    /// 設置金融商品為非活躍狀態
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.touch();
    }
    
    /// 將可選的日期時間轉換為毫秒時間戳（供資料庫層使用）
    ///
    /// 這個方法主要用於在將 Instrument 存入資料庫時，
    /// 將 DateTime<Utc> 類型的日期時間字段轉換為 i64 毫秒時間戳
    pub fn listing_date_to_timestamp(&self) -> Option<i64> {
        opt_datetime_to_opt_timestamp_ms(&self.listing_date)
    }
    
    /// 將可選的日期時間轉換為毫秒時間戳（供資料庫層使用）
    pub fn expiry_date_to_timestamp(&self) -> Option<i64> {
        opt_datetime_to_opt_timestamp_ms(&self.expiry_date)
    }
    
    /// 從毫秒時間戳創建 Instrument 的日期時間字段
    ///
    /// 這個方法主要用於從資料庫讀取 Instrument 時，
    /// 將 i64 毫秒時間戳轉換為 DateTime<Utc> 類型
    pub fn timestamp_to_listing_date(timestamp: Option<i64>) -> Option<DateTime<Utc>> {
        opt_timestamp_ms_to_opt_datetime(timestamp)
    }
    
    /// 從毫秒時間戳創建 Instrument 的日期時間字段
    pub fn timestamp_to_expiry_date(timestamp: Option<i64>) -> Option<DateTime<Utc>> {
        opt_timestamp_ms_to_opt_datetime(timestamp)
    }
}

impl fmt::Display for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}) - {}", self.symbol, self.name, self.asset_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_creation() {
        let inst = Instrument::new("AAPL123", "AAPL", "NASDAQ", AssetType::Stock)
            .with_name("Apple Inc.")
            .with_currency("USD");

        assert_eq!(inst.instrument_id, "AAPL123");
        assert_eq!(inst.symbol, "AAPL");
        assert_eq!(inst.exchange, "NASDAQ");
        assert_eq!(inst.asset_type, AssetType::Stock);
        assert_eq!(inst.name, "Apple Inc.");
        assert_eq!(inst.currency, "USD");
        assert!(inst.is_active);
    }

    #[test]
    fn test_instrument_builder() {
        let stock_attrs = StockAttributes {
            sector: Some("Technology".into()),
            industry: Some("Consumer Electronics".into()),
            market_cap: Some(2500000000000.0),
            is_etf: false,
            dividend_yield: Some(0.53),
        };

        let inst = Instrument::builder()
            .instrument_id("AAPL123")
            .symbol("AAPL")
            .exchange("NASDAQ")
            .asset_type(AssetType::Stock)
            .name("Apple Inc.")
            .description("Apple Inc. designs, manufactures, and markets smartphones, personal computers, tablets, wearables, and accessories worldwide.")
            .currency("USD")
            .tick_size(0.01)
            .lot_size(100.0)
            .stock_attributes(stock_attrs)
            .build()
            .unwrap();

        assert_eq!(inst.instrument_id, "AAPL123");
        assert_eq!(inst.symbol, "AAPL");
        assert_eq!(inst.asset_type, AssetType::Stock);
        assert_eq!(inst.name, "Apple Inc.");
        assert_eq!(inst.lot_size, 100.0);

        let attrs = inst.get_stock_attributes().unwrap();
        assert_eq!(attrs.sector, Some("Technology".to_string()));
        assert_eq!(attrs.industry, Some("Consumer Electronics".to_string()));
        assert!(!attrs.is_etf);
    }

    #[test]
    fn test_market_id() {
        let inst = Instrument::new("BTC123", "BTC/USD", "Binance", AssetType::Crypto);
        assert_eq!(inst.market_id(), "Binance:BTC/USD");
    }

    #[test]
    fn test_instrument_with_uuid() {
        let inst = Instrument::with_uuid("AAPL", "NASDAQ", AssetType::Stock);
        assert!(!inst.instrument_id.is_empty());
        assert_ne!(inst.instrument_id, "AAPL"); // Should be a UUID
        assert_eq!(inst.symbol, "AAPL");
    }
    
    #[test]
    fn test_datetime_timestamp_conversion() {
        // 創建帶有日期時間的金融商品
        let listing_date = Utc::now();
        let expiry_date = listing_date + chrono::Duration::days(90);
        
        let inst = Instrument::builder()
            .instrument_id("TEST123")
            .symbol("TEST")
            .exchange("TEST")
            .asset_type(AssetType::Future)
            .listing_date(listing_date)
            .expiry_date(expiry_date)
            .build()
            .unwrap();
            
        // 測試轉換為毫秒時間戳
        let listing_ts = inst.listing_date_to_timestamp();
        let expiry_ts = inst.expiry_date_to_timestamp();
        
        assert!(listing_ts.is_some());
        assert!(expiry_ts.is_some());
        
        // 測試從毫秒時間戳轉換回日期時間
        let listing_dt = Instrument::timestamp_to_listing_date(listing_ts);
        let expiry_dt = Instrument::timestamp_to_expiry_date(expiry_ts);
        
        assert!(listing_dt.is_some());
        assert!(expiry_dt.is_some());
        
        // 比較原始日期時間和轉換後的日期時間
        let listing_diff = (listing_date - listing_dt.unwrap()).num_milliseconds().abs();
        let expiry_diff = (expiry_date - expiry_dt.unwrap()).num_milliseconds().abs();
        
        // 允許1毫秒的轉換誤差
        assert!(listing_diff <= 1, "Listing date conversion error too large: {}", listing_diff);
        assert!(expiry_diff <= 1, "Expiry date conversion error too large: {}", expiry_diff);
    }
    
    #[test]
    fn test_optional_datetime_conversion() {
        // 測試 None 值的情況
        let inst = Instrument::builder()
            .instrument_id("TEST123")
            .symbol("TEST")
            .exchange("TEST")
            .asset_type(AssetType::Stock)
            .build()
            .unwrap();
            
        // None 值應該轉換為 None
        assert!(inst.listing_date.is_none());
        assert!(inst.listing_date_to_timestamp().is_none());
        
        // None 時間戳應該轉換為 None DateTime
        assert!(Instrument::timestamp_to_listing_date(None).is_none());
    }
}
