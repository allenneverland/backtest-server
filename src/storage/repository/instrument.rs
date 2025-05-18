use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, FromRow};
use std::fmt::Debug;
use serde_json;
use std::str::FromStr;
use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::domain_types::asset_types::AssetType;
use crate::storage::models::{
    Instrument,
    instrument::{Stock, Future, OptionContract, Forex, Crypto, FutureComplete, StockComplete, OptionComplete, ForexComplete, CryptoComplete},
};
use crate::storage::repository::{DbExecutor, PageQuery, Page};
use crate::storage::models::Exchange;

/// 支持的視圖類型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewType {
    StockComplete,
    FutureComplete,
    OptionComplete,
    ForexComplete,
    CryptoComplete,
}

impl ViewType {
    /// 將字符串轉換為視圖類型
    pub fn from_str(view_type: &str) -> Option<Self> {
        match view_type.to_lowercase().as_str() {
            "stock_complete" => Some(ViewType::StockComplete),
            "future_complete" => Some(ViewType::FutureComplete),
            "option_complete" => Some(ViewType::OptionComplete),
            "forex_complete" => Some(ViewType::ForexComplete),
            "crypto_complete" => Some(ViewType::CryptoComplete),
            _ => None,
        }
    }
    
    /// 獲取視圖表名
    pub fn table_name(&self) -> &'static str {
        match self {
            ViewType::StockComplete => "stock_complete",
            ViewType::FutureComplete => "future_complete",
            ViewType::OptionComplete => "option_complete",
            ViewType::ForexComplete => "forex_complete",
            ViewType::CryptoComplete => "crypto_complete",
        }
    }
}

/// 金融商品儲存庫特性
#[async_trait]
pub trait InstrumentRepository: Send + Sync {
    // 基本操作
    async fn get_instrument_by_id(&self, instrument_id: i32) -> Result<Option<Instrument>>;
    async fn get_instrument_by_symbol(&self, symbol: &str, exchange_id: Option<i32>, instrument_type: Option<&str>) -> Result<Option<Instrument>>;
    async fn get_instruments(&self, instrument_type: Option<&str>, page: PageQuery) -> Result<Page<Instrument>>;
    async fn insert_instrument(&self, instrument: &Instrument) -> Result<i32>;
    async fn update_instrument(&self, instrument: &Instrument) -> Result<()>;
    
    // 股票特定操作
    async fn get_stock_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Stock>>;
    async fn insert_stock(&self, stock: &Stock) -> Result<()>;
    async fn update_stock(&self, stock: &Stock) -> Result<()>;
    
    // 期貨特定操作
    async fn get_future_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Future>>;
    async fn insert_future(&self, future: &Future) -> Result<()>;
    async fn update_future(&self, future: &Future) -> Result<()>;
    
    // 選擇權特定操作
    async fn get_option_contract_by_instrument_id(&self, instrument_id: i32) -> Result<Option<OptionContract>>;
    async fn insert_option_contract(&self, option_contract: &OptionContract) -> Result<()>;
    async fn update_option_contract(&self, option_contract: &OptionContract) -> Result<()>;
    
    // 外匯特定操作
    async fn get_forex_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Forex>>;
    async fn insert_forex(&self, forex: &Forex) -> Result<()>;
    async fn update_forex(&self, forex: &Forex) -> Result<()>;
    
    // 加密貨幣特定操作
    async fn get_crypto_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Crypto>>;
    async fn insert_crypto(&self, crypto: &Crypto) -> Result<()>;
    async fn update_crypto(&self, crypto: &Crypto) -> Result<()>;
    
    // 創建或獲取金融商品
    async fn get_or_create_instrument(&self, symbol: &str, asset_type: AssetType) -> Result<i32>;
    
    // 交易所相關操作
    async fn get_exchange_by_id(&self, exchange_id: i32) -> Result<Option<Exchange>>;
    async fn get_exchange_by_code(&self, code: &str) -> Result<Option<Exchange>>;
    async fn get_exchanges(&self) -> Result<Vec<Exchange>>;
    
    // 視圖相關操作
    async fn get_future_complete_by_id(&self, instrument_id: i32) -> Result<Option<FutureComplete>>;
    async fn get_future_complete_list(&self, page: PageQuery) -> Result<Page<FutureComplete>>;
    
