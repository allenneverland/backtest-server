use anyhow::Result;
use backtest_server::{
    config::types::DatabaseConfig,
    domain_types::{
        instrument::{Instrument as DomainInstrument, StockAttributes},
        types::AssetType,
    },
    storage::{
        database,
        models::{
            instrument::{InstrumentInsert, StockAttributes as DbStockAttributes},
            Exchange, 
        },
        repository::{
            ExchangeRepository, InstrumentRepository, PageQuery,
            exchange::ExchangeInsert,
        },
    },
};
use rust_decimal_macros::dec;
use sqlx::PgPool;
use std::str::FromStr;
use chrono::NaiveDate;

// 測試配置
const TEST_DB_URL: &str = "postgres://postgres:postgres@timescaledb/postgres";

// 獲取測試用的數據庫連接池
async fn get_test_db_pool() -> PgPool {
    let db_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| TEST_DB_URL.to_string());
    
    let config = DatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "postgres".to_string(),
        password: "postgres".to_string(),
        database: db_url.split('/').last().unwrap_or("testdb").to_string(),
        connection_pool_size: 5,
        max_connections: 5,
        min_connections: 1,
        max_lifetime_secs: 1800,
        acquire_timeout_secs: 3,
        idle_timeout_secs: 30,
    };
    
    database::init_db_pool(&config).await.expect("Failed to connect to test database")
}

// 清理測試數據
async fn cleanup_test_data(pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM instrument").execute(pool).await?;
    sqlx::query!("DELETE FROM exchange").execute(pool).await?;
    Ok(())
}

// 創建測試交易所
async fn create_test_exchange(repo: &ExchangeRepository) -> Result<Exchange> {
    let exchange = ExchangeInsert {
        code: "NASDAQ".to_string(),
        name: "NASDAQ Stock Exchange".to_string(),
        country: "USA".to_string(),
        timezone: "America/New_York".to_string(),
        operating_hours: None,
    };
    
    repo.create(exchange.clone()).await
}

#[sqlx::test]
async fn test_exchange_repository_crud_operations() -> Result<()> {
    let pool = get_test_db_pool().await;
    let repo = ExchangeRepository::new(pool.clone());
    
    // 確保測試開始前數據庫是乾淨的
    cleanup_test_data(&pool).await?;
    
    // 創建交易所
    let exchange = create_test_exchange(&repo).await?;
    assert_eq!(exchange.code, "NASDAQ");
    assert_eq!(exchange.name, "NASDAQ Stock Exchange");
    
    // 獲取交易所
    let fetched = repo.get_by_id(exchange.exchange_id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.code, "NASDAQ");
    
    // 通過代碼獲取
    let by_code = repo.get_by_code("NASDAQ").await?;
    assert!(by_code.is_some());
    assert_eq!(by_code.unwrap().exchange_id, exchange.exchange_id);
    
    // 更新交易所
    let updated_exchange = ExchangeInsert {
        code: "NASDAQ".to_string(),
        name: "NASDAQ Global Select Market".to_string(),
        country: "USA".to_string(),
        timezone: "America/New_York".to_string(),
        operating_hours: None,
    };
    let updated = repo.update(
        exchange.exchange_id,
        updated_exchange.clone(),
    ).await?;
    assert_eq!(updated.name, "NASDAQ Global Select Market");
    
    // 獲取所有交易所
    let all = repo.get_all().await?;
    assert!(!all.is_empty());
    assert!(all.iter().any(|e| e.code == "NASDAQ"));
    
    // 刪除
    let deleted = repo.delete(exchange.exchange_id).await?;
    assert!(deleted);
    
    // 確認已刪除
    let fetched = repo.get_by_id(exchange.exchange_id).await?;
    assert!(fetched.is_none());
    
    Ok(())
}

