use anyhow::{anyhow, Result};
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Row, Transaction};

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

        // 計算屬性 JSON 值
        let attributes_json = match &instrument.attributes {
            Some(json_val) => json_val.0.clone(),
            None => Value::Object(serde_json::Map::new()),
        };

        let result = sqlx::query(
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
        )
        .bind(&instrument.symbol)
        .bind(&instrument.exchange_id)
        .bind(&instrument.instrument_type)
        .bind(&instrument.name)
        .bind(&instrument.description)
        .bind(&instrument.currency)
        .bind(&instrument.tick_size)
        .bind(&instrument.lot_size)
        .bind(&instrument.is_active)
        .bind(&instrument.trading_start_date)
        .bind(&instrument.trading_end_date)
        .bind(&attributes_json)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;

        let id: i32 = result.get("instrument_id");

        // 重新獲取完整的金融商品資訊
        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow!("Failed to retrieve newly created instrument"))
    }

    /// 創建股票及其特定屬性
    pub async fn create_stock(
        &self,
        mut instrument: InstrumentInsert,
        stock_attrs: StockAttributes,
    ) -> Result<Instrument> {
        // 將股票特定屬性轉換為JSON
        let attrs_json = serde_json::to_value(stock_attrs)?;
        instrument.attributes = Some(Json(attrs_json));

        // 創建金融商品
        self.create(instrument).await
    }

    /// 創建期貨及其特定屬性
    pub async fn create_future(
        &self,
        mut instrument: InstrumentInsert,
        future_attrs: FutureAttributes,
    ) -> Result<Instrument> {
        // 將期貨特定屬性轉換為JSON
        let attrs_json = serde_json::to_value(future_attrs)?;
        instrument.attributes = Some(Json(attrs_json));

        // 創建金融商品
        self.create(instrument).await
    }

    /// 創建選擇權及其特定屬性
    pub async fn create_option(
        &self,
        mut instrument: InstrumentInsert,
        option_attrs: OptionAttributes,
    ) -> Result<Instrument> {
        // 將選擇權特定屬性轉換為JSON
        let attrs_json = serde_json::to_value(option_attrs)?;
        instrument.attributes = Some(Json(attrs_json));

        // 創建金融商品
        self.create(instrument).await
    }

    /// 創建外匯及其特定屬性
    pub async fn create_forex(
        &self,
        mut instrument: InstrumentInsert,
        forex_attrs: ForexAttributes,
    ) -> Result<Instrument> {
        // 將外匯特定屬性轉換為JSON
        let attrs_json = serde_json::to_value(forex_attrs)?;
        instrument.attributes = Some(Json(attrs_json));

        // 創建金融商品
        self.create(instrument).await
    }

    /// 創建加密貨幣及其特定屬性
    pub async fn create_crypto(
        &self,
        mut instrument: InstrumentInsert,
        crypto_attrs: CryptoAttributes,
    ) -> Result<Instrument> {
        // 將加密貨幣特定屬性轉換為JSON
        let attrs_json = serde_json::to_value(crypto_attrs)?;
        instrument.attributes = Some(Json(attrs_json));

        // 創建金融商品
        self.create(instrument).await
    }

    /// 根據ID獲取金融商品
    pub async fn get_by_id(&self, instrument_id: i32) -> Result<Option<Instrument>> {
        let record = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            WHERE instrument_id = $1
            "#,
        )
        .bind(instrument_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 根據交易代碼和交易所獲取金融商品
    pub async fn get_by_symbol_and_exchange(
        &self,
        symbol: &str,
        exchange_id: i32,
    ) -> Result<Option<Instrument>> {
        let record = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            WHERE symbol = $1 AND exchange_id = $2
            "#,
        )
        .bind(symbol)
        .bind(exchange_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 根據交易代碼、交易所和類型獲取金融商品
    pub async fn get_by_symbol_exchange_and_type(
        &self,
        symbol: &str,
        exchange_id: i32,
        instrument_type: &str,
    ) -> Result<Option<Instrument>> {
        let record = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            WHERE symbol = $1 AND exchange_id = $2 AND instrument_type = $3
            "#,
        )
        .bind(symbol)
        .bind(exchange_id)
        .bind(instrument_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 獲取特定交易所的所有金融商品
    pub async fn get_by_exchange(&self, exchange_id: i32) -> Result<Vec<Instrument>> {
        let records = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            WHERE exchange_id = $1
            ORDER BY symbol
            "#,
        )
        .bind(exchange_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 獲取特定類型的所有金融商品
    pub async fn get_by_type(&self, instrument_type: &str) -> Result<Vec<Instrument>> {
        let records = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            WHERE instrument_type = $1
            ORDER BY symbol
            "#,
        )
        .bind(instrument_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 獲取所有活躍的金融商品
    pub async fn get_active(&self) -> Result<Vec<Instrument>> {
        let records = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            WHERE is_active = true
            ORDER BY symbol
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 分頁獲取所有金融商品
    pub async fn get_all_paged(&self, page_query: PageQuery) -> Result<Page<Instrument>> {
        let offset = (page_query.page - 1) * page_query.page_size;

        // 獲取總記錄數
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM instrument")
            .fetch_one(&self.pool)
            .await?;

        if total_count == 0 {
            return Ok(Page::empty(page_query.page, page_query.page_size));
        }

        // 獲取分頁數據
        let records = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            ORDER BY symbol
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(page_query.page_size)
        .bind(offset)
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

        let records = sqlx::query_as::<_, Instrument>(
            r#"
            SELECT 
                instrument_id, symbol, exchange_id, instrument_type, name, 
                description, currency, tick_size, lot_size, is_active, 
                trading_start_date, trading_end_date, 
                attributes, created_at, updated_at
            FROM instrument
            WHERE symbol ILIKE $1 OR name ILIKE $1
            ORDER BY symbol
            LIMIT 100
            "#,
        )
        .bind(search_term)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 更新金融商品
    pub async fn update(
        &self,
        instrument_id: i32,
        instrument: InstrumentInsert,
    ) -> Result<Instrument> {
        let now = Utc::now();

        let _result = sqlx::query(
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
                attributes = $12,
                updated_at = $13
            WHERE instrument_id = $14
            "#,
        )
        .bind(&instrument.symbol)
        .bind(&instrument.exchange_id)
        .bind(&instrument.instrument_type)
        .bind(&instrument.name)
        .bind(&instrument.description)
        .bind(&instrument.currency)
        .bind(&instrument.tick_size)
        .bind(&instrument.lot_size)
        .bind(&instrument.is_active)
        .bind(&instrument.trading_start_date)
        .bind(&instrument.trading_end_date)
        .bind(instrument.attributes.as_ref().map(|attr| &attr.0))
        .bind(&now)
        .bind(instrument_id)
        .execute(&self.pool)
        .await?;

        // 重新獲取更新後的金融商品資訊
        self.get_by_id(instrument_id)
            .await?
            .ok_or_else(|| anyhow!("Instrument not found after update"))
    }

    /// 更新股票特定屬性
    pub async fn update_stock_attributes(
        &self,
        instrument_id: i32,
        attrs: StockAttributes,
    ) -> Result<Instrument> {
        // 获取当前金融商品
        let mut instrument = self
            .get_by_id(instrument_id)
            .await?
            .ok_or_else(|| anyhow!("Instrument not found"))?;

        // 確保是股票類型
        if instrument.instrument_type != "STOCK" {
            return Err(anyhow!("Instrument is not a stock"));
        }

        // 設置新的屬性
        instrument.set_stock_attributes(attrs)?;

        // 更新屬性
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE instrument
            SET attributes = $1, updated_at = $2
            WHERE instrument_id = $3
            "#,
        )
        .bind(instrument.attributes.as_ref().map(|attr| &attr.0))
        .bind(&now)
        .bind(instrument_id)
        .execute(&self.pool)
        .await?;

        // 重新獲取
        self.get_by_id(instrument_id)
            .await?
            .ok_or_else(|| anyhow!("Instrument not found after update"))
    }

    /// 更新期貨特定屬性
    pub async fn update_future_attributes(
        &self,
        instrument_id: i32,
        attrs: FutureAttributes,
    ) -> Result<Instrument> {
        // 获取当前金融商品
        let mut instrument = self
            .get_by_id(instrument_id)
            .await?
            .ok_or_else(|| anyhow!("Instrument not found"))?;

        // 確保是期貨類型
        if instrument.instrument_type != "FUTURE" {
            return Err(anyhow!("Instrument is not a future"));
        }

        // 設置新的屬性
        instrument.set_future_attributes(attrs)?;

        // 更新屬性
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE instrument
            SET attributes = $1, updated_at = $2
            WHERE instrument_id = $3
            "#,
        )
        .bind(instrument.attributes.as_ref().map(|attr| &attr.0))
        .bind(&now)
        .bind(instrument_id)
        .execute(&self.pool)
        .await?;

        // 重新獲取
        self.get_by_id(instrument_id)
            .await?
            .ok_or_else(|| anyhow!("Instrument not found after update"))
    }

    /// 刪除金融商品
    pub async fn delete(&self, instrument_id: i32) -> Result<bool> {
        // 直接刪除金融商品記錄（所有屬性都存儲在同一個表中）
        let result = sqlx::query(
            r#"
            DELETE FROM instrument
            WHERE instrument_id = $1
            "#,
        )
        .bind(instrument_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 獲取金融商品和交易所資訊
    pub async fn get_instrument_with_exchange(
        &self,
        instrument_id: i32,
    ) -> Result<Option<InstrumentWithExchange>> {
        let record = sqlx::query_as::<_, InstrumentWithExchange>(
            r#"
            SELECT 
                i.instrument_id, i.symbol, i.exchange_id, i.instrument_type, i.name,
                i.description, i.currency, i.tick_size, i.lot_size, i.is_active,
                i.trading_start_date, i.trading_end_date, 
                i.attributes,
                i.created_at, i.updated_at,
                e.code as exchange_code, e.name as exchange_name, e.country as exchange_country
            FROM instrument i
            JOIN exchange e ON i.exchange_id = e.exchange_id
            WHERE i.instrument_id = $1
            "#,
        )
        .bind(instrument_id)
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

        let result = sqlx::query_as::<_, Instrument>(
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
                trading_start_date, trading_end_date, attributes,
                created_at, updated_at
            "#,
        )
        .bind(&instrument.symbol)
        .bind(&instrument.exchange_id)
        .bind(&instrument.instrument_type)
        .bind(&instrument.name)
        .bind(&instrument.description)
        .bind(&instrument.currency)
        .bind(&instrument.tick_size)
        .bind(&instrument.lot_size)
        .bind(&instrument.is_active)
        .bind(&instrument.trading_start_date)
        .bind(&instrument.trading_end_date)
        .bind(&serde_json::Value::Object(serde_json::Map::new()))
        .bind(&now)
        .bind(&now)
        .fetch_one(&mut **tx)
        .await?;

        Ok(result)
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
        // 使用域模型的時間轉換方法，避免直接處理DateTime
        // 但由於資料庫需要NaiveDate類型，還需要進一步轉換
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
            attributes: Some(Json(domain_instrument.attributes.clone())),
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
            _ => {
                return Err(anyhow!(
                    "Unknown instrument type: {}",
                    db_instrument.instrument_type
                ))
            }
        };

        // 從NaiveDate轉換為UTC DateTime
        // 這裡我們不能直接使用time_utils，因為我們需要從NaiveDate轉換，而不是timestamp
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
            tick_size: db_instrument
                .tick_size
                .map(|d| d.to_f64().unwrap_or(0.01))
                .unwrap_or(0.01),
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
    use crate::domain_types::instrument::{
        Instrument as DomainInstrument, StockAttributes as DomainStockAttributes,
    };
    use crate::domain_types::types::AssetType;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_domain_model_conversion() -> Result<()> {
        // 創建域模型實例
        let stock_attrs = DomainStockAttributes {
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
        let db_model = InstrumentRepository::domain_to_db_model(&domain_instrument, None);

        assert_eq!(db_model.symbol, "AAPL");
        assert_eq!(db_model.name, "Apple Inc.");
        assert_eq!(db_model.instrument_type, "STOCK");
        assert_eq!(db_model.exchange_id, None);

        // 創建模擬的數據庫實例以測試反向轉換
        let now = Utc::now();
        let db_instance = Instrument {
            instrument_id: 1,
            symbol: "AAPL".to_string(),
            exchange_id: None,
            instrument_type: "STOCK".to_string(),
            name: "Apple Inc.".to_string(),
            description: Some(
                "Apple Inc. designs, manufactures, and markets smartphones.".to_string(),
            ),
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
        let converted_domain =
            InstrumentRepository::db_to_domain_model(&db_instance, Some("NASDAQ".to_string()))?;

        assert_eq!(converted_domain.instrument_id, "1");
        assert_eq!(converted_domain.symbol, "AAPL");
        assert_eq!(converted_domain.exchange, "NASDAQ");
        assert_eq!(converted_domain.asset_type, AssetType::Stock);

        Ok(())
    }
}
