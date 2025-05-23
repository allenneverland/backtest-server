use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::types::Decimal;
use std::str::FromStr;
use tracing::info;

use crate::storage::{
    models::market_data::{MinuteBarInsert, TickInsert},
    repository::market_data::MarketDataRepository,
};

/// 將 OHLCV 數據寫入資料庫
pub async fn write_ohlcv_to_db<R: MarketDataRepository>(
    repo: &R,
    data: Vec<(DateTime<Utc>, f64, f64, f64, f64, f64)>, // (timestamp, open, high, low, close, volume)
    instrument_id: i32,
    batch_size: usize,
) -> Result<usize> {
    let bars: Vec<MinuteBarInsert> = data
        .into_iter()
        .map(|(time, open, high, low, close, volume)| {
            Ok(MinuteBarInsert {
                time,
                instrument_id,
                open: Decimal::from_str(&open.to_string())?,
                high: Decimal::from_str(&high.to_string())?,
                low: Decimal::from_str(&low.to_string())?,
                close: Decimal::from_str(&close.to_string())?,
                volume: Decimal::from_str(&volume.to_string())?,
                amount: None,
                open_interest: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let count = bars.len();
    repo.batch_insert_minute_bars(&bars, batch_size).await?;
    info!("寫入 {} 條 OHLCV 記錄到資料庫", count);
    Ok(count)
}

/// 將 Tick 數據寫入資料庫
pub async fn write_ticks_to_db<R: MarketDataRepository>(
    repo: &R,
    data: Vec<(DateTime<Utc>, f64, f64)>, // (timestamp, price, volume)
    instrument_id: i32,
    batch_size: usize,
) -> Result<usize> {
    let ticks: Vec<TickInsert> = data
        .into_iter()
        .map(|(time, price, volume)| {
            Ok(TickInsert {
                time,
                instrument_id,
                price: Decimal::from_str(&price.to_string())?,
                volume: Decimal::from_str(&volume.to_string())?,
                trade_type: None,
                bid_price_1: None,
                bid_volume_1: None,
                ask_price_1: None,
                ask_volume_1: None,
                bid_prices: None,
                bid_volumes: None,
                ask_prices: None,
                ask_volumes: None,
                open_interest: None,
                spread: None,
                metadata: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let count = ticks.len();
    repo.batch_insert_ticks(&ticks, batch_size).await?;
    info!("寫入 {} 條 Tick 記錄到資料庫", count);
    Ok(count)
}