#[sqlx::test]
async fn test_instrument_repository_operations() -> Result<()> {
    let pool = get_test_db_pool().await;
    let exchange_repo = ExchangeRepository::new(pool.clone());
    let instrument_repo = InstrumentRepository::new(pool.clone());
    
    // 確保測試開始前數據庫是乾淨的
    cleanup_test_data(&pool).await?;
    
    // 創建交易所
    let exchange = create_test_exchange(&exchange_repo).await?;
    
    // 創建金融商品
    let instrument = InstrumentInsert {
        symbol: "AAPL".to_string(),
        exchange_id: Some(exchange.exchange_id),
        instrument_type: "STOCK".to_string(),
        name: "Apple Inc.".to_string(),
        description: Some("Apple Inc. is an American multinational technology company.".to_string()),
        currency: "USD".to_string(),
        tick_size: Some(dec!(0.01)),
        lot_size: Some(100),
        is_active: true,
        trading_start_date: Some(NaiveDate::from_str("1980-12-12").unwrap()),
        trading_end_date: None,
        attributes: None,
    };
    
    // 測試創建
    let created = instrument_repo.create(instrument).await?;
    assert_eq!(created.symbol, "AAPL");
    assert_eq!(created.name, "Apple Inc.");
    
    // 測試獲取
    let fetched = instrument_repo.get_by_id(created.instrument_id).await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.symbol, "AAPL");
    
    // 測試通過代碼獲取
    let by_symbol = instrument_repo.get_by_symbol_and_exchange("AAPL", exchange.exchange_id).await?;
    assert!(by_symbol.is_some());
    assert_eq!(by_symbol.unwrap().instrument_id, created.instrument_id);
    
    // 測試創建股票屬性
    let stock_attrs = DbStockAttributes {
        sector: Some("Technology".to_string()),
        industry: Some("Consumer Electronics".to_string()),
        market_cap: Some(dec!(2000000000000)),
        shares_outstanding: Some(16_000_000_000),
        free_float: Some(15_900_000_000),
        listing_date: Some(NaiveDate::from_str("1980-12-12").unwrap()),
        delisting_date: None,
        dividend_yield: Some(dec!(0.6)),
        pe_ratio: Some(dec!(28.5)),
    };
    
    let msft_instrument = InstrumentInsert {
        symbol: "MSFT".to_string(),
        exchange_id: Some(exchange.exchange_id),
        instrument_type: "STOCK".to_string(),
        name: "Microsoft Corporation".to_string(),
        description: Some("Microsoft Corporation is an American multinational technology company.".to_string()),
        currency: "USD".to_string(),
        tick_size: Some(dec!(0.01)),
        lot_size: Some(100),
        is_active: true,
        trading_start_date: Some(NaiveDate::from_str("1986-03-13").unwrap()),
        trading_end_date: None,
        attributes: None,
    };
    
    let msft = instrument_repo.create_stock(msft_instrument, stock_attrs).await?;
    assert_eq!(msft.symbol, "MSFT");
    assert_eq!(msft.instrument_type, "STOCK");
    
    // 測試獲取股票屬性
    let msft_fetched = instrument_repo.get_by_id(msft.instrument_id).await?;
    assert!(msft_fetched.is_some());
    let msft_fetched = msft_fetched.unwrap();
    let attrs = msft_fetched.get_stock_attributes();
    assert!(attrs.is_some());
    let attrs = attrs.unwrap();
    assert_eq!(attrs.sector, Some("Technology".to_string()));
    assert_eq!(attrs.industry, Some("Consumer Electronics".to_string()));
    
    // 測試按類型獲取
    let stocks = instrument_repo.get_by_type("STOCK").await?;
    assert_eq!(stocks.len(), 2);
    
    // 測試按交易所獲取
    let exchange_instruments = instrument_repo.get_by_exchange(exchange.exchange_id).await?;
    assert_eq!(exchange_instruments.len(), 2);
    
    // 測試分頁
    let page_query = PageQuery {
        page: 1,
        page_size: 10,
    };
    
    let paged = instrument_repo.get_all_paged(page_query).await?;
    assert_eq!(paged.total, 2);
    assert_eq!(paged.items.len(), 2);
    
    // 測試搜索
    let search_results = instrument_repo.search("App").await?;
    assert!(search_results.iter().any(|i| i.symbol == "AAPL"));
    
    // 測試更新
    let update_instrument = InstrumentInsert {
        symbol: "AAPL".to_string(),
        exchange_id: Some(exchange.exchange_id),
        instrument_type: "STOCK".to_string(),
        name: "Apple Inc. Updated".to_string(),
        description: Some("Updated description".to_string()),
        currency: "USD".to_string(),
        tick_size: Some(dec!(0.01)),
        lot_size: Some(100),
        is_active: true,
        trading_start_date: Some(NaiveDate::from_str("1980-12-12").unwrap()),
        trading_end_date: None,
        attributes: None,
    };
    
    let updated = instrument_repo.update(created.instrument_id, update_instrument).await?;
    assert_eq!(updated.name, "Apple Inc. Updated");
    
    // 測試刪除
    let deleted = instrument_repo.delete(created.instrument_id).await?;
    assert!(deleted);
    
    let deleted = instrument_repo.delete(msft.instrument_id).await?;
    assert!(deleted);
    
    // 清理
    exchange_repo.delete(exchange.exchange_id).await?;
    
    Ok(())
}

