use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;

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
    pub attributes: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub attributes: Option<Json<serde_json::Value>>,
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
            attributes: self.attributes,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// 股票特定屬性模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockAttributes {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureAttributes {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionAttributes {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForexAttributes {
    pub base_currency: String,
    pub quote_currency: String,
    pub pip_value: Decimal,
    pub typical_spread: Option<Decimal>,
    pub margin_requirement: Option<Decimal>,
    pub trading_hours: Option<serde_json::Value>,
}

/// 虛擬貨幣特定屬性模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoAttributes {
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

// 完整視圖資料結構
#[derive(Debug, Clone, FromRow)]
pub struct InstrumentWithExchange {
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
    pub attributes: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 交易所相關欄位
    pub exchange_code: String,
    pub exchange_name: String,
    pub exchange_country: String,
}

impl Instrument {
    /// 從屬性中獲取股票特定屬性
    pub fn get_stock_attributes(&self) -> Option<StockAttributes> {
        if self.instrument_type == "STOCK" && self.attributes.is_some() {
            let attrs = self.attributes.as_ref().unwrap();
            serde_json::from_value(attrs.0.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取期貨特定屬性
    pub fn get_future_attributes(&self) -> Option<FutureAttributes> {
        if self.instrument_type == "FUTURE" && self.attributes.is_some() {
            let attrs = self.attributes.as_ref().unwrap();
            serde_json::from_value(attrs.0.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取選擇權特定屬性
    pub fn get_option_attributes(&self) -> Option<OptionAttributes> {
        if self.instrument_type == "OPTIONCONTRACT" && self.attributes.is_some() {
            let attrs = self.attributes.as_ref().unwrap();
            serde_json::from_value(attrs.0.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取外匯特定屬性
    pub fn get_forex_attributes(&self) -> Option<ForexAttributes> {
        if self.instrument_type == "FOREX" && self.attributes.is_some() {
            let attrs = self.attributes.as_ref().unwrap();
            serde_json::from_value(attrs.0.clone()).ok()
        } else {
            None
        }
    }

    /// 從屬性中獲取虛擬貨幣特定屬性
    pub fn get_crypto_attributes(&self) -> Option<CryptoAttributes> {
        if self.instrument_type == "CRYPTO" && self.attributes.is_some() {
            let attrs = self.attributes.as_ref().unwrap();
            serde_json::from_value(attrs.0.clone()).ok()
        } else {
            None
        }
    }

    /// 設置股票特定屬性
    pub fn set_stock_attributes(
        &mut self,
        attrs: StockAttributes,
    ) -> Result<(), serde_json::Error> {
        let json = serde_json::to_value(attrs)?;
        self.attributes = Some(Json(json));
        Ok(())
    }

    /// 設置期貨特定屬性
    pub fn set_future_attributes(
        &mut self,
        attrs: FutureAttributes,
    ) -> Result<(), serde_json::Error> {
        let json = serde_json::to_value(attrs)?;
        self.attributes = Some(Json(json));
        Ok(())
    }

    /// 設置選擇權特定屬性
    pub fn set_option_attributes(
        &mut self,
        attrs: OptionAttributes,
    ) -> Result<(), serde_json::Error> {
        let json = serde_json::to_value(attrs)?;
        self.attributes = Some(Json(json));
        Ok(())
    }

    /// 設置外匯特定屬性
    pub fn set_forex_attributes(
        &mut self,
        attrs: ForexAttributes,
    ) -> Result<(), serde_json::Error> {
        let json = serde_json::to_value(attrs)?;
        self.attributes = Some(Json(json));
        Ok(())
    }

    /// 設置虛擬貨幣特定屬性
    pub fn set_crypto_attributes(
        &mut self,
        attrs: CryptoAttributes,
    ) -> Result<(), serde_json::Error> {
        let json = serde_json::to_value(attrs)?;
        self.attributes = Some(Json(json));
        Ok(())
    }
}
