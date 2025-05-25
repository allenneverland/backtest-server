//! 資料攝取示範程式
//!
//! 展示如何使用資料攝取功能將數據寫入資料庫

use anyhow::Result;
use backtest_server::{
    config::{get_config, init_config},
    data_ingestion::{write_ohlcv_to_db, write_ticks_to_db},
    storage::{
        database::init_db_pool,
        models::instrument::InstrumentInsert,
        repository::{
            exchange::{ExchangeInsert, ExchangeRepository},
            instrument::InstrumentRepository,
            market_data::PgMarketDataRepository,
        },
    },
};
use chrono::Utc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    tracing_subscriber::fmt::init();

    // 初始化配置
    init_config()?;
    let config = get_config();

    // 連接資料庫
    info!("連接到資料庫...");
    let pool = init_db_pool(&config.database).await?;

    // 創建儲存庫
    let exchange_repo = ExchangeRepository::new(pool.clone());
    let instrument_repo = InstrumentRepository::new(pool.clone());
    let market_data_repo = PgMarketDataRepository::new(pool.clone());

    // 確保測試交易所存在
    info!("準備測試交易所...");
    let exchange_id = match exchange_repo.get_by_code("TEST").await? {
        Some(exchange) => {
            info!("交易所已存在，使用 ID: {}", exchange.exchange_id);
            exchange.exchange_id
        }
        None => {
            // 創建測試交易所
            let exchange = ExchangeInsert {
                code: "TEST".to_string(),
                name: "測試交易所".to_string(),
                country: "TW".to_string(),
                timezone: "Asia/Taipei".to_string(),
                operating_hours: None,
            };

            let created = exchange_repo.create(exchange).await?;
            info!("創建交易所成功，ID: {}", created.exchange_id);
            created.exchange_id
        }
    };

    // 準備測試金融商品
    info!("準備測試金融商品...");

    // 檢查是否已存在
    let existing = instrument_repo
        .get_by_symbol_and_exchange("TEST001", exchange_id)
        .await?;
    let instrument_id = if let Some(existing) = existing {
        info!("金融商品已存在，使用 ID: {}", existing.instrument_id);
        existing.instrument_id
    } else {
        // 創建新的金融商品
        let instrument = InstrumentInsert {
            symbol: "TEST001".to_string(),
            exchange_id: Some(exchange_id),
            instrument_type: "STOCK".to_string(),
            name: "測試股票001".to_string(),
            description: Some("資料攝取示範用測試股票".to_string()),
            currency: "TWD".to_string(),
            tick_size: Some(sqlx::types::Decimal::from_str_exact("0.01")?),
            lot_size: Some(1000),
            is_active: true,
            trading_start_date: None,
            trading_end_date: None,
            attributes: None,
        };

        let created = instrument_repo.create(instrument).await?;
        info!("創建金融商品成功，ID: {}", created.instrument_id);
        created.instrument_id
    };

    // 示範寫入 OHLCV 數據
    info!("\n寫入 OHLCV 數據示範");
    let now = Utc::now();
    let ohlcv_data = vec![
        (
            now - chrono::Duration::minutes(2),
            100.0,
            105.0,
            99.0,
            103.0,
            10000.0,
        ),
        (
            now - chrono::Duration::minutes(1),
            103.0,
            106.0,
            102.0,
            105.0,
            12000.0,
        ),
        (now, 105.0, 107.0, 104.0, 106.0, 15000.0),
    ];

    let count = write_ohlcv_to_db(&market_data_repo, ohlcv_data, instrument_id, 1000).await?;
    info!("成功寫入 {} 條 OHLCV 記錄", count);

    // 示範寫入 Tick 數據
    info!("\n寫入 Tick 數據示範");
    let tick_data = vec![
        (now - chrono::Duration::seconds(2), 100.0, 100.0),
        (now - chrono::Duration::seconds(1), 100.1, 200.0),
        (now, 99.9, 150.0),
    ];

    let count = write_ticks_to_db(&market_data_repo, tick_data, instrument_id, 1000).await?;
    info!("成功寫入 {} 條 Tick 記錄", count);

    info!("\n資料攝取示範完成");
    info!("提示：您可以使用以下 SQL 查詢來檢查寫入的數據：");
    info!(
        "  SELECT * FROM minute_bar WHERE instrument_id = {} ORDER BY time DESC LIMIT 5;",
        instrument_id
    );
    info!(
        "  SELECT * FROM tick WHERE instrument_id = {} ORDER BY time DESC LIMIT 5;",
        instrument_id
    );

    Ok(())
}