#[sqlx::test]
async fn test_domain_model_integration() -> Result<()> {
    let pool = get_test_db_pool().await;
    let exchange_repo = ExchangeRepository::new(pool.clone());
    let instrument_repo = InstrumentRepository::new(pool.clone());
    
    // 確保測試開始前數據庫是乾淨的
    cleanup_test_data(&pool).await?;
    
    // 創建交易所
    let exchange = create_test_exchange(&exchange_repo).await?;
    
    // 創建域模型
    let stock_attrs = StockAttributes {
        sector: Some("Technology".to_string()),
        industry: Some("Consumer Electronics".to_string()),
        market_cap: Some(2000000000000.0),
        is_etf: false,
        dividend_yield: Some(0.6),
    };
    
    let domain_instrument = DomainInstrument::builder()
        .instrument_id("DOMAIN1")
        .symbol("GOOGL")
        .exchange("NASDAQ")
        .asset_type(AssetType::Stock)
        .name("Alphabet Inc.")
        .description("Alphabet Inc. is an American multinational technology conglomerate.")
        .currency("USD")
        .tick_size(0.01)
        .lot_size(100.0)
        .stock_attributes(stock_attrs)
        .build()
        .unwrap();
    
    // 將域模型轉換為數據庫模型
    let db_model = InstrumentRepository::domain_to_db_model(&domain_instrument, Some(exchange.exchange_id));
    
    // 創建金融商品
    let created = instrument_repo.create(db_model).await?;
    assert_eq!(created.symbol, "GOOGL");
    assert_eq!(created.name, "Alphabet Inc.");
    
    // 將數據庫模型轉換回域模型
    let converted_domain = InstrumentRepository::db_to_domain_model(&created, Some("NASDAQ".to_string()))?;
    
    // 驗證轉換是否正確
    assert_eq!(converted_domain.symbol, "GOOGL");
    assert_eq!(converted_domain.exchange, "NASDAQ");
    assert_eq!(converted_domain.asset_type, AssetType::Stock);
    
    // 獲取屬性
    if let Some(converted_attrs) = converted_domain.get_stock_attributes() {
        assert_eq!(converted_attrs.sector, Some("Technology".to_string()));
        assert_eq!(converted_attrs.industry, Some("Consumer Electronics".to_string()));
        assert!(!converted_attrs.is_etf);
    } else {
        panic!("Expected stock attributes to be present");
    }
    
    // 測試與交易所關聯的查詢
    let with_exchange = instrument_repo.get_instrument_with_exchange(created.instrument_id).await?;
    assert!(with_exchange.is_some());
    let with_exchange = with_exchange.unwrap();
    assert_eq!(with_exchange.exchange_code, "NASDAQ");
    
    // 清理
    instrument_repo.delete(created.instrument_id).await?;
    exchange_repo.delete(exchange.exchange_id).await?;
    
    Ok(())
}

