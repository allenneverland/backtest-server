use anyhow::{anyhow, Result};
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domain_types::types::AssetType;
use crate::storage::models::instrument::*;
use crate::storage::repository::{DbExecutor, Page, PageQuery};

/// 金融商品數據庫操作
pub struct InstrumentRepository {
    pool: PgPool,
}

impl InstrumentRepository {
    /// 創建新的金融商品數據庫操作實例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 創建金融商品
    pub async fn create(&self, instrument: InstrumentInsert) -> Result<Instrument> {
        let now = Utc::now();

        let result = sqlx::query!(
            r#"
            INSERT INTO instrument (
                symbol, exchange_id, instrument_type, name, description,
                currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, attributes,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            RETURNING 
                instrument_id
            "#,
            instrument.symbol,
            instrument.exchange_id,
            instrument.instrument_type,
            instrument.name,
            instrument.description,
            instrument.currency,
            instrument.tick_size,
            instrument.lot_size,
            instrument.is_active,
            instrument.trading_start_date,
            instrument.trading_end_date,
            serde_json::Value::Object(serde_json::Map::new()) as _,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        let id = result.instrument_id;

        // 重新獲取完整的金融商品資訊
        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow!("Failed to retrieve newly created instrument"))
    }

    /// 創建股票及其特定屬性
    pub async fn create_stock(&self, instrument: InstrumentInsert, stock: StockInsert) -> Result<Instrument> {
        let created_instrument = self.create(instrument).await?;

        // 創建股票特定屬性
        let stock_with_id = StockInsert {
            instrument_id: created_instrument.instrument_id,
            ..stock
        };

        sqlx::query!(
            r#"
            INSERT INTO stock (
                instrument_id, sector, industry, market_cap, 
                shares_outstanding, free_float, listing_date, delisting_date,
                dividend_yield, pe_ratio
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
            )
            "#,
            stock_with_id.instrument_id,
            stock_with_id.sector,
            stock_with_id.industry,
            stock_with_id.market_cap,
            stock_with_id.shares_outstanding,
            stock_with_id.free_float,
            stock_with_id.listing_date,
            stock_with_id.delisting_date,
            stock_with_id.dividend_yield,
            stock_with_id.pe_ratio
        )
        .execute(&self.pool)
        .await?;

        Ok(created_instrument)
    }

    /// 創建期貨及其特定屬性
    pub async fn create_future(&self, instrument: InstrumentInsert, future: FutureInsert) -> Result<Instrument> {
        let created_instrument = self.create(instrument).await?;

        // 創建期貨特定屬性
        let future_with_id = FutureInsert {
            instrument_id: created_instrument.instrument_id,
            ..future
        };

        sqlx::query!(
            r#"
            INSERT INTO future (
                instrument_id, underlying_asset, contract_size, contract_unit,
                delivery_date, first_notice_date, last_trading_date,
                settlement_type, initial_margin, maintenance_margin, price_quotation
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            "#,
            future_with_id.instrument_id,
            future_with_id.underlying_asset,
            future_with_id.contract_size,
            future_with_id.contract_unit,
            future_with_id.delivery_date,
            future_with_id.first_notice_date,
            future_with_id.last_trading_date,
            future_with_id.settlement_type,
            future_with_id.initial_margin,
            future_with_id.maintenance_margin,
            future_with_id.price_quotation
        )
        .execute(&self.pool)
        .await?;

        Ok(created_instrument)
    }

    /// 創建選擇權及其特定屬性
    pub async fn create_option(&self, instrument: InstrumentInsert, option: OptionInsert) -> Result<Instrument> {
        let created_instrument = self.create(instrument).await?;

        // 創建選擇權特定屬性
        let option_with_id = OptionInsert {
            instrument_id: created_instrument.instrument_id,
            ..option
        };

        sqlx::query!(
            r#"
            INSERT INTO option_contract (
                instrument_id, underlying_instrument_id, option_type, strike_price,
                expiration_date, exercise_style, contract_size, implied_volatility,
                delta, gamma, theta, vega, rho
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
            "#,
            option_with_id.instrument_id,
            option_with_id.underlying_instrument_id,
            option_with_id.option_type,
            option_with_id.strike_price,
            option_with_id.expiration_date,
            option_with_id.exercise_style,
            option_with_id.contract_size,
            option_with_id.implied_volatility,
            option_with_id.delta,
            option_with_id.gamma,
            option_with_id.theta,
            option_with_id.vega,
            option_with_id.rho
        )
        .execute(&self.pool)
        .await?;

        Ok(created_instrument)
    }

    /// 創建外匯及其特定屬性
    pub async fn create_forex(&self, instrument: InstrumentInsert, forex: ForexInsert) -> Result<Instrument> {
        let created_instrument = self.create(instrument).await?;

        // 創建外匯特定屬性
        let forex_with_id = ForexInsert {
            instrument_id: created_instrument.instrument_id,
            ..forex
        };

        sqlx::query!(
            r#"
            INSERT INTO forex (
                instrument_id, base_currency, quote_currency, pip_value,
                typical_spread, margin_requirement, trading_hours
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7
            )
            "#,
            forex_with_id.instrument_id,
            forex_with_id.base_currency,
            forex_with_id.quote_currency,
            forex_with_id.pip_value,
            forex_with_id.typical_spread,
            forex_with_id.margin_requirement,
            forex_with_id.trading_hours
        )
        .execute(&self.pool)
        .await?;

        Ok(created_instrument)
    }

    /// 創建加密貨幣及其特定屬性
    pub async fn create_crypto(&self, instrument: InstrumentInsert, crypto: CryptoInsert) -> Result<Instrument> {
        let created_instrument = self.create(instrument).await?;

        // 創建加密貨幣特定屬性
        let crypto_with_id = CryptoInsert {
            instrument_id: created_instrument.instrument_id,
            ..crypto
        };

        sqlx::query!(
            r#"
            INSERT INTO crypto (
                instrument_id, blockchain_network, total_supply, circulating_supply,
                max_supply, mining_algorithm, consensus_mechanism,
                website_url, whitepaper_url, github_url
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
            )
            "#,
            crypto_with_id.instrument_id,
            crypto_with_id.blockchain_network,
            crypto_with_id.total_supply,
            crypto_with_id.circulating_supply,
            crypto_with_id.max_supply,
            crypto_with_id.mining_algorithm,
            crypto_with_id.consensus_mechanism,
            crypto_with_id.website_url,
            crypto_with_id.whitepaper_url,
            crypto_with_id.github_url
        )
        .execute(&self.pool)
        .await?;

        Ok(created_instrument)
    }

    /// 根據ID獲取金融商品
    pub async fn get_by_id(&self, instrument_id: i32) -> Result<Option<Instrument>> {
        let record = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            WHERE instrument_id = $1
            "#,
            instrument_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 根據交易代碼和交易所獲取金融商品
    pub async fn get_by_symbol_and_exchange(&self, symbol: &str, exchange_id: i32) -> Result<Option<Instrument>> {
        let record = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            WHERE symbol = $1 AND exchange_id = $2
            "#,
            symbol,
            exchange_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 根據交易代碼、交易所和類型獲取金融商品
    pub async fn get_by_symbol_exchange_and_type(
        &self, 
        symbol: &str, 
        exchange_id: i32, 
        instrument_type: &str
    ) -> Result<Option<Instrument>> {
        let record = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            WHERE symbol = $1 AND exchange_id = $2 AND instrument_type = $3
            "#,
            symbol,
            exchange_id,
            instrument_type
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 獲取特定交易所的所有金融商品
    pub async fn get_by_exchange(&self, exchange_id: i32) -> Result<Vec<Instrument>> {
        let records = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            WHERE exchange_id = $1
            ORDER BY symbol
            "#,
            exchange_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 獲取特定類型的所有金融商品
    pub async fn get_by_type(&self, instrument_type: &str) -> Result<Vec<Instrument>> {
        let records = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            WHERE instrument_type = $1
            ORDER BY symbol
            "#,
            instrument_type
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 獲取所有活躍的金融商品
    pub async fn get_active(&self) -> Result<Vec<Instrument>> {
        let records = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            WHERE is_active = true
            ORDER BY symbol
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 分頁獲取所有金融商品
    pub async fn get_all_paged(&self, page_query: PageQuery) -> Result<Page<Instrument>> {
        let offset = (page_query.page - 1) * page_query.page_size;

        // 獲取總記錄數
        let total_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM instrument")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        if total_count == 0 {
            return Ok(Page::empty(page_query.page, page_query.page_size));
        }

        // 獲取分頁數據
        let records = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            ORDER BY symbol
            LIMIT $1 OFFSET $2
            "#,
            page_query.page_size,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(Page::new(
            records,
            total_count,
            page_query.page,
            page_query.page_size,
        ))
    }

    /// 搜尋金融商品
    pub async fn search(&self, query: &str) -> Result<Vec<Instrument>> {
        let search_term = format!("%{}%", query);

        let records = sqlx::query_as!(
            Instrument,
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            FROM instrument
            WHERE symbol ILIKE $1 OR name ILIKE $1
            ORDER BY symbol
            LIMIT 100
            "#,
            search_term
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 更新金融商品
    pub async fn update(&self, instrument_id: i32, instrument: InstrumentInsert) -> Result<Instrument> {
        let now = Utc::now();

        let _result = sqlx::query!(
            r#"
            UPDATE instrument
            SET 
                symbol = $1,
                exchange_id = $2,
                instrument_type = $3,
                name = $4,
                description = $5,
                currency = $6,
                tick_size = $7,
                lot_size = $8,
                is_active = $9,
                trading_start_date = $10,
                trading_end_date = $11,
                updated_at = $12
            WHERE instrument_id = $13
            "#,
            instrument.symbol,
            instrument.exchange_id,
            instrument.instrument_type,
            instrument.name,
            instrument.description,
            instrument.currency,
            instrument.tick_size,
            instrument.lot_size,
            instrument.is_active,
            instrument.trading_start_date,
            instrument.trading_end_date,
            now,
            instrument_id
        )
        .execute(&self.pool)
        .await?;

        // 重新獲取更新後的金融商品資訊
        self.get_by_id(instrument_id)
            .await?
            .ok_or_else(|| anyhow!("Instrument not found after update"))
    }

    /// 更新股票特定屬性
    pub async fn update_stock(&self, stock: StockInsert) -> Result<Stock> {
        let result = sqlx::query_as!(
            Stock,
            r#"
            UPDATE stock
            SET 
                sector = $1,
                industry = $2,
                market_cap = $3,
                shares_outstanding = $4,
                free_float = $5,
                listing_date = $6,
                delisting_date = $7,
                dividend_yield = $8,
                pe_ratio = $9
            WHERE instrument_id = $10
            RETURNING 
                instrument_id, sector, industry, market_cap, 
                shares_outstanding, free_float, listing_date, delisting_date,
                dividend_yield, pe_ratio
            "#,
            stock.sector,
            stock.industry,
            stock.market_cap,
            stock.shares_outstanding,
            stock.free_float,
            stock.listing_date,
            stock.delisting_date,
            stock.dividend_yield,
            stock.pe_ratio,
            stock.instrument_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// 刪除金融商品
    pub async fn delete(&self, instrument_id: i32) -> Result<bool> {
        // 取得金融商品類型 (先查詢再删除，以便刪除對應子表中的數據)
        let instrument = self.get_by_id(instrument_id).await?;
        
        if let Some(instrument) = instrument {
            // 先刪除子表中的記錄
            match instrument.instrument_type.as_str() {
                "STOCK" => {
                    sqlx::query!("DELETE FROM stock WHERE instrument_id = $1", instrument_id)
                        .execute(&self.pool)
                        .await?;
                }
                "FUTURE" => {
                    sqlx::query!("DELETE FROM future WHERE instrument_id = $1", instrument_id)
                        .execute(&self.pool)
                        .await?;
                }
                "OPTIONCONTRACT" => {
                    sqlx::query!("DELETE FROM option_contract WHERE instrument_id = $1", instrument_id)
                        .execute(&self.pool)
                        .await?;
                }
                "FOREX" => {
                    sqlx::query!("DELETE FROM forex WHERE instrument_id = $1", instrument_id)
                        .execute(&self.pool)
                        .await?;
                }
                "CRYPTO" => {
                    sqlx::query!("DELETE FROM crypto WHERE instrument_id = $1", instrument_id)
                        .execute(&self.pool)
                        .await?;
                }
                _ => {}
            }
            
            // 再刪除主表記錄
            let result = sqlx::query!(
                r#"
                DELETE FROM instrument
                WHERE instrument_id = $1
                "#,
                instrument_id
            )
            .execute(&self.pool)
            .await?;
            
            Ok(result.rows_affected() > 0)
        } else {
            Ok(false)
        }
    }

    /// 獲取股票完整視圖（包含交易所資訊）
    pub async fn get_stock_complete(&self, instrument_id: i32) -> Result<Option<StockComplete>> {
        let record = sqlx::query_as!(
            StockComplete,
            r#"
            SELECT 
                i.instrument_id, i.symbol, i.exchange_id, i.instrument_type, i.name,
                i.description, i.currency, i.tick_size, i.lot_size, i.is_active,
                i.trading_start_date, i.trading_end_date, i.created_at, i.updated_at,
                e.code as exchange_code, e.name as exchange_name, e.country as exchange_country,
                s.sector, s.industry, s.market_cap, s.shares_outstanding, s.free_float,
                s.listing_date, s.delisting_date, s.dividend_yield, s.pe_ratio
            FROM instrument i
            JOIN exchange e ON i.exchange_id = e.exchange_id
            JOIN stock s ON i.instrument_id = s.instrument_id
            WHERE i.instrument_id = $1
            "#,
            instrument_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 獲取期貨完整視圖（包含交易所資訊）
    pub async fn get_future_complete(&self, instrument_id: i32) -> Result<Option<FutureComplete>> {
        let record = sqlx::query_as!(
            FutureComplete,
            r#"
            SELECT 
                i.instrument_id, i.symbol, i.exchange_id, i.instrument_type, i.name,
                i.description, i.currency, i.tick_size, i.lot_size, i.is_active,
                i.trading_start_date, i.trading_end_date, i.created_at, i.updated_at,
                e.code as exchange_code, e.name as exchange_name, e.country as exchange_country,
                f.underlying_asset, f.contract_size, f.contract_unit,
                f.delivery_date, f.first_notice_date, f.last_trading_date,
                f.settlement_type, f.initial_margin, f.maintenance_margin, f.price_quotation
            FROM instrument i
            JOIN exchange e ON i.exchange_id = e.exchange_id
            JOIN future f ON i.instrument_id = f.instrument_id
            WHERE i.instrument_id = $1
            "#,
            instrument_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 在事務中創建金融商品
    pub async fn create_in_tx<'a>(
        &self,
        tx: &mut Transaction<'a, Postgres>,
        instrument: InstrumentInsert,
    ) -> Result<Instrument> {
        let now = Utc::now();

        let result = sqlx::query!(
            r#"
            INSERT INTO instrument (
                symbol, exchange_id, instrument_type, name, description,
                currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, attributes,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            RETURNING 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, attributes as "attributes: Json<serde_json::Value>",
                created_at, updated_at
            "#,
            instrument.symbol,
            instrument.exchange_id,
            instrument.instrument_type,
            instrument.name,
            instrument.description,
            instrument.currency,
            instrument.tick_size,
            instrument.lot_size,
            instrument.is_active,
            instrument.trading_start_date,
            instrument.trading_end_date,
            serde_json::Value::Object(serde_json::Map::new()) as _,
            now,
            now
        )
        .fetch_one(&mut **tx)
        .await?;

        Ok(Instrument {
            instrument_id: result.instrument_id,
            symbol: result.symbol,
            exchange_id: result.exchange_id,
            instrument_type: result.instrument_type,
            name: result.name,
            description: result.description,
            currency: result.currency,
            tick_size: result.tick_size,
            lot_size: result.lot_size,
            is_active: result.is_active,
            trading_start_date: result.trading_start_date,
            trading_end_date: result.trading_end_date,
            attributes: result.attributes,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }

    /// 將域模型轉換為數據庫模型
    pub fn domain_to_db_model(
        domain_instrument: &crate::domain_types::instrument::Instrument,
        exchange_id: Option<i32>,
    ) -> InstrumentInsert {
        // 將域模型類型轉換為數據庫模型類型
        let instrument_type = match domain_instrument.asset_type {
            AssetType::Stock => "STOCK",
            AssetType::Future => "FUTURE",
            AssetType::Option => "OPTIONCONTRACT",
            AssetType::Forex => "FOREX",
            AssetType::Crypto => "CRYPTO",
        };

        // 從UTC DateTime轉換為NaiveDate
        let trading_start_date = domain_instrument
            .listing_date
            .map(|dt| dt.naive_utc().date());
        
        let trading_end_date = domain_instrument
            .expiry_date
            .map(|dt| dt.naive_utc().date());

        // 轉換為數據庫模型
        InstrumentInsert {
            symbol: domain_instrument.symbol.clone(),
            exchange_id,
            instrument_type: instrument_type.to_string(),
            name: domain_instrument.name.clone(),
            description: domain_instrument.description.clone(),
            currency: domain_instrument.currency.clone(),
            tick_size: Some(Decimal::try_from(domain_instrument.tick_size).unwrap_or_default()),
            lot_size: Some(domain_instrument.lot_size as i32),
            is_active: domain_instrument.is_active,
            trading_start_date,
            trading_end_date,
        }
    }

    /// 將數據庫模型轉換為域模型
    pub fn db_to_domain_model(
        db_instrument: &Instrument,
        exchange_code: Option<String>,
    ) -> Result<crate::domain_types::instrument::Instrument> {
        // 將數據庫模型類型轉換為域模型類型
        let asset_type = match db_instrument.instrument_type.as_str() {
            "STOCK" => AssetType::Stock,
            "FUTURE" => AssetType::Future,
            "OPTIONCONTRACT" => AssetType::Option,
            "FOREX" => AssetType::Forex,
            "CRYPTO" => AssetType::Crypto,
            _ => return Err(anyhow!("Unknown instrument type: {}", db_instrument.instrument_type)),
        };

        // 從NaiveDate轉換為UTC DateTime
        let listing_date = db_instrument.trading_start_date.map(|d| {
            use chrono::TimeZone;
            Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
        });
        
        let expiry_date = db_instrument.trading_end_date.map(|d| {
            use chrono::TimeZone;
            Utc.from_utc_datetime(&d.and_hms_opt(23, 59, 59).unwrap())
        });

        // 提取屬性
        let attributes = db_instrument
            .attributes
            .clone()
            .map(|a| a.0)
            .unwrap_or_else(|| serde_json::Value::Null);

        // 構建域模型
        let instrument = crate::domain_types::instrument::Instrument {
            instrument_id: db_instrument.instrument_id.to_string(),
            symbol: db_instrument.symbol.clone(),
            exchange: exchange_code.unwrap_or_else(|| "UNKNOWN".to_string()),
            asset_type,
            name: db_instrument.name.clone(),
            description: db_instrument.description.clone(),
            is_active: db_instrument.is_active,
            currency: db_instrument.currency.clone(),
            listing_date,
            expiry_date,
            lot_size: db_instrument.lot_size.unwrap_or(1) as f64,
            tick_size: db_instrument.tick_size.map(|d| d.to_f64().unwrap_or(0.01)).unwrap_or(0.01),
            created_at: db_instrument.created_at,
            updated_at: db_instrument.updated_at,
            attributes,
        };

        Ok(instrument)
    }
}

impl DbExecutor for InstrumentRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::storage::models::instrument::{
        InstrumentInsert, StockInsert, FutureInsert, 
        OptionInsert, ForexInsert, CryptoInsert
    };
    use crate::domain_types::types::AssetType;
    use crate::domain_types::instrument::{
        Instrument as DomainInstrument, StockAttributes, FutureAttributes,
        OptionAttributes, ForexAttributes, CryptoAttributes
    };
    use std::str::FromStr;
    use chrono::NaiveDate;
    use sqlx::postgres::PgPoolOptions;

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/testdb".to_string());
        
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database")
    }

    #[sqlx::test]
    async fn test_create_and_get_instrument() -> Result<()> {
        let pool = setup_test_db().await;
        let repo = InstrumentRepository::new(pool);
        
        // 創建基本的金融商品
        let instrument = InstrumentInsert {
            symbol: "AAPL".to_string(),
            exchange_id: Some(1),  // 假設交易所ID為1
            instrument_type: "STOCK".to_string(),
            name: "Apple Inc.".to_string(),
            description: Some("Apple Inc. is an American multinational technology company.".to_string()),
            currency: "USD".to_string(),
            tick_size: Some(dec!(0.01)),
            lot_size: Some(100),
            is_active: true,
            trading_start_date: Some(NaiveDate::from_str("1980-12-12").unwrap()),
            trading_end_date: None,
        };
        
        // 創建金融商品
        let created = repo.create(instrument).await?;
        
        // 確認創建成功
        assert_eq!(created.symbol, "AAPL");
        assert_eq!(created.name, "Apple Inc.");
        assert_eq!(created.instrument_type, "STOCK");
        
        // 獲取金融商品
        let fetched = repo.get_by_id(created.instrument_id).await?;
        
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        
        assert_eq!(fetched.symbol, "AAPL");
        assert_eq!(fetched.name, "Apple Inc.");
        
        // 清理測試數據
        repo.delete(created.instrument_id).await?;
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_stock_with_attributes() -> Result<()> {
        let pool = setup_test_db().await;
        let repo = InstrumentRepository::new(pool);
        
        // 創建基本的金融商品
        let instrument = InstrumentInsert {
            symbol: "MSFT".to_string(),
            exchange_id: Some(1),
            instrument_type: "STOCK".to_string(),
            name: "Microsoft Corporation".to_string(),
            description: Some("Microsoft Corporation is an American multinational technology company.".to_string()),
            currency: "USD".to_string(),
            tick_size: Some(dec!(0.01)),
            lot_size: Some(100),
            is_active: true,
            trading_start_date: Some(NaiveDate::from_str("1986-03-13").unwrap()),
            trading_end_date: None,
        };
        
        // 創建股票特定屬性
        let stock_attrs = StockInsert {
            instrument_id: 0,  // 會在創建時被覆蓋
            sector: Some("Technology".to_string()),
            industry: Some("Software".to_string()),
            market_cap: Some(dec!(1800000000000)),
            shares_outstanding: Some(7_500_000_000),
            free_float: Some(7_450_000_000),
            listing_date: Some(NaiveDate::from_str("1986-03-13").unwrap()),
            delisting_date: None,
            dividend_yield: Some(dec!(0.9)),
            pe_ratio: Some(dec!(30.5)),
        };
        
        // 創建股票及其特定屬性
        let created = repo.create_stock(instrument, stock_attrs).await?;
        
        // 獲取完整的股票信息
        let stock_complete = repo.get_stock_complete(created.instrument_id).await?;
        
        assert!(stock_complete.is_some());
        let stock_complete = stock_complete.unwrap();
        
        assert_eq!(stock_complete.symbol, "MSFT");
        assert_eq!(stock_complete.sector, Some("Technology".to_string()));
        assert_eq!(stock_complete.industry, Some("Software".to_string()));
        
        // 清理測試數據
        repo.delete(created.instrument_id).await?;
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_domain_model_conversion() -> Result<()> {
        // 創建域模型實例
        let stock_attrs = StockAttributes {
            sector: Some("Technology".to_string()),
            industry: Some("Electronics".to_string()),
            market_cap: Some(2500000000000.0),
            is_etf: false,
            dividend_yield: Some(0.5),
        };

        let domain_instrument = DomainInstrument::builder()
            .instrument_id("AAPL123")
            .symbol("AAPL")
            .exchange("NASDAQ")
            .asset_type(AssetType::Stock)
            .name("Apple Inc.")
            .description("Apple Inc. designs, manufactures, and markets smartphones.")
            .currency("USD")
            .tick_size(0.01)
            .lot_size(100.0)
            .stock_attributes(stock_attrs)
            .build()
            .unwrap();

        // 轉換為數據庫模型
        let db_model = InstrumentRepository::domain_to_db_model(&domain_instrument, Some(1));
        
        assert_eq!(db_model.symbol, "AAPL");
        assert_eq!(db_model.name, "Apple Inc.");
        assert_eq!(db_model.instrument_type, "STOCK");
        assert_eq!(db_model.exchange_id, Some(1));
        
        // 創建模擬的數據庫實例以測試反向轉換
        let now = Utc::now();
        let db_instance = Instrument {
            instrument_id: 1,
            symbol: "AAPL".to_string(),
            exchange_id: Some(1),
            instrument_type: "STOCK".to_string(),
            name: "Apple Inc.".to_string(),
            description: Some("Apple Inc. designs, manufactures, and markets smartphones.".to_string()),
            currency: "USD".to_string(),
            tick_size: Some(dec!(0.01)),
            lot_size: Some(100),
            is_active: true,
            trading_start_date: Some(NaiveDate::from_str("1980-12-12").unwrap()),
            trading_end_date: None,
            attributes: Some(sqlx::types::Json(serde_json::Value::Null)),
            created_at: now,
            updated_at: now,
        };
        
        // 轉換回域模型
        let converted_domain = InstrumentRepository::db_to_domain_model(&db_instance, Some("NASDAQ".to_string()))?;
        
        assert_eq!(converted_domain.instrument_id, "1");
        assert_eq!(converted_domain.symbol, "AAPL");
        assert_eq!(converted_domain.exchange, "NASDAQ");
        assert_eq!(converted_domain.asset_type, AssetType::Stock);
        
        Ok(())
    }
}