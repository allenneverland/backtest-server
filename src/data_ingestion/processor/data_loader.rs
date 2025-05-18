use anyhow::{Result, Context};
use super::csv_io::{CSVImporter, CSVReaderConfig};
use crate::domain_types::{TimeSeries, OHLCVPoint, TickPoint, AssetType, DataType, TradeType, Frequency};
use crate::data_ingestion::validator::{
    time_series_validator::{create_ohlcv_validator, create_tick_validator},
    ohlcv_validator::OHLCVValidator,
    tick_validator::TickValidator,
    traits::DataValidator,
};
use crate::storage::{
    repository::{market_data::MarketDataRepository, TimeRange},
    database::get_db_pool,
};
use chrono::{DateTime, Utc};
use std::path::Path;
use std::collections::HashMap;
use rust_decimal::prelude::ToPrimitive;
use serde_json;

#[derive(Debug)]
pub struct DataLoader {}

impl DataLoader {
    pub fn new() -> Self {
        Self {}
    }
    
    /// 載入OHLCV數據並進行驗證
    pub async fn load_ohlcv_from_csv(
        &self,
        file_path: impl AsRef<Path>,
        symbol: &str,
        asset_type: AssetType,
        config: &CSVReaderConfig,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        // 先檢查是否要對時間順序進行排序
        let validate_chronological_order = true;
        
        // 載入原始數據
        let mut time_series = CSVImporter::import_ohlcv(&file_path, symbol, asset_type, config)
            .context("從CSV導入OHLCV數據失敗")?;
        
        // 為每個數據點的metadata增加頻率信息
        if let Some(freq) = &time_series.frequency {
            let frequency_str = format!("{:?}", freq);
            tracing::info!("設置數據頻率: {} ({})", frequency_str, symbol);
            
            // 輸出前幾個數據點的時間戳，用於調試
            if !time_series.data.is_empty() {
                let sample_size = std::cmp::min(5, time_series.data.len());
                tracing::info!("數據樣本（前{}筆）時間戳:", sample_size);
                for i in 0..sample_size {
                    tracing::info!("  [{}] {}", i, time_series.data[i].timestamp);
                }
            }
            
            for point in &mut time_series.data {
                point.metadata.insert("frequency".to_string(), frequency_str.clone());
            }
        }
        
        let validator = OHLCVValidator::new()
            .with_price_range(0.0, f64::MAX) 
            .with_volume_range(0.0, f64::MAX);
        
        // 根據配置調整時間序列驗證器
        let ts_validator = create_ohlcv_validator()
            .with_min_data_points(1)
            .set_validate_chronological_order(validate_chronological_order);
        
        // 使用 validator 進行數據驗證
        validator.validate_batch(&time_series.data)
            .map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;
        
        // 排序數據確保時間順序正確
        if !validate_chronological_order {
            tracing::info!("按時間順序排序數據...");
            time_series.data.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }
        
        // 驗證時間序列，使用異步方法檢查時間重疊和數據庫重複
        ts_validator.validate_async(&mut time_series)
            .await
            .map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;
        
        Ok(time_series)
    }
    
    /// 載入Tick數據並進行驗證
    pub async fn load_tick_from_csv(
        &self,
        file_path: impl AsRef<Path>,
        symbol: &str,
        asset_type: AssetType,
        config: &CSVReaderConfig,
    ) -> Result<TimeSeries<TickPoint>> {
        // 先檢查是否要對時間順序進行排序
        let validate_chronological_order = true;
        
        // 載入原始數據
        let mut time_series = CSVImporter::import_tick(&file_path, symbol, asset_type, config)
            .context("從CSV導入Tick數據失敗")?;
        
        let validator = TickValidator::new();
        
        // 根據配置調整時間序列驗證器
        let ts_validator = create_tick_validator()
            .with_min_data_points(1)
            .set_validate_chronological_order(validate_chronological_order);
        
        // 使用 validator 進行數據驗證
        validator.validate_batch(&time_series.data)
            .map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;
        
        // 排序數據確保時間順序正確
        if !validate_chronological_order {
            tracing::info!("按時間順序排序數據...");
            time_series.data.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }
        
        // 驗證時間序列，使用異步方法檢查時間重疊和數據庫重複
        ts_validator.validate_async(&mut time_series)
            .await
            .map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;
        
        Ok(time_series)
    }
    
