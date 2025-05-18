use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, FromRow};
use std::fmt::Debug;

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
        let stock = sqlx::query_as::<_, Stock>(
            "SELECT * FROM stock WHERE instrument_id = $1"
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(stock)
    }
    
    async fn get_future_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Future>> {
        let future = sqlx::query_as::<_, Future>(
            "SELECT * FROM future WHERE instrument_id = $1"
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(future)
    }
    
    async fn get_option_contract_by_instrument_id(&self, instrument_id: i32) -> Result<Option<OptionContract>> {
        let option_contract = sqlx::query_as::<_, OptionContract>(
            "SELECT * FROM option_contract WHERE instrument_id = $1"
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(option_contract)
    }
    
    async fn get_forex_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Forex>> {
        let forex = sqlx::query_as::<_, Forex>(
            "SELECT * FROM forex WHERE instrument_id = $1"
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(forex)
    }
    
    async fn get_crypto_by_instrument_id(&self, instrument_id: i32) -> Result<Option<Crypto>> {
        let crypto = sqlx::query_as::<_, Crypto>(
            "SELECT * FROM crypto WHERE instrument_id = $1"
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(crypto)
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
                trading_start_date, trading_end_date, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
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
                updated_at = $12
            WHERE instrument_id = $13
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
            Utc::now(),
            instrument.instrument_id
        )
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No instrument found with ID: {}", instrument.instrument_id));
        }

        Ok(())
    }
    
    async fn insert_stock(&self, stock: &Stock) -> Result<()> {
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
            stock.instrument_id,
            stock.sector,
            stock.industry,
            stock.market_cap,
            stock.shares_outstanding,
            stock.free_float,
            stock.listing_date,
            stock.delisting_date,
            stock.dividend_yield,
            stock.pe_ratio
        )
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn update_stock(&self, stock: &Stock) -> Result<()> {
        let rows_affected = sqlx::query!(
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
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No stock found with instrument ID: {}", stock.instrument_id));
        }

        Ok(())
    }
    
    async fn insert_future(&self, future: &Future) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO future (
                instrument_id, underlying_asset, contract_size, contract_unit,
                delivery_date, first_notice_date, last_trading_date, settlement_type,
                initial_margin, maintenance_margin, price_quotation
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            "#,
            future.instrument_id,
            future.underlying_asset,
            future.contract_size,
            future.contract_unit,
            future.delivery_date,
            future.first_notice_date,
            future.last_trading_date,
            future.settlement_type,
            future.initial_margin,
            future.maintenance_margin,
            future.price_quotation
        )
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn update_future(&self, future: &Future) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE future
            SET 
                underlying_asset = $1,
                contract_size = $2,
                contract_unit = $3,
                delivery_date = $4,
                first_notice_date = $5,
                last_trading_date = $6, 
                settlement_type = $7,
                initial_margin = $8,
                maintenance_margin = $9,
                price_quotation = $10
            WHERE instrument_id = $11
            "#,
            future.underlying_asset,
            future.contract_size,
            future.contract_unit,
            future.delivery_date,
            future.first_notice_date,
            future.last_trading_date,
            future.settlement_type,
            future.initial_margin,
            future.maintenance_margin,
            future.price_quotation,
            future.instrument_id
        )
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No future found with instrument ID: {}", future.instrument_id));
        }

        Ok(())
    }
    
    async fn insert_option_contract(&self, option_contract: &OptionContract) -> Result<()> {
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
            option_contract.instrument_id,
            option_contract.underlying_instrument_id,
            option_contract.option_type,
            option_contract.strike_price,
            option_contract.expiration_date,
            option_contract.exercise_style,
            option_contract.contract_size,
            option_contract.implied_volatility,
            option_contract.delta,
            option_contract.gamma,
            option_contract.theta,
            option_contract.vega,
            option_contract.rho
        )
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn update_option_contract(&self, option_contract: &OptionContract) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE option_contract
            SET 
                underlying_instrument_id = $1,
                option_type = $2,
                strike_price = $3,
                expiration_date = $4,
                exercise_style = $5,
                contract_size = $6,
                implied_volatility = $7,
                delta = $8,
                gamma = $9,
                theta = $10,
                vega = $11,
                rho = $12
            WHERE instrument_id = $13
            "#,
            option_contract.underlying_instrument_id,
            option_contract.option_type,
            option_contract.strike_price,
            option_contract.expiration_date,
            option_contract.exercise_style,
            option_contract.contract_size,
            option_contract.implied_volatility,
            option_contract.delta,
            option_contract.gamma,
            option_contract.theta,
            option_contract.vega,
            option_contract.rho,
            option_contract.instrument_id
        )
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No option contract found with instrument ID: {}", option_contract.instrument_id));
        }

        Ok(())
    }
    
    async fn insert_forex(&self, forex: &Forex) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO forex (
                instrument_id, base_currency, quote_currency, pip_value,
                typical_spread, margin_requirement, trading_hours
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7
            )
            "#,
            forex.instrument_id,
            forex.base_currency,
            forex.quote_currency,
            forex.pip_value,
            forex.typical_spread,
            forex.margin_requirement,
            forex.trading_hours.as_ref().map(|j| j.0.clone())
        )
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn update_forex(&self, forex: &Forex) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE forex
            SET 
                base_currency = $1,
                quote_currency = $2,
                pip_value = $3,
                typical_spread = $4,
                margin_requirement = $5,
                trading_hours = $6
            WHERE instrument_id = $7
            "#,
            forex.base_currency,
            forex.quote_currency,
            forex.pip_value,
            forex.typical_spread,
            forex.margin_requirement,
            forex.trading_hours.as_ref().map(|j| j.0.clone()),
            forex.instrument_id
        )
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No forex found with instrument ID: {}", forex.instrument_id));
        }

        Ok(())
    }
    
    async fn insert_crypto(&self, crypto: &Crypto) -> Result<()> {
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
            crypto.instrument_id,
            crypto.blockchain_network,
            crypto.total_supply,
            crypto.circulating_supply,
            crypto.max_supply,
            crypto.mining_algorithm,
            crypto.consensus_mechanism,
            crypto.website_url,
            crypto.whitepaper_url,
            crypto.github_url
        )
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn update_crypto(&self, crypto: &Crypto) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE crypto
            SET 
                blockchain_network = $1,
                total_supply = $2,
                circulating_supply = $3,
                max_supply = $4,
                mining_algorithm = $5,
                consensus_mechanism = $6,
                website_url = $7,
                whitepaper_url = $8,
                github_url = $9
            WHERE instrument_id = $10
            "#,
            crypto.blockchain_network,
            crypto.total_supply,
            crypto.circulating_supply,
            crypto.max_supply,
            crypto.mining_algorithm,
            crypto.consensus_mechanism,
            crypto.website_url,
            crypto.whitepaper_url,
            crypto.github_url,
            crypto.instrument_id
        )
        .execute(self.get_pool())
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("No crypto found with instrument ID: {}", crypto.instrument_id));
        }

        Ok(())
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