    // 泛型視圖查詢方法
    async fn get_view_by_id<T>(&self, view_type: ViewType, instrument_id: i32) -> Result<Option<T>>
        where T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + Debug;
    
    async fn get_view_list<T>(&self, view_type: ViewType, page: PageQuery) -> Result<Page<T>>
        where T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + Debug;
    
    // 新增的視圖方法，委派給泛型方法
    async fn get_stock_complete_by_id(&self, instrument_id: i32) -> Result<Option<StockComplete>> {
        self.get_view_by_id::<StockComplete>(ViewType::StockComplete, instrument_id).await
    }

    async fn get_stock_complete_list(&self, page: PageQuery) -> Result<Page<StockComplete>> {
        self.get_view_list::<StockComplete>(ViewType::StockComplete, page).await
    }
    
    async fn get_option_complete_by_id(&self, instrument_id: i32) -> Result<Option<OptionComplete>> {
        self.get_view_by_id::<OptionComplete>(ViewType::OptionComplete, instrument_id).await
    }

    async fn get_option_complete_list(&self, page: PageQuery) -> Result<Page<OptionComplete>> {
        self.get_view_list::<OptionComplete>(ViewType::OptionComplete, page).await
    }
    
    async fn get_forex_complete_by_id(&self, instrument_id: i32) -> Result<Option<ForexComplete>> {
        self.get_view_by_id::<ForexComplete>(ViewType::ForexComplete, instrument_id).await
    }

    async fn get_forex_complete_list(&self, page: PageQuery) -> Result<Page<ForexComplete>> {
        self.get_view_list::<ForexComplete>(ViewType::ForexComplete, page).await
    }
    
    async fn get_crypto_complete_by_id(&self, instrument_id: i32) -> Result<Option<CryptoComplete>> {
        self.get_view_by_id::<CryptoComplete>(ViewType::CryptoComplete, instrument_id).await
    }

    async fn get_crypto_complete_list(&self, page: PageQuery) -> Result<Page<CryptoComplete>> {
        self.get_view_list::<CryptoComplete>(ViewType::CryptoComplete, page).await
    }
}

/// PostgreSQL 金融商品儲存庫實現
pub struct PgInstrumentRepository {
    pool: PgPool,
}

impl PgInstrumentRepository {
    /// 創建新的 PostgreSQL 金融商品儲存庫實例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl DbExecutor for PgInstrumentRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl InstrumentRepository for PgInstrumentRepository {
    async fn get_instrument_by_id(&self, instrument_id: i32) -> Result<Option<Instrument>> {
        let instrument = sqlx::query_as::<_, Instrument>(
            "SELECT * FROM instrument WHERE instrument_id = $1"
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(instrument)
    }

    async fn get_instrument_by_symbol(&self, symbol: &str, exchange_id: Option<i32>, instrument_type: Option<&str>) -> Result<Option<Instrument>> {
        let instrument = match (exchange_id, instrument_type) {
            (Some(exchange_id), Some(instrument_type)) => {
                sqlx::query_as::<_, Instrument>(
                    "SELECT * FROM instrument WHERE symbol = $1 AND exchange_id = $2 AND instrument_type = $3"
                )
                .bind(symbol)
                .bind(exchange_id)
                .bind(instrument_type)
                .fetch_optional(self.get_pool())
                .await?
            },
            (Some(exchange_id), None) => {
                sqlx::query_as::<_, Instrument>(
                    "SELECT * FROM instrument WHERE symbol = $1 AND exchange_id = $2"
                )
                .bind(symbol)
                .bind(exchange_id)
                .fetch_optional(self.get_pool())
                .await?
            },
            (None, Some(instrument_type)) => {
                sqlx::query_as::<_, Instrument>(
                    "SELECT * FROM instrument WHERE symbol = $1 AND instrument_type = $2 ORDER BY is_active DESC LIMIT 1"
                )
                .bind(symbol)
                .bind(instrument_type)
                .fetch_optional(self.get_pool())
                .await?
            },
            (None, None) => {
                sqlx::query_as::<_, Instrument>(
                    "SELECT * FROM instrument WHERE symbol = $1 ORDER BY is_active DESC LIMIT 1"
                )
                .bind(symbol)
                .fetch_optional(self.get_pool())
                .await?
            }
        };

        Ok(instrument)
    }

    async fn get_instruments(&self, instrument_type: Option<&str>, page: PageQuery) -> Result<Page<Instrument>> {
        let offset = (page.page - 1) * page.page_size;
        
        let (instruments, total) = match instrument_type {
            Some(instrument_type) => {
                let instruments = sqlx::query_as::<_, Instrument>(
                    "SELECT * FROM instrument 
                     WHERE instrument_type = $1
                     ORDER BY instrument_id
                     LIMIT $2 OFFSET $3"
                )
                .bind(instrument_type)
                .bind(page.page_size)
                .bind(offset)
                .fetch_all(self.get_pool())
                .await?;

                let total = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM instrument WHERE instrument_type = $1"
                )
                .bind(instrument_type)
                .fetch_one(self.get_pool())
                .await?;
                
                (instruments, total)
            },
            None => {
                let instruments = sqlx::query_as::<_, Instrument>(
                    "SELECT * FROM instrument
                     ORDER BY instrument_id
                     LIMIT $1 OFFSET $2"
                )
                .bind(page.page_size)
                .bind(offset)
                .fetch_all(self.get_pool())
                .await?;

                let total = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM instrument"
                )
                .fetch_one(self.get_pool())
                .await?;
                
                (instruments, total)
            }
        };

