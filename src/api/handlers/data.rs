use axum::{
    extract::{Path, Query, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use chrono::{Utc, NaiveDate};
use uuid::Uuid;
use rust_decimal::Decimal;
use std::fmt::Debug;
use crate::{
    domain_types::{
        asset_types::AssetType,
        TradeType,
        csv_format::CSVFormat,
    },
    data_ingestion::processor::{
        csv_io::{CSVImportOption},
        data_loader::DataLoader,
    },
    storage::{
        repository::{PageQuery, TimeRange, market_data::{MarketDataRepository, PgMarketDataRepository}, InstrumentRepository, instrument::{PgInstrumentRepository, ViewType}}, 
        database::get_db_pool,
        models::{
            market_data::{MinuteBarInsert, TickInsert}, 
            instrument::{Stock, Future, OptionContract, Forex, Crypto, FutureComplete, StockComplete, OptionComplete, ForexComplete, CryptoComplete, ViewToSymbolInfo}
        }
    }
};

#[derive(Deserialize)]
pub struct SymbolDataQuery {
    pub data_type: Option<String>,
    pub frequency: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct SymbolsQuery {
    pub instrument_type: Option<String>,
    pub view: Option<String>,
}

#[derive(Serialize)]
pub struct SymbolInfo {
    pub symbol: String,
    pub name: String,
    pub asset_type: String,
    pub exchange: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub listing_date: Option<String>,
    pub is_active: bool,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub file_id: String,
    pub records_processed: usize,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct SymbolDataResponse<T> {
    pub symbol: String,
    pub data_type: String,
    pub frequency: String,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub data: Vec<T>,
}

#[derive(Deserialize)]
pub struct CreateStrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub code: String,
}

#[derive(Serialize)]
pub struct CreateStrategyResponse {
    pub id: String,
    pub version: u32,
}

#[derive(Deserialize)]
pub struct UploadRequest {
    pub symbol: String,
    pub data_type: String,
    pub asset_type: String,
    pub file_content: String,
}

#[derive(Deserialize)]
pub struct FolderImportRequest {
    pub folder_name: String,
    pub data_type: String,
    pub asset_type: String,
    pub format_type: String,
}

#[derive(Serialize)]
pub struct FolderImportResponse {
    pub success: bool,
    pub folder_id: String,
    pub records_processed: usize,
    pub files_processed: usize,
    pub errors: Vec<String>,
}

pub async fn create(
    Json(_payload): Json<CreateStrategyRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // 實現策略創建邏輯
    let response = CreateStrategyResponse {
        id: uuid::Uuid::new_v4().to_string(),
        version: 1,
    };
    
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get(
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // 實現策略查詢邏輯
    Ok(Json(serde_json::json!({
        "id": id,
        "name": "Example Strategy",
        "status": "active",
        "version": 1,
    })))
}

// 泛型視圖查詢函數，減少重複代碼
async fn query_view<T>(repo: &PgInstrumentRepository, view_type: ViewType, page: PageQuery) -> Result<Vec<SymbolInfo>, StatusCode>
where
    T: ViewToSymbolInfo + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + Debug
{
    let items = repo.get_view_list::<T>(view_type, page)
        .await
        .map_err(|e| {
            tracing::error!("查詢 {:?} 視圖失敗: {}", view_type, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // 轉換為API響應格式
    let symbols = items.items
        .iter()
        .map(|item| item.to_symbol_info())
        .collect();
    
    Ok(symbols)
}

// 列出所有可用的金融商品，可按類型過濾
pub async fn list_symbols(
    Query(params): Query<SymbolsQuery>
) -> Result<impl IntoResponse, StatusCode> {
    // 從數據庫獲取所有金融商品信息
    let pool = get_db_pool(false).await
        .map_err(|e| {
            tracing::error!("獲取數據庫連接池失敗: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let repo = PgInstrumentRepository::new(pool.clone());
    
    let page = PageQuery {
        page: 1,
        page_size: 1000,  // 增加數量以獲取更多的金融商品
    };
    
    // 檢查是否需要使用視圖查詢
    if let Some(view) = &params.view {
        // 解析視圖類型
        if let Some(view_type) = ViewType::from_str(view) {
            tracing::info!("使用視圖查詢: {:?}", view_type);
            
            // 根據視圖類型查詢
            let instruments_vec = match view_type {
                ViewType::StockComplete => query_view::<StockComplete>(&repo, view_type, page).await?,
                ViewType::FutureComplete => query_view::<FutureComplete>(&repo, view_type, page).await?,
                ViewType::OptionComplete => query_view::<OptionComplete>(&repo, view_type, page).await?,
                ViewType::ForexComplete => query_view::<ForexComplete>(&repo, view_type, page).await?,
                ViewType::CryptoComplete => query_view::<CryptoComplete>(&repo, view_type, page).await?,
            };
            
            return Ok(Json(instruments_vec));
        } else {
            // 不支持的視圖類型，返回錯誤
            tracing::warn!("不支持的視圖類型: {}", view);
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    
    // 根據 instrument_type 參數查詢
    let instrument_type_ref = params.instrument_type.as_deref();
    let instruments_page = repo.get_instruments(instrument_type_ref, page)
        .await
        .map_err(|e| {
            tracing::error!("獲取金融商品列表失敗: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // 獲取所有交易所信息
    let exchanges = repo.get_exchanges()
        .await
        .map_err(|e| {
            tracing::error!("獲取交易所列表失敗: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // 創建交易所 ID 到名稱的映射
    let exchange_map: std::collections::HashMap<i32, String> = exchanges
        .into_iter()
        .map(|ex| (ex.exchange_id, ex.name))
        .collect();
    
    // 獲取所有金融商品信息
    let mut instruments_vec = Vec::new();
    for instrument in instruments_page.items {
        let exchange_name = match instrument.exchange_id {
            Some(exchange_id) => exchange_map
                .get(&exchange_id)
                .cloned()
                .unwrap_or_else(|| exchange_id.to_string()),
            None => "Unknown".to_string()
        };
        
        // 根據資產類型獲取詳細信息
        let (sector, industry, listing_date) = match instrument.instrument_type.as_str() {
            "STOCK" => {
                // 獲取 Stock 信息
                match repo.get_stock_by_instrument_id(instrument.instrument_id).await {
                    Ok(Some(stock_info)) => (stock_info.sector, stock_info.industry, stock_info.listing_date.map(|d| d.to_string())),
                    Err(e) => {
                        tracing::warn!("獲取股票信息失敗: {}", e);
                        (None, None, None)
                    },
                    _ => (None, None, None), // 如果無法找到 Stock 信息
                }
            },
            "FUTURE" => {
                // 獲取 Future 信息
                match repo.get_future_by_instrument_id(instrument.instrument_id).await {
                    Ok(Some(future_info)) => (
                        None, 
                        None, 
                        Some(future_info.delivery_date.to_string())
                    ),
                    Err(e) => {
                        tracing::warn!("獲取期貨信息失敗: {}", e);
                        (None, None, None)
                    },
                    _ => (None, None, None),
                }
            },
            "FOREX" => {
                // Forex 通常沒有行業和上市信息
                (Some("Currencies".to_string()), Some("Forex".to_string()), None)
            },
            "CRYPTO" => {
                // 加密貨幣信息
                (Some("Cryptocurrencies".to_string()), None, None)
            },
            _ => (None, None, None), // 其他資產類型
        };
        
        instruments_vec.push(SymbolInfo {
            symbol: instrument.symbol,
            name: instrument.name,
            asset_type: instrument.instrument_type,
            exchange: exchange_name,
            sector,
            industry,
            listing_date,
            is_active: instrument.is_active,
        });
    }
    
    Ok(Json(instruments_vec))
}

/// 獲取指定交易對/股票的歷史數據
pub async fn get_symbol_data(
    Path(symbol): Path<String>,
    Query(params): Query<SymbolDataQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    // 從數據庫獲取指定金融商品
    let pool = get_db_pool(false).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo = PgInstrumentRepository::new(pool.clone());
    
    // 獲取金融商品，不再限制類型為 STOCK
    let instrument = repo.get_instrument_by_symbol(&symbol, None, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // 不需要強制獲取股票特定屬性
    // 根據資產類型可以獲取不同的詳細信息
    match instrument.instrument_type.as_str() {
        "STOCK" => {
            // 可以獲取股票特定屬性，但不強制要求
            let _stock = repo.get_stock_by_instrument_id(instrument.instrument_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                // 不再使用 .ok_or(StatusCode::NOT_FOUND)? 因為可能不是股票
        },
        "FUTURE" => {
            // 可以獲取期貨特定屬性
            let _future = repo.get_future_by_instrument_id(instrument.instrument_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        },
        // 其他資產類型的特殊處理可以在這裡添加
        _ => {},
    }
    
    // 解析查詢參數
    let data_type = params.data_type.unwrap_or("OHLCV".to_string());
    let frequency = params.frequency.unwrap_or("1d".to_string());
    
    // 解析時間範圍
    let now = Utc::now();
    let to_date = match params.to_date {
        Some(date_str) => {
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map(|d| {
                    use chrono::offset::TimeZone;
                    Utc.from_utc_datetime(&d.and_hms_opt(23, 59, 59).unwrap())
                })
                .map_err(|_| StatusCode::BAD_REQUEST)?
        },
        None => now,
    };
    
    let from_date = match params.from_date {
        Some(date_str) => {
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map(|d| {
                    use chrono::offset::TimeZone;
                    Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
                })
                .map_err(|_| StatusCode::BAD_REQUEST)?
        },
        None => {
            // 默認獲取30天的數據
            to_date - chrono::Duration::days(30)
        },
    };
    
    let time_range = TimeRange {
        start: from_date,
        end: to_date,
    };
    
    // 根據頻率和數據類型獲取數據
    let market_data_repo = PgMarketDataRepository::new(pool.clone());
    
    if data_type == "OHLCV" {
        if frequency == "1m" {
            // 獲取分鐘級數據
            let minute_bars = market_data_repo.get_minute_bars(instrument.instrument_id, time_range, params.limit)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            // 轉換為API響應格式
            let data: Vec<serde_json::Value> = minute_bars.into_iter()
                .map(|bar| {
                    serde_json::json!({
                        "timestamp": bar.time.timestamp(),
                        "open": bar.open,
                        "high": bar.high,
                        "low": bar.low,
                        "close": bar.close,
                        "volume": bar.volume,
                        "amount": bar.amount
                    })
                })
                .collect();
            
            let response = SymbolDataResponse {
                symbol: symbol.clone(),
                data_type,
                frequency,
                from_date: Some(from_date.to_rfc3339()),
                to_date: Some(to_date.to_rfc3339()),
                data,
            };
            
            Ok(Json(response))
        } else if frequency == "1d" {
            // 獲取日級數據
            let daily_bars = market_data_repo.get_daily_bars(instrument.instrument_id, time_range)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            // 轉換為API響應格式
            let data: Vec<serde_json::Value> = daily_bars.into_iter()
                .map(|bar| {
                    use chrono::offset::TimeZone;
                    let timestamp = Utc.from_utc_datetime(&bar.date.and_hms_opt(0, 0, 0).unwrap()).timestamp();
                    
                    serde_json::json!({
                        "timestamp": timestamp,
                        "open": bar.open,
                        "high": bar.high,
                        "low": bar.low,
                        "close": bar.close,
                        "volume": bar.volume,
                        "amount": bar.amount
                    })
                })
                .collect();
            
            let response = SymbolDataResponse {
                symbol: symbol.clone(),
                data_type,
                frequency,
                from_date: Some(from_date.to_rfc3339()),
                to_date: Some(to_date.to_rfc3339()),
                data,
            };
            
            Ok(Json(response))
        } else {
            // 目前僅支持 1m 和 1d 頻率
            Err(StatusCode::BAD_REQUEST)
        }
    } else if data_type == "TICK" {
        // 獲取 Tick 數據
        let ticks = market_data_repo.get_ticks(instrument.instrument_id, time_range, params.limit)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        // 轉換為API響應格式
        let data: Vec<serde_json::Value> = ticks.into_iter()
            .map(|tick| {
                serde_json::json!({
                    "timestamp": tick.time.timestamp(),
                    "price": tick.price,
                    "volume": tick.volume,
                    "trade_type": tick.trade_type,
                    "bid_price": tick.bid_price_1,
                    "bid_volume": tick.bid_volume_1,
                    "ask_price": tick.ask_price_1,
                    "ask_volume": tick.ask_volume_1,
                })
            })
            .collect();
        
        let response = SymbolDataResponse {
            symbol: symbol.clone(),
            data_type,
            frequency: "tick".to_string(),
            from_date: Some(from_date.to_rfc3339()),
            to_date: Some(to_date.to_rfc3339()),
            data,
        };
        
        Ok(Json(response))
    } else {
        // 不支持的數據類型
        Err(StatusCode::BAD_REQUEST)
    }
}

/// 從原始資料夾匯入數據到數據庫
pub async fn import_folder(
    Json(payload): Json<FolderImportRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // 驗證資料類型
    let data_type = payload.data_type.to_uppercase();
    if !matches!(data_type.as_str(), "OHLCV" | "TICK") {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // 解析資產類型
    let asset_type = match payload.asset_type.to_uppercase().as_str() {
        "STOCK" => AssetType::Stock,
        "FUTURE" => AssetType::Future,
        "FOREX" => AssetType::Forex,
        "CRYPTO" => AssetType::Crypto,
        "OPTIONCONTRACT" => AssetType::OptionContract,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    // 構建原始資料夾的路徑
    let raw_folder_path = std::path::Path::new("raw").join(&payload.folder_name);
    if !raw_folder_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    // 根據format_type選擇CSV格式
    let csv_format = CSVFormat::from_str(&payload.format_type)
        .ok_or_else(|| {
            tracing::error!("不支援的CSV格式: {}", payload.format_type);
            StatusCode::BAD_REQUEST
        })?;
    
    let csv_option = CSVImportOption::PredefinedFormat(csv_format);
    
    // 獲取配置
    let config = csv_option.to_reader_config();
    
    // 創建資料加載器
    let data_loader = DataLoader::new();
    
    // 記錄配置資訊，幫助診斷
    tracing::info!(
        "準備導入資料 - 資料夾: {}, 資料類型: {}, 資產類型: {:?}, 格式類型: {}, 時間戳列名: {}", 
        payload.folder_name, 
        data_type, 
        asset_type, 
        csv_format,
        config.timestamp_column
    );
    
    // 處理結果統計
    let mut records_processed = 0;
    let mut files_processed = 0;
    let mut errors = Vec::new();
    
    // 獲取數據庫連接池
    let pool = get_db_pool(false).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let market_data_repo = PgMarketDataRepository::new(pool.clone());
    
    // 根據數據類型處理資料夾
    match data_type.as_str() {
        "OHLCV" => {
            // 從資料夾載入OHLCV數據
            tracing::info!("開始載入OHLCV數據，路徑: {:?}", raw_folder_path);
            
            let time_series_list = match data_loader.load_ohlcv_dir(&raw_folder_path, asset_type, &config).await {
                Ok(list) => {
                    tracing::info!("成功載入 {} 個OHLCV時間序列", list.len());
                    list
                },
                Err(e) => {
                    let error_msg = format!("載入OHLCV資料夾失敗: {}", e);
                    tracing::error!("{}", error_msg);
                    errors.push(error_msg);
                    return Ok(Json(FolderImportResponse {
                        success: false,
                        folder_id: Uuid::new_v4().to_string(),
                        records_processed: 0,
                        files_processed,
                        errors,
                    }));
                }
            };
            
            if time_series_list.is_empty() {
                let error_msg = "沒有找到有效的OHLCV時間序列數據".to_string();
                tracing::warn!("{}", error_msg);
                errors.push(error_msg);
                return Ok(Json(FolderImportResponse {
                    success: false,
                    folder_id: Uuid::new_v4().to_string(),
                    records_processed: 0,
                    files_processed,
                    errors,
                }));
            }
            
            files_processed = time_series_list.len();
            
            // 嚴格模式：如果有任何文件加載錯誤，則中止整個匯入流程
            if errors.len() > 0 {
                tracing::error!("存在數據錯誤，中止匯入流程，錯誤數量: {}", errors.len());
                return Ok(Json(FolderImportResponse {
                    success: false,
                    folder_id: Uuid::new_v4().to_string(),
                    records_processed: 0,
                    files_processed,
                    errors,
                }));
            }
            
            // 處理每個時間序列
            for time_series in time_series_list {
                tracing::info!("處理商品 {}, 有 {} 筆數據", time_series.symbol, time_series.data.len());
                
                let repo = PgInstrumentRepository::new(pool.clone());
                
                // 查找或創建金融商品
                let instrument_id = match repo.get_or_create_instrument(&time_series.symbol, asset_type).await {
                    Ok(id) => id,
                    Err(e) => {
                        let error_msg = format!("處理商品 {} 失敗: {}", time_series.symbol, e);
                        tracing::error!("{}", error_msg);
                        errors.push(error_msg);
                        continue;
                    }
                };
                
                // 根據資產類型創建相應的專門表記錄
                match asset_type {
                    AssetType::Stock => {
                        // 如果 stock 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_stock_by_instrument_id(instrument_id).await {
                            // 創建一個基本的股票記錄
                            let stock = Stock {
                                instrument_id,
                                sector: None,
                                industry: None,
                                market_cap: None,
                                shares_outstanding: None,
                                free_float: None,
                                listing_date: None,
                                delisting_date: None,
                                dividend_yield: None,
                                pe_ratio: None,
                            };
                            if let Err(e) = repo.insert_stock(&stock).await {
                                let error_msg = format!("創建股票記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理，因為金融商品已經創建
                            }
                        }
                    },
                    AssetType::Future => {
                        // 如果 future 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_future_by_instrument_id(instrument_id).await {
                            // 創建一個基本的期貨記錄 (注意：這裡需要填寫必要的字段)
                            let future = Future {
                                instrument_id,
                                underlying_asset: "Unknown".to_string(),  // 必須提供
                                contract_size: Decimal::new(1, 0),        // 必須提供
                                contract_unit: None,
                                delivery_date: Utc::now().date_naive(),   // 必須提供
                                first_notice_date: None,
                                last_trading_date: Utc::now().date_naive(), // 必須提供
                                settlement_type: "CASH".to_string(),       // 必須提供
                                initial_margin: None,
                                maintenance_margin: None,
                                price_quotation: None,
                            };
                            if let Err(e) = repo.insert_future(&future).await {
                                let error_msg = format!("創建期貨記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    },
                    AssetType::Forex => {
                        // 如果 forex 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_forex_by_instrument_id(instrument_id).await {
                            // 創建一個基本的外匯記錄
                            let forex = Forex {
                                instrument_id,
                                base_currency: "USD".to_string(),         // 必須提供
                                quote_currency: "EUR".to_string(),        // 必須提供
                                pip_value: Decimal::new(1, 0),           // 必須提供
                                typical_spread: None,
                                margin_requirement: None,
                                trading_hours: None,
                            };
                            if let Err(e) = repo.insert_forex(&forex).await {
                                let error_msg = format!("創建外匯記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    },
                    AssetType::Crypto => {
                        // 如果 crypto 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_crypto_by_instrument_id(instrument_id).await {
                            // 創建一個基本的加密貨幣記錄
                            let crypto = Crypto {
                                instrument_id,
                                blockchain_network: None,
                                total_supply: None,
                                circulating_supply: None,
                                max_supply: None,
                                mining_algorithm: None,
                                consensus_mechanism: None,
                                website_url: None,
                                whitepaper_url: None,
                                github_url: None,
                            };
                            if let Err(e) = repo.insert_crypto(&crypto).await {
                                let error_msg = format!("創建加密貨幣記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    },
                    AssetType::OptionContract => {
                        // 如果 option_contract 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_option_contract_by_instrument_id(instrument_id).await {
                            // 創建一個基本的期權記錄
                            let option_contract = OptionContract {
                                instrument_id,
                                underlying_instrument_id: None,
                                option_type: "CALL".to_string(),          // 必須提供
                                strike_price: Decimal::new(100, 0),       // 必須提供
                                expiration_date: Utc::now().date_naive(),  // 必須提供
                                exercise_style: "AMERICAN".to_string(),    // 必須提供
                                contract_size: 100,                       // 必須提供
                                implied_volatility: None,
                                delta: None,
                                gamma: None,
                                theta: None,
                                vega: None,
                                rho: None,
                            };
                            if let Err(e) = repo.insert_option_contract(&option_contract).await {
                                let error_msg = format!("創建期權記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    }
                }
                
                // 轉換數據為分鐘K線插入格式
                let mut minute_bars = Vec::new();
                for point in &time_series.data {
                    minute_bars.push(MinuteBarInsert {
                        time: point.timestamp,
                        instrument_id,
                        open: Decimal::try_from(point.open).unwrap_or_default(),
                        high: Decimal::try_from(point.high).unwrap_or_default(),
                        low: Decimal::try_from(point.low).unwrap_or_default(),
                        close: Decimal::try_from(point.close).unwrap_or_default(),
                        volume: Decimal::try_from(point.volume).unwrap_or_default(),
                        amount: None,
                        open_interest: None,
                    });
                }
                
                // 批量插入分鐘K線
                if let Err(e) = market_data_repo.batch_insert_minute_bars(&minute_bars, 1000).await {
                    let error_msg = format!("插入 {} 的分鐘K線數據失敗: {}", time_series.symbol, e);
                    tracing::error!("{}", error_msg);
                    errors.push(error_msg);
                    continue;
                } else {
                    tracing::info!("成功插入 {} 的 {} 筆分鐘K線數據", time_series.symbol, minute_bars.len());
                }
                
                records_processed += time_series.data.len();
            }
        },
        "TICK" => {
            // 從資料夾載入TICK數據
            tracing::info!("開始載入TICK數據，路徑: {:?}", raw_folder_path);
            
            let time_series_list = match data_loader.load_tick_dir(&raw_folder_path, asset_type, &config).await {
                Ok(list) => {
                    tracing::info!("成功載入 {} 個TICK時間序列", list.len());
                    list
                },
                Err(e) => {
                    let error_msg = format!("載入TICK資料夾失敗: {}", e);
                    tracing::error!("{}", error_msg);
                    errors.push(error_msg);
                    return Ok(Json(FolderImportResponse {
                        success: false,
                        folder_id: Uuid::new_v4().to_string(),
                        records_processed: 0,
                        files_processed,
                        errors,
                    }));
                }
            };
            
            if time_series_list.is_empty() {
                let error_msg = "沒有找到有效的TICK時間序列數據".to_string();
                tracing::warn!("{}", error_msg);
                errors.push(error_msg);
                return Ok(Json(FolderImportResponse {
                    success: false,
                    folder_id: Uuid::new_v4().to_string(),
                    records_processed: 0,
                    files_processed,
                    errors,
                }));
            }
            
            files_processed = time_series_list.len();
            
            // 嚴格模式：如果有任何文件加載錯誤，則中止整個匯入流程
            if errors.len() > 0 {
                tracing::error!("存在數據錯誤，中止匯入流程，錯誤數量: {}", errors.len());
                return Ok(Json(FolderImportResponse {
                    success: false,
                    folder_id: Uuid::new_v4().to_string(),
                    records_processed: 0,
                    files_processed,
                    errors,
                }));
            }
            
            // 處理每個時間序列
            for time_series in time_series_list {
                tracing::info!("處理商品 {}, 有 {} 筆TICK數據", time_series.symbol, time_series.data.len());
                
                let repo = PgInstrumentRepository::new(pool.clone());
                
                // 查找或創建金融商品
                let instrument_id = match repo.get_or_create_instrument(&time_series.symbol, asset_type).await {
                    Ok(id) => id,
                    Err(e) => {
                        let error_msg = format!("處理商品 {} 失敗: {}", time_series.symbol, e);
                        tracing::error!("{}", error_msg);
                        errors.push(error_msg);
                        continue;
                    }
                };
                
                // 根據資產類型創建相應的專門表記錄
                match asset_type {
                    AssetType::Stock => {
                        // 如果 stock 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_stock_by_instrument_id(instrument_id).await {
                            // 創建一個基本的股票記錄
                            let stock = Stock {
                                instrument_id,
                                sector: None,
                                industry: None,
                                market_cap: None,
                                shares_outstanding: None,
                                free_float: None,
                                listing_date: None,
                                delisting_date: None,
                                dividend_yield: None,
                                pe_ratio: None,
                            };
                            if let Err(e) = repo.insert_stock(&stock).await {
                                let error_msg = format!("創建股票記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理，因為金融商品已經創建
                            }
                        }
                    },
                    AssetType::Future => {
                        // 如果 future 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_future_by_instrument_id(instrument_id).await {
                            // 創建一個基本的期貨記錄
                            let future = Future {
                                instrument_id,
                                underlying_asset: "Unknown".to_string(),  // 必須提供
                                contract_size: Decimal::new(1, 0),        // 必須提供
                                contract_unit: None,
                                delivery_date: Utc::now().date_naive(),   // 必須提供
                                first_notice_date: None,
                                last_trading_date: Utc::now().date_naive(), // 必須提供
                                settlement_type: "CASH".to_string(),       // 必須提供
                                initial_margin: None,
                                maintenance_margin: None,
                                price_quotation: None,
                            };
                            if let Err(e) = repo.insert_future(&future).await {
                                let error_msg = format!("創建期貨記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    },
                    AssetType::Forex => {
                        // 如果 forex 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_forex_by_instrument_id(instrument_id).await {
                            // 創建一個基本的外匯記錄
                            let forex = Forex {
                                instrument_id,
                                base_currency: "USD".to_string(),         // 必須提供
                                quote_currency: "EUR".to_string(),        // 必須提供
                                pip_value: Decimal::new(1, 0),           // 必須提供
                                typical_spread: None,
                                margin_requirement: None,
                                trading_hours: None,
                            };
                            if let Err(e) = repo.insert_forex(&forex).await {
                                let error_msg = format!("創建外匯記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    },
                    AssetType::Crypto => {
                        // 如果 crypto 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_crypto_by_instrument_id(instrument_id).await {
                            // 創建一個基本的加密貨幣記錄
                            let crypto = Crypto {
                                instrument_id,
                                blockchain_network: None,
                                total_supply: None,
                                circulating_supply: None,
                                max_supply: None,
                                mining_algorithm: None,
                                consensus_mechanism: None,
                                website_url: None,
                                whitepaper_url: None,
                                github_url: None,
                            };
                            if let Err(e) = repo.insert_crypto(&crypto).await {
                                let error_msg = format!("創建加密貨幣記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    },
                    AssetType::OptionContract => {
                        // 如果 option_contract 表還沒有對應的記錄，創建一個
                        if let Ok(None) = repo.get_option_contract_by_instrument_id(instrument_id).await {
                            // 創建一個基本的期權記錄
                            let option_contract = OptionContract {
                                instrument_id,
                                underlying_instrument_id: None,
                                option_type: "CALL".to_string(),          // 必須提供
                                strike_price: Decimal::new(100, 0),       // 必須提供
                                expiration_date: Utc::now().date_naive(),  // 必須提供
                                exercise_style: "AMERICAN".to_string(),    // 必須提供
                                contract_size: 100,                       // 必須提供
                                implied_volatility: None,
                                delta: None,
                                gamma: None,
                                theta: None,
                                vega: None,
                                rho: None,
                            };
                            if let Err(e) = repo.insert_option_contract(&option_contract).await {
                                let error_msg = format!("創建期權記錄 {} 失敗: {}", time_series.symbol, e);
                                tracing::error!("{}", error_msg);
                                errors.push(error_msg);
                                // 但繼續處理
                            }
                        }
                    }
                }
                
                // 轉換數據為TICK插入格式
                let mut ticks = Vec::new();
                for point in &time_series.data {
                    let trade_type_value = match point.trade_type {
                        TradeType::Buy => Some(1i16),
                        TradeType::Sell => Some(2i16),
                        TradeType::Neutral => Some(0i16),
                        TradeType::Cross => Some(3i16),
                        TradeType::Unknown => None,
                    };
                    
                    let tick = TickInsert {
                        time: point.timestamp,
                        instrument_id,
                        price: Decimal::try_from(point.price).unwrap_or_default(),
                        volume: Decimal::try_from(point.volume).unwrap_or_default(),
                        trade_type: trade_type_value,
                        bid_price_1: if point.bid_price_1 > 0.0 { 
                            Some(Decimal::try_from(point.bid_price_1).unwrap_or_default()) 
                        } else { 
                            None 
                        },
                        bid_volume_1: if point.bid_volume_1 > 0.0 { 
                            Some(Decimal::try_from(point.bid_volume_1).unwrap_or_default()) 
                        } else { 
                            None 
                        },
                        ask_price_1: if point.ask_price_1 > 0.0 { 
                            Some(Decimal::try_from(point.ask_price_1).unwrap_or_default()) 
                        } else { 
                            None 
                        },
                        ask_volume_1: if point.ask_volume_1 > 0.0 { 
                            Some(Decimal::try_from(point.ask_volume_1).unwrap_or_default()) 
                        } else { 
                            None 
                        },
                        bid_prices: None,
                        bid_volumes: None,
                        ask_prices: None,
                        ask_volumes: None,
                        open_interest: None,
                        spread: None,
                        metadata: None,
                    };
                    ticks.push(tick);
                }
                
                // 批量插入TICK數據
                if let Err(e) = market_data_repo.batch_insert_ticks(&ticks, 1000).await {
                    let error_msg = format!("插入 {} 的TICK數據失敗: {}", time_series.symbol, e);
                    tracing::error!("{}", error_msg);
                    errors.push(error_msg);
                    continue;
                } else {
                    tracing::info!("成功插入 {} 的 {} 筆TICK數據", time_series.symbol, ticks.len());
                }
                
                records_processed += time_series.data.len();
            }
        },
        _ => return Err(StatusCode::BAD_REQUEST),
    }
    
    // 生成唯一標識符
    let folder_id = Uuid::new_v4().to_string();
    
    // 記錄匯入結果
    if errors.is_empty() {
        tracing::info!(
            "成功完成資料匯入 - 資料夾: {}, 處理記錄數: {}, 處理文件數: {}", 
            payload.folder_name, records_processed, files_processed
        );
    } else {
        tracing::warn!(
            "資料匯入部分成功 - 資料夾: {}, 處理記錄數: {}, 處理文件數: {}, 錯誤數: {}", 
            payload.folder_name, records_processed, files_processed, errors.len()
        );
    }
    
    // 返回導入結果
    let response = FolderImportResponse {
        success: errors.is_empty(),
        folder_id,
        records_processed,
        files_processed,
        errors,
    };
    
    Ok(Json(response))
}