    /// 批量載入目錄中的OHLCV數據
    pub async fn load_ohlcv_dir(
        &self,
        dir_path: impl AsRef<Path>,
        asset_type: AssetType,
        config: &CSVReaderConfig,
    ) -> Result<Vec<TimeSeries<OHLCVPoint>>> {
        let mut result: Vec<TimeSeries<OHLCVPoint>> = Vec::new();
        
        // 遍歷目錄中的所有文件
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("csv") {
                // 從文件名獲取交易代碼
                let symbol = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // 導入數據（異步方法）
                match self.load_ohlcv_from_csv(&path, &symbol, asset_type, config).await {
                    Ok(ts) => result.push(ts),
                    Err(e) => {
                        let error_msg = format!("導入文件 {:?} 失敗: {}", path, e);
                        tracing::warn!("{}", error_msg);
                        // 只返回第一個錯誤，而不是收集所有錯誤
                        return Err(anyhow::anyhow!("{}", error_msg));
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// 批量載入目錄中的Tick數據
    pub async fn load_tick_dir(
        &self,
        dir_path: impl AsRef<Path>,
        asset_type: AssetType,
        config: &CSVReaderConfig,
    ) -> Result<Vec<TimeSeries<TickPoint>>> {
        let mut result: Vec<TimeSeries<TickPoint>> = Vec::new();
        
        // 遍歷目錄中的所有文件
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("csv") {
                // 從文件名獲取交易代碼
                let symbol = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // 導入數據（異步方法）
                match self.load_tick_from_csv(&path, &symbol, asset_type, config).await {
                    Ok(ts) => result.push(ts),
                    Err(e) => {
                        let error_msg = format!("導入文件 {:?} 失敗: {}", path, e);
                        tracing::warn!("{}", error_msg);
                        // 只返回第一個錯誤，而不是收集所有錯誤
                        return Err(anyhow::anyhow!("{}", error_msg));
                    }
                }
            }
        }
        
        Ok(result)
    }

    /// 從數據庫加載OHLCV數據
    pub async fn load_ohlcv_from_db(
        &self,
        stock_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<OHLCVPoint>> {
        let pool = get_db_pool(false).await?;
        let repo = crate::storage::repository::market_data::PgMarketDataRepository::new(pool.clone());
        
        let time_range = TimeRange::new(start_time, end_time);
        let bars = repo.get_minute_bars(stock_id, time_range, limit).await?;
        
        // 轉換為OHLCV時間序列
        let mut time_series = TimeSeries::new(
            stock_id.to_string(),
            AssetType::Stock,
            DataType::OHLCV,
            Some(Frequency::Minute(1)), // 明確設置頻率為1分鐘級
            "UTC".to_string(),
        );
        
        // 用於設置OHLCV點的頻率信息
        let frequency_str = format!("{:?}", Frequency::Minute(1));
        tracing::info!("從數據庫加載數據，設置頻率: {} ({})", frequency_str, stock_id);
        
        for bar in bars {
            let mut metadata = HashMap::new();
            metadata.insert("frequency".to_string(), frequency_str.clone());
            
            time_series.add_point(OHLCVPoint {
                timestamp: bar.time,
                open: bar.open.to_f64().unwrap_or_default(),
                high: bar.high.to_f64().unwrap_or_default(),
                low: bar.low.to_f64().unwrap_or_default(),
                close: bar.close.to_f64().unwrap_or_default(),
                volume: bar.volume.to_f64().unwrap_or_default(),
                metadata,
            });
        }
        
        // 輸出前幾個數據點的時間戳，用於調試
        if !time_series.data.is_empty() {
            let sample_size = std::cmp::min(5, time_series.data.len());
            tracing::info!("數據樣本（前{}筆）時間戳:", sample_size);
            for i in 0..sample_size {
                tracing::info!("  [{}] {}", i, time_series.data[i].timestamp);
            }
        }
        
        let validator = OHLCVValidator::new()
            .with_price_range(0.0, f64::MAX) 
            .with_volume_range(0.0, f64::MAX);
        
        // 使用帶有數據庫重複檢查功能的驗證器
        let ts_validator = create_ohlcv_validator()
            .with_min_data_points(1);
        
        // 使用 validator 進行數據驗證
        validator.validate_batch(&time_series.data)
            .map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;
        
        // 驗證時間序列，使用異步方法檢查時間重疊和數據庫重複
        ts_validator.validate_async(&mut time_series)
            .await
            .map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;
        
        Ok(time_series)
    }
    
    /// 從數據庫加載Tick數據
    pub async fn load_tick_from_db(
        &self,
        stock_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<TimeSeries<TickPoint>> {
        let pool = get_db_pool(false).await?;
        let repo = crate::storage::repository::market_data::PgMarketDataRepository::new(pool.clone());

        let time_range = TimeRange::new(start_time, end_time);
        let ticks = repo.get_ticks(stock_id, time_range, limit).await?;

        let mut time_series = TimeSeries::new(
            stock_id.to_string(),
            AssetType::Stock, // Assuming Stock, might need to be dynamic
            DataType::Tick,
            None,
            "UTC".to_string(),
        );

        for tick in ticks {
            // 從 JSON 中提取更多的深度行情數據（若有）
            let mut tick_point = TickPoint {
                timestamp: tick.time,
                price: tick.price.to_f64().unwrap_or_default(),
                volume: tick.volume.to_f64().unwrap_or_default(),
                trade_type: match tick.trade_type {
                    Some(1) => TradeType::Buy,
                    Some(2) => TradeType::Sell,
                    Some(3) => TradeType::Cross,
                    Some(0) => TradeType::Neutral,
                    _ => TradeType::Unknown,
                },
                bid_price_1: tick.bid_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_price_2: tick.bid_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_price_3: tick.bid_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_price_4: tick.bid_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_price_5: tick.bid_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_volume_1: tick.bid_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_volume_2: tick.bid_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_volume_3: tick.bid_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_volume_4: tick.bid_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                bid_volume_5: tick.bid_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_price_1: tick.ask_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_price_2: tick.ask_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_price_3: tick.ask_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_price_4: tick.ask_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_price_5: tick.ask_price_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_volume_1: tick.ask_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_volume_2: tick.ask_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_volume_3: tick.ask_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_volume_4: tick.ask_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                ask_volume_5: tick.ask_volume_1.unwrap_or_default().to_f64().unwrap_or_default(),
                metadata: HashMap::new(),
            };
            
            // 從 JSON 中提取深度行情數據
            if let Some(bid_prices) = &tick.bid_prices {
                if let Ok(prices) = serde_json::from_value::<Vec<f64>>(bid_prices.0.clone()) {
                    if prices.len() > 1 && prices.len() >= 2 { tick_point.bid_price_2 = prices[1]; }
                    if prices.len() >= 3 { tick_point.bid_price_3 = prices[2]; }
                    if prices.len() >= 4 { tick_point.bid_price_4 = prices[3]; }
                    if prices.len() >= 5 { tick_point.bid_price_5 = prices[4]; }
                }
            }
            
            if let Some(ask_prices) = &tick.ask_prices {
                if let Ok(prices) = serde_json::from_value::<Vec<f64>>(ask_prices.0.clone()) {
                    if prices.len() > 1 && prices.len() >= 2 { tick_point.ask_price_2 = prices[1]; }
                    if prices.len() >= 3 { tick_point.ask_price_3 = prices[2]; }
                    if prices.len() >= 4 { tick_point.ask_price_4 = prices[3]; }
                    if prices.len() >= 5 { tick_point.ask_price_5 = prices[4]; }
                }
            }
            
            if let Some(bid_volumes) = &tick.bid_volumes {
                if let Ok(volumes) = serde_json::from_value::<Vec<f64>>(bid_volumes.0.clone()) {
                    if volumes.len() > 1 && volumes.len() >= 2 { tick_point.bid_volume_2 = volumes[1]; }
                    if volumes.len() >= 3 { tick_point.bid_volume_3 = volumes[2]; }
                    if volumes.len() >= 4 { tick_point.bid_volume_4 = volumes[3]; }
                    if volumes.len() >= 5 { tick_point.bid_volume_5 = volumes[4]; }
                }
            }
            
            if let Some(ask_volumes) = &tick.ask_volumes {
                if let Ok(volumes) = serde_json::from_value::<Vec<f64>>(ask_volumes.0.clone()) {
                    if volumes.len() > 1 && volumes.len() >= 2 { tick_point.ask_volume_2 = volumes[1]; }
                    if volumes.len() >= 3 { tick_point.ask_volume_3 = volumes[2]; }
                    if volumes.len() >= 4 { tick_point.ask_volume_4 = volumes[3]; }
                    if volumes.len() >= 5 { tick_point.ask_volume_5 = volumes[4]; }
                }
            }
            
            time_series.add_point(tick_point);
        }

        let validator = TickValidator::new();
        // 使用帶有數據庫重複檢查功能的驗證器
        let ts_validator = create_tick_validator().with_min_data_points(1);

        validator.validate_batch(&time_series.data).map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;
        // 驗證時間序列，使用異步方法檢查時間重疊和數據庫重複
        ts_validator.validate_async(&mut time_series)
            .await
            .map_err(|e| anyhow::anyhow!("數據驗證錯誤: {}", e))?;

        Ok(time_series)
    }
} 