        Ok(Page::new(instruments, total, page.page, page.page_size))
    }
    
    async fn get_stock_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Stock>> {
        let instrument = sqlx::query!(
            "SELECT attributes FROM instrument WHERE instrument_id = $1 AND instrument_type = 'STOCK'",
            instrument_id
        )
        .fetch_optional(self.get_pool())
        .await?;
        
        if let Some(row) = instrument {
            let attrs = row.attributes;
            // 解析 JSON 值
            let stock = Stock {
                instrument_id,
                sector: attrs.get("sector").and_then(|v| v.as_str()).map(String::from),
                industry: attrs.get("industry").and_then(|v| v.as_str()).map(String::from),
                market_cap: attrs.get("market_cap").and_then(|v| v.as_str()).and_then(|s| Decimal::from_str(s).ok()),
                shares_outstanding: attrs.get("shares_outstanding").and_then(|v| v.as_i64()),
                free_float: attrs.get("free_float").and_then(|v| v.as_i64()),
                listing_date: attrs.get("listing_date").and_then(|v| v.as_str()).and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                delisting_date: attrs.get("delisting_date").and_then(|v| v.as_str()).and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                dividend_yield: attrs.get("dividend_yield").and_then(|v| v.as_str()).and_then(|s| Decimal::from_str(s).ok()),
                pe_ratio: attrs.get("pe_ratio").and_then(|v| v.as_str()).and_then(|s| Decimal::from_str(s).ok()),
            };
            return Ok(Some(stock));
        }
        
        Ok(None)
    }
    
    async fn get_future_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Future>> {
        let instrument = sqlx::query!(
            "SELECT attributes FROM instrument WHERE instrument_id = $1 AND instrument_type = 'FUTURE'",
            instrument_id
        )
        .fetch_optional(self.get_pool())
        .await?;
        
        if let Some(row) = instrument {
            let _attrs = row.attributes;
            // 使用類似 get_stock_by_instrument_id 的方式從 JSON 中提取資料
            // 具體實作省略，需要參考 Future 結構的欄位
            return Err(anyhow!("Future attributes extraction not implemented yet"));
        }
        
        Ok(None)
    }
    
    async fn get_option_contract_by_instrument_id(&self, instrument_id: i32) -> Result<Option<OptionContract>> {
        let instrument = sqlx::query!(
            "SELECT attributes FROM instrument WHERE instrument_id = $1 AND instrument_type = 'OPTIONCONTRACT'",
            instrument_id
        )
        .fetch_optional(self.get_pool())
        .await?;
        
        if let Some(row) = instrument {
            let _attrs = row.attributes;
            // 使用類似 get_stock_by_instrument_id 的方式從 JSON 中提取資料
            // 具體實作省略，需要參考 OptionContract 結構的欄位
            return Err(anyhow!("OptionContract attributes extraction not implemented yet"));
        }
        
        Ok(None)
    }
    
    async fn get_forex_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Forex>> {
        let instrument = sqlx::query!(
            "SELECT attributes FROM instrument WHERE instrument_id = $1 AND instrument_type = 'FOREX'",
            instrument_id
        )
        .fetch_optional(self.get_pool())
        .await?;
        
        if let Some(row) = instrument {
            let _attrs = row.attributes;
            // 使用類似 get_stock_by_instrument_id 的方式從 JSON 中提取資料
            // 具體實作省略，需要參考 Forex 結構的欄位
            return Err(anyhow!("Forex attributes extraction not implemented yet"));
        }
        
        Ok(None)
    }
    
    async fn get_crypto_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Crypto>> {
        let instrument = sqlx::query!(
            "SELECT attributes FROM instrument WHERE instrument_id = $1 AND instrument_type = 'CRYPTO'",
            instrument_id
        )
        .fetch_optional(self.get_pool())
        .await?;
        
        if let Some(row) = instrument {
            let _attrs = row.attributes;
            // 使用類似 get_stock_by_instrument_id 的方式從 JSON 中提取資料
            // 具體實作省略，需要參考 Crypto 結構的欄位
            return Err(anyhow!("Crypto attributes extraction not implemented yet"));
        }
        
        Ok(None)
    }
    
    async fn get_exchange_by_id(&self, exchange_id: i32) -> Result<Option<Exchange>> {
        let exchange = sqlx::query_as::<_, Exchange>(
            "SELECT * FROM exchange WHERE exchange_id = $1"
        )
        .bind(exchange_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(exchange)
    }
    
    async fn get_exchange_by_code(&self, code: &str) -> Result<Option<Exchange>> {
        let exchange = sqlx::query_as::<_, Exchange>(
            "SELECT * FROM exchange WHERE code = $1"
        )
        .bind(code)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(exchange)
    }
    
    async fn get_exchanges(&self) -> Result<Vec<Exchange>> {
        let exchanges = sqlx::query_as::<_, Exchange>(
            "SELECT * FROM exchange ORDER BY code"
        )
        .fetch_all(self.get_pool())
        .await?;

        Ok(exchanges)
    }
    
    async fn get_or_create_instrument(&self, symbol: &str, asset_type: AssetType) -> Result<i32> {
        // 首先嘗試獲取已存在的商品
        let instrument_type = match asset_type {
            AssetType::Stock => "STOCK",
            AssetType::Future => "FUTURE",
            AssetType::Forex => "FOREX", 
            AssetType::Crypto => "CRYPTO",
            AssetType::OptionContract => "OPTIONCONTRACT",
        };
        
        if let Some(instrument) = self.get_instrument_by_symbol(symbol, None, Some(instrument_type)).await? {
            return Ok(instrument.instrument_id);
        }
        
        // 如果不存在，則創建新的商品
        let now = Utc::now();
        let instrument = Instrument {
            instrument_id: 0,
            symbol: symbol.to_string(),
            name: symbol.to_string(),
            instrument_type: instrument_type.to_string(),
            exchange_id: None,
            description: None,
            currency: "USD".to_string(),
            tick_size: None,
            lot_size: None,
            is_active: true,
            trading_start_date: None,
            trading_end_date: None,
            attributes: None,
            created_at: now,
            updated_at: now,
        };
        
        let id = self.insert_instrument(&instrument).await?;
        Ok(id)
    }
    
    async fn insert_instrument(&self, instrument: &Instrument) -> Result<i32> {
        let instrument_id = sqlx::query_scalar!(
            r#"
            INSERT INTO instrument (
                symbol, name, instrument_type, exchange_id, is_active,
                description, currency, tick_size, lot_size,
                trading_start_date, trading_end_date, attributes, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            RETURNING instrument_id
            "#,
            instrument.symbol,
            instrument.name,
            instrument.instrument_type,
            instrument.exchange_id,
            instrument.is_active,
            instrument.description,
            instrument.currency,
            instrument.tick_size,
            instrument.lot_size,
            instrument.trading_start_date,
            instrument.trading_end_date,
            instrument.attributes.as_ref().map(|j| j.0.clone()),
            instrument.created_at,
            instrument.updated_at
        )
        .fetch_one(self.get_pool())
        .await?;

        Ok(instrument_id)
    }

    async fn update_instrument(&self, instrument: &Instrument) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE instrument
            SET 
                symbol = $1,
                name = $2,
                instrument_type = $3,
                exchange_id = $4,
                is_active = $5,
                description = $6,
                currency = $7,
                tick_size = $8,
                lot_size = $9,
                trading_start_date = $10,
                trading_end_date = $11,
                attributes = $12,
                updated_at = $13
            WHERE instrument_id = $14
            "#,
            instrument.symbol,
            instrument.name,
            instrument.instrument_type,
            instrument.exchange_id,
            instrument.is_active,
            instrument.description,
            instrument.currency,
            instrument.tick_size,
            instrument.lot_size,
            instrument.trading_start_date,
            instrument.trading_end_date,
            instrument.attributes.as_ref().map(|j| j.0.clone()),
            instrument.updated_at,
            instrument.instrument_id
        )
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No instrument found with instrument ID: {}", instrument.instrument_id));
        }

        Ok(())
    }
    
    async fn insert_stock(&self, stock: &Stock) -> Result<()> {
        // 將 Stock 資料轉換為 JSON 並存入 attributes 欄位
        let mut attributes = serde_json::Map::new();
        if let Some(sector) = &stock.sector {
            attributes.insert("sector".to_string(), serde_json::Value::String(sector.clone()));
        }
        if let Some(industry) = &stock.industry {
            attributes.insert("industry".to_string(), serde_json::Value::String(industry.clone()));
        }
        if let Some(market_cap) = stock.market_cap {
            attributes.insert("market_cap".to_string(), serde_json::Value::String(market_cap.to_string()));
        }
        if let Some(shares_outstanding) = stock.shares_outstanding {
            attributes.insert("shares_outstanding".to_string(), serde_json::Value::Number(serde_json::Number::from(shares_outstanding)));
        }
        if let Some(free_float) = stock.free_float {
            attributes.insert("free_float".to_string(), serde_json::Value::Number(serde_json::Number::from(free_float)));
        }
        if let Some(listing_date) = stock.listing_date {
            attributes.insert("listing_date".to_string(), serde_json::Value::String(listing_date.format("%Y-%m-%d").to_string()));
        }
        if let Some(delisting_date) = stock.delisting_date {
            attributes.insert("delisting_date".to_string(), serde_json::Value::String(delisting_date.format("%Y-%m-%d").to_string()));
        }
        if let Some(dividend_yield) = stock.dividend_yield {
            attributes.insert("dividend_yield".to_string(), serde_json::Value::String(dividend_yield.to_string()));
        }
        if let Some(pe_ratio) = stock.pe_ratio {
            attributes.insert("pe_ratio".to_string(), serde_json::Value::String(pe_ratio.to_string()));
        }
        
        // 更新 instrument 表中的 attributes 欄位
        sqlx::query!(
            r#"
            UPDATE instrument
            SET attributes = $1, updated_at = $2
            WHERE instrument_id = $3 AND instrument_type = 'STOCK'
            "#,
            serde_json::Value::Object(attributes),
            Utc::now(),
            stock.instrument_id
        )
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn update_stock(&self, stock: &Stock) -> Result<()> {
        // 與 insert_stock 相同，使用 UPDATE 語句更新 attributes
        let mut attributes = serde_json::Map::new();
        if let Some(sector) = &stock.sector {
            attributes.insert("sector".to_string(), serde_json::Value::String(sector.clone()));
        }
        if let Some(industry) = &stock.industry {
            attributes.insert("industry".to_string(), serde_json::Value::String(industry.clone()));
        }
        if let Some(market_cap) = stock.market_cap {
            attributes.insert("market_cap".to_string(), serde_json::Value::String(market_cap.to_string()));
        }
        if let Some(shares_outstanding) = stock.shares_outstanding {
            attributes.insert("shares_outstanding".to_string(), serde_json::Value::Number(serde_json::Number::from(shares_outstanding)));
        }
        if let Some(free_float) = stock.free_float {
            attributes.insert("free_float".to_string(), serde_json::Value::Number(serde_json::Number::from(free_float)));
        }
        if let Some(listing_date) = stock.listing_date {
            attributes.insert("listing_date".to_string(), serde_json::Value::String(listing_date.format("%Y-%m-%d").to_string()));
        }
        if let Some(delisting_date) = stock.delisting_date {
            attributes.insert("delisting_date".to_string(), serde_json::Value::String(delisting_date.format("%Y-%m-%d").to_string()));
        }
        if let Some(dividend_yield) = stock.dividend_yield {
            attributes.insert("dividend_yield".to_string(), serde_json::Value::String(dividend_yield.to_string()));
        }
        if let Some(pe_ratio) = stock.pe_ratio {
            attributes.insert("pe_ratio".to_string(), serde_json::Value::String(pe_ratio.to_string()));
        }
        
        let rows_affected = sqlx::query!(
            r#"
            UPDATE instrument
            SET attributes = $1, updated_at = $2
            WHERE instrument_id = $3 AND instrument_type = 'STOCK'
            "#,
            serde_json::Value::Object(attributes),
            Utc::now(),
            stock.instrument_id
        )
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No stock found with instrument ID: {}", stock.instrument_id));
        }

        Ok(())
    }
    
    async fn insert_future(&self, _future: &Future) -> Result<()> {
        // 將 Future 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 insert_stock 類似，需要參考 Future 結構的欄位
        return Err(anyhow!("Future attributes update not implemented yet"));
    }

    async fn update_future(&self, _future: &Future) -> Result<()> {
        // 將 Future 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 update_stock 類似，需要參考 Future 結構的欄位
        return Err(anyhow!("Future attributes update not implemented yet"));
    }
    
    async fn insert_option_contract(&self, _option_contract: &OptionContract) -> Result<()> {
        // 將 OptionContract 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 insert_stock 類似，需要參考 OptionContract 結構的欄位
        return Err(anyhow!("OptionContract attributes update not implemented yet"));
    }

    async fn update_option_contract(&self, _option_contract: &OptionContract) -> Result<()> {
        // 將 OptionContract 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 update_stock 類似，需要參考 OptionContract 結構的欄位
        return Err(anyhow!("OptionContract attributes update not implemented yet"));
    }
    
    async fn insert_forex(&self, _forex: &Forex) -> Result<()> {
        // 將 Forex 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 insert_stock 類似，需要參考 Forex 結構的欄位
        return Err(anyhow!("Forex attributes update not implemented yet"));
    }

    async fn update_forex(&self, _forex: &Forex) -> Result<()> {
        // 將 Forex 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 update_stock 類似，需要參考 Forex 結構的欄位
        return Err(anyhow!("Forex attributes update not implemented yet"));
    }
    
    async fn insert_crypto(&self, _crypto: &Crypto) -> Result<()> {
        // 將 Crypto 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 insert_stock 類似，需要參考 Crypto 結構的欄位
        return Err(anyhow!("Crypto attributes update not implemented yet"));
    }

    async fn update_crypto(&self, _crypto: &Crypto) -> Result<()> {
        // 將 Crypto 特定欄位轉換為 JSON 並更新到 attributes
        // 具體實作與 update_stock 類似，需要參考 Crypto 結構的欄位
        return Err(anyhow!("Crypto attributes update not implemented yet"));
    }

    async fn get_future_complete_by_id(&self, instrument_id: i32) -> Result<Option<FutureComplete>> {
        self.get_view_by_id::<FutureComplete>(ViewType::FutureComplete, instrument_id).await
    }

    async fn get_future_complete_list(&self, page: PageQuery) -> Result<Page<FutureComplete>> {
        self.get_view_list::<FutureComplete>(ViewType::FutureComplete, page).await
    }

    async fn get_view_by_id<T>(&self, view_type: ViewType, instrument_id: i32) -> Result<Option<T>>
        where T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + Debug {
        let query = format!("SELECT * FROM {} WHERE instrument_id = $1", view_type.table_name());
        let view = sqlx::query_as::<_, T>(&query)
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(view)
    }

    async fn get_view_list<T>(&self, view_type: ViewType, page: PageQuery) -> Result<Page<T>>
        where T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + Debug {
        let offset = (page.page - 1) * page.page_size;
        
        let query = format!("SELECT * FROM {} ORDER BY instrument_id LIMIT $1 OFFSET $2", view_type.table_name());
        let views = sqlx::query_as::<_, T>(&query)
        .bind(page.page_size)
        .bind(offset)
        .fetch_all(self.get_pool())
        .await?;

        let count_query = format!("SELECT COUNT(*) FROM {}", view_type.table_name());
        let total = sqlx::query_scalar::<_, i64>(&count_query)
        .fetch_one(self.get_pool())
        .await?;

        Ok(Page::new(views, total, page.page, page.page_size))
    }
} 