#[sqlx::test]
async fn test_transaction_support() -> Result<()> {
    let pool = get_test_db_pool().await;
    let exchange_repo = ExchangeRepository::new(pool.clone());
    let instrument_repo = InstrumentRepository::new(pool.clone());
    
    // 確保測試開始前數據庫是乾淨的
    cleanup_test_data(&pool).await?;
    
    // 創建交易所
    let exchange = create_test_exchange(&exchange_repo).await?;
    
    // 使用事務創建多個金融商品
    let mut tx = pool.begin().await?;
    
    let instrument1 = InstrumentInsert {
        symbol: "TXN".to_string(),
        exchange_id: Some(exchange.exchange_id),
        instrument_type: "STOCK".to_string(),
        name: "Texas Instruments".to_string(),
        description: Some("Texas Instruments Inc. is an American technology company.".to_string()),
        currency: "USD".to_string(),
        tick_size: Some(dec!(0.01)),
        lot_size: Some(100),
        is_active: true,
        trading_start_date: None,
        trading_end_date: None,
        attributes: None,
    };
    
    let created1 = instrument_repo.create_in_tx(&mut tx, instrument1).await?;
    assert_eq!(created1.symbol, "TXN");
    
    let instrument2 = InstrumentInsert {
        symbol: "AMD".to_string(),
        exchange_id: Some(exchange.exchange_id),
        instrument_type: "STOCK".to_string(),
        name: "Advanced Micro Devices".to_string(),
        description: Some("Advanced Micro Devices, Inc. is an American multinational semiconductor company.".to_string()),
        currency: "USD".to_string(),
        tick_size: Some(dec!(0.01)),
        lot_size: Some(100),
        is_active: true,
        trading_start_date: None,
        trading_end_date: None,
        attributes: None,
    };
    
    let created2 = instrument_repo.create_in_tx(&mut tx, instrument2).await?;
    assert_eq!(created2.symbol, "AMD");
    
    // 提交事務
    tx.commit().await?;
    
    // 驗證事務中創建的對象
    let fetched1 = instrument_repo.get_by_id(created1.instrument_id).await?;
    assert!(fetched1.is_some());
    
    let fetched2 = instrument_repo.get_by_id(created2.instrument_id).await?;
    assert!(fetched2.is_some());
    
    // 測試事務回滾
    let mut tx = pool.begin().await?;
    
    let instrument3 = InstrumentInsert {
        symbol: "NVDA".to_string(),
        exchange_id: Some(exchange.exchange_id),
        instrument_type: "STOCK".to_string(),
        name: "NVIDIA Corporation".to_string(),
        description: Some("NVIDIA Corporation is an American technology company.".to_string()),
        currency: "USD".to_string(),
        tick_size: Some(dec!(0.01)),
        lot_size: Some(100),
        is_active: true,
        trading_start_date: None,
        trading_end_date: None,
        attributes: None,
    };
    
    let created3 = instrument_repo.create_in_tx(&mut tx, instrument3).await?;
    
    // 回滾事務
    tx.rollback().await?;
    
    // 驗證對象未被創建
    let fetched3 = instrument_repo.get_by_id(created3.instrument_id).await?;
    assert!(fetched3.is_none());
    
    // 清理
    instrument_repo.delete(created1.instrument_id).await?;
    instrument_repo.delete(created2.instrument_id).await?;
    exchange_repo.delete(exchange.exchange_id).await?;
    
    Ok(())
}