use super::{
    traits::DataValidator,
    error::{DataValidationError, ValidationResult},
    ohlcv_validator::OHLCVValidator,
    tick_validator::TickValidator,
};
use crate::domain_types::{TimeSeries, OHLCVPoint, TickPoint};
use crate::storage::repository::TimeRange;
use chrono::{DateTime, Utc, Duration};

/// 可獲取時間戳的數據點特徵
pub trait HasTimestamp {
    fn get_timestamp(&self) -> DateTime<Utc>;
    
    // 獲取數據點的時間範圍，默認實現為時間戳作為開始和結束
    fn get_time_range(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let timestamp = self.get_timestamp();
        (timestamp, timestamp)
    }
}

// 為 OHLCVPoint 實現 HasTimestamp 特徵
impl HasTimestamp for OHLCVPoint {
    fn get_timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    
    // 覆寫get_time_range方法，將OHLCV點視為一個時間範圍
    // 對於分鐘K線，每個點代表從timestamp到timestamp+1分鐘的數據
    fn get_time_range(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        // 從metadata中嘗試獲取頻率信息，若無則假設為1分鐘
        let end_time = if let Some(freq) = self.metadata.get("frequency") {
            // 使用更精確的字符串匹配，處理Debug格式的頻率字符串
            // 例如 "Minute(1)", "Hour(1)", "Day" 等格式
            if freq.starts_with("Minute(") {
                // 嘗試解析分鐘數，例如從 "Minute(5)" 提取 5
                if let Some(min_str) = freq.strip_prefix("Minute(").and_then(|s| s.strip_suffix(')')) {
                    if let Ok(mins) = min_str.parse::<i64>() {
                        self.timestamp + Duration::minutes(mins)
                    } else {
                        self.timestamp + Duration::minutes(1) // 默認1分鐘
                    }
                } else {
                    self.timestamp + Duration::minutes(1)
                }
            } else if freq.starts_with("Hour(") {
                // 嘗試解析小時數
                if let Some(hour_str) = freq.strip_prefix("Hour(").and_then(|s| s.strip_suffix(')')) {
                    if let Ok(hours) = hour_str.parse::<i64>() {
                        self.timestamp + Duration::hours(hours)
                    } else {
                        self.timestamp + Duration::hours(1) // 默認1小時
                    }
                } else {
                    self.timestamp + Duration::hours(1)
                }
            } else if freq == "Day" {
                self.timestamp + Duration::days(1)
            } else if freq == "Week" {
                self.timestamp + Duration::weeks(1)
            } else if freq == "Month" {
                self.timestamp + Duration::days(30) // 簡化處理，一個月約30天
            } else if freq == "Quarter" {
                self.timestamp + Duration::days(91) // 簡化處理，一個季度約91天
            } else if freq == "Year" {
                self.timestamp + Duration::days(365) // 簡化處理，一年約365天
            } else if freq == "Tick" {
                // Tick數據沒有時間範圍，使用極小的時間差
                self.timestamp + Duration::nanoseconds(1)
            } else {
                // 默認假設為1分鐘K線
                self.timestamp + Duration::minutes(1)
            }
        } else {
            // 默認假設為1分鐘K線
            println!("數據點 {} 無頻率信息，使用默認1分鐘", self.timestamp);
            self.timestamp + Duration::minutes(1)
        };
        
        let range = (self.timestamp, end_time);
        range
    }
}

// 為 TickPoint 實現 HasTimestamp 特徵
impl HasTimestamp for TickPoint {
    fn get_timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    
    // Tick 點通常是一個精確的時間點，不代表範圍
}

/// 時間重疊檢查結果
#[derive(Debug, Clone)]
pub struct TimeOverlapResult {
    pub has_overlap: bool,                // 是否有重疊
    pub overlap_start: Option<DateTime<Utc>>,  // 重疊開始時間
    pub overlap_end: Option<DateTime<Utc>>,    // 重疊結束時間
    pub overlap_points_count: usize,      // 重疊的數據點數量
    pub message: String,                  // 描述訊息
}

impl TimeOverlapResult {
    pub fn new_no_overlap() -> Self {
        Self {
            has_overlap: false,
            overlap_start: None,
            overlap_end: None,
            overlap_points_count: 0,
            message: "無時間重疊".to_string(),
        }
    }
    
    pub fn new_with_overlap(
        start: DateTime<Utc>, 
        end: DateTime<Utc>, 
        count: usize
    ) -> Self {
        Self {
            has_overlap: true,
            overlap_start: Some(start),
            overlap_end: Some(end),
            overlap_points_count: count,
            message: format!(
                "發現時間重疊：{} 至 {}，共 {} 個數據點", 
                start, end, count
            ),
        }
    }
}

pub struct TimeSeriesValidator<T, V: DataValidator<T>> {
    item_validator: V,
    validate_chronological_order: bool,
    min_data_points: Option<usize>,
    phantom: std::marker::PhantomData<T>,
    check_database_duplicates: bool,
}

impl<T, V: DataValidator<T>> TimeSeriesValidator<T, V> {
    pub fn new(validator: V) -> Self {
        Self {
            item_validator: validator,
            validate_chronological_order: true,
            min_data_points: None,
            phantom: std::marker::PhantomData,
            check_database_duplicates: false,
        }
    }
    
    pub fn with_min_data_points(mut self, min_points: usize) -> Self {
        self.min_data_points = Some(min_points);
        self
    }
    
    pub fn set_validate_chronological_order(mut self, validate: bool) -> Self {
        self.validate_chronological_order = validate;
        self
    }
    
    pub fn set_check_database_duplicates(mut self, check: bool) -> Self {
        self.check_database_duplicates = check;
        self
    }
    
    pub fn validate_chronological_order(&self) -> bool {
        self.validate_chronological_order
    }
    

    // 新增異步驗證方法，用於需要檢查資料庫重複的情況
    pub async fn validate_async(&self, time_series: &mut TimeSeries<T>) -> ValidationResult<()>
    where
        T: Clone + HasTimestamp
    {
        // 檢查最小數據點要求
        if let Some(min_points) = self.min_data_points {
            if time_series.len() < min_points {
                return Err(DataValidationError::MissingData {
                    field: format!("time_series.data (至少需要{}個數據點)", min_points),
                    context: None,
                });
            }
        }
        
        // 檢查數據庫中是否有重複數據
        if self.check_database_duplicates && !time_series.is_empty() {
            self.check_database_duplicates_async(time_series).await?;
        }
        
        // 驗證每個數據點並過濾無效點
        let mut validated_data = Vec::with_capacity(time_series.len());
        
        for item in time_series.data.iter() {
            if self.item_validator.validate_item(item).is_ok() {
                validated_data.push(item.clone());
            }
        }
        time_series.data = validated_data;
        
        // 檢查時間順序（如果啟用且數據點多於一個）
        if self.validate_chronological_order && time_series.len() > 1 {
            let mut last_timestamp = time_series.data[0].get_timestamp();
            
            for i in 1..time_series.len() {
                let current_timestamp = time_series.data[i].get_timestamp();
                
                if current_timestamp < last_timestamp {
                    return Err(DataValidationError::TimeSeriesError {
                        message: format!(
                            "時間序列順序錯誤：索引 {} 的時間 ({}) 早於前一時間 ({})",
                            i, current_timestamp, last_timestamp
                        ),
                        context: None,
                    });
                } else if current_timestamp == last_timestamp{
                    return Err(DataValidationError::TimeSeriesError {
                        message: format!(
                            "時間序列順序錯誤：索引 {} 的時間 ({}) 與前一時間相同，且不允許重複或未處理重複",
                            i, current_timestamp
                        ),
                        context: None,
                    });
                }
                
                last_timestamp = current_timestamp;
            }
        }
        
        // 重新計算時間範圍
        if !time_series.is_empty() {
            let first = time_series.data.first().unwrap();
            let last = time_series.data.last().unwrap();
            time_series.start_time = Some(first.get_timestamp());
            time_series.end_time = Some(last.get_timestamp());
        } else {
            time_series.start_time = None;
            time_series.end_time = None;
        }
        
        // 檢查時間重疊
        if time_series.len() > 1 {
            // 輸出時間序列頻率信息，幫助診斷
            if let Some(freq) = &time_series.frequency {
                println!("時間序列頻率: {:?}", freq);
            } else {
                println!("時間序列未設置頻率");
            }
            
            self.check_time_series_overlap(time_series)?;
        }
        
        Ok(())
    }
    
    // 將檢查資料庫中是否有重複數據的方法改為異步方法
    async fn check_database_duplicates_async(&self, time_series: &TimeSeries<T>) -> ValidationResult<()>
    where
        T: HasTimestamp
    {
        use crate::storage::database::get_db_pool;
        use sqlx::Row;
        
        // 獲取解析後的symbol並轉換為instrument_id
        let symbol = &time_series.symbol;
        
        // 獲取數據庫連接池
        let pool = match get_db_pool(false).await {
            Ok(pool) => pool,
            Err(e) => {
                return Err(DataValidationError::SystemError {
                    message: format!("無法獲取數據庫連接池: {}", e),
                    source: None,
                });
            }
        };
        
        // 如果symbol是數字，直接當作instrument_id使用，否則查詢數據庫
        let instrument_id = match symbol.parse::<i32>() {
            Ok(id) => id,
            Err(_) => {
                // 查詢instrument表獲取instrument_id
                match sqlx::query("SELECT instrument_id FROM instrument WHERE symbol = $1")
                    .bind(symbol)
                    .fetch_optional(pool)
                    .await
                {
                    Ok(Some(row)) => row.get::<i32, _>("instrument_id"),
                    Ok(None) => {
                        // 如果找不到對應的instrument_id，說明這是新數據，不存在重複
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(DataValidationError::SystemError {
                            message: format!("查詢instrument_id時發生錯誤: {}", e),
                            source: None,
                        });
                    }
                }
            }
        };
        
        println!("檢查商品ID {} 是否有重複數據", instrument_id);
        
        // 獲取時間範圍
        if time_series.is_empty() {
            return Ok(());
        }
        
        let start_time = time_series.start_time.unwrap_or_else(|| time_series.data[0].get_timestamp());
        let end_time = time_series.end_time.unwrap_or_else(|| time_series.data.last().unwrap().get_timestamp());
        
        let time_range = TimeRange::new(start_time, end_time);
        
        // 根據數據類型檢查數據庫中是否存在重複數據
        match time_series.data_type {
            crate::domain_types::DataType::OHLCV => {
                // 檢查分鐘K線表
                let result = sqlx::query(
                    "SELECT time FROM minute_bar 
                    WHERE instrument_id = $1 AND time >= $2 AND time <= $3 
                    ORDER BY time ASC"
                )
                .bind(instrument_id)
                .bind(time_range.start)
                .bind(time_range.end)
                .fetch_all(pool)
                .await;
                
                match result {
                    Ok(rows) => {
                        if !rows.is_empty() {
                            // 將已存在的時間戳轉換為HashSet，用於快速查找
                            use std::collections::HashSet;
                            let mut existing_times = HashSet::new();
                            
                            for row in rows {
                                let time: chrono::DateTime<chrono::Utc> = row.get("time");
                                existing_times.insert(time);
                            }
                            
                            // 檢查時間序列中是否有與數據庫重複的時間戳
                            for (i, point) in time_series.data.iter().enumerate() {
                                let ts = point.get_timestamp();
                                if existing_times.contains(&ts) {
                                    return Err(DataValidationError::duplicate_with_timestamp(
                                        format!("數據點 {} 在數據庫中已存在(時間戳: {})", i, ts),
                                        ts
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(DataValidationError::SystemError {
                            message: format!("查詢分鐘K線數據時發生錯誤: {}", e),
                            source: None,
                        });
                    }
                }
            }
            crate::domain_types::DataType::Tick => {
                // 檢查Tick表
                let result = sqlx::query(
                    "SELECT time FROM tick 
                    WHERE instrument_id = $1 AND time >= $2 AND time <= $3 
                    ORDER BY time ASC"
                )
                .bind(instrument_id)
                .bind(time_range.start)
                .bind(time_range.end)
                .fetch_all(pool)
                .await;
                
                match result {
                    Ok(rows) => {
                        if !rows.is_empty() {
                            // 將已存在的時間戳轉換為HashSet，用於快速查找
                            use std::collections::HashSet;
                            let mut existing_times = HashSet::new();
                            
                            for row in rows {
                                let time: chrono::DateTime<chrono::Utc> = row.get("time");
                                existing_times.insert(time);
                            }
                            
                            // 檢查時間序列中是否有與數據庫重複的時間戳
                            for (i, point) in time_series.data.iter().enumerate() {
                                let ts = point.get_timestamp();
                                if existing_times.contains(&ts) {
                                    return Err(DataValidationError::duplicate_with_timestamp(
                                        format!("數據點 {} 在數據庫中已存在(時間戳: {})", i, ts),
                                        ts
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(DataValidationError::SystemError {
                            message: format!("查詢Tick數據時發生錯誤: {}", e),
                            source: None,
                        });
                    }
                }
            }
            _ => {
                // 其他數據類型，目前不實現
                println!("不支持的數據類型: {:?}", time_series.data_type);
            }
        }
        
        Ok(())
    }
    
    // 檢查時間序列中是否存在時間重疊
    fn check_time_series_overlap(&self, time_series: &TimeSeries<T>) -> ValidationResult<()>
    where 
        T: HasTimestamp
    {
        // 實現檢查時間重疊的邏輯
        // 如果數據點已按時間順序排序，我們只需檢查相鄰點之間的重疊
        
        if time_series.len() <= 1 {
            return Ok(());
        }
        
        println!("開始檢查時間序列重疊，共有 {} 個數據點", time_series.len());
        
        for i in 0..time_series.len() - 1 {
            let current_range = time_series.data[i].get_time_range();
            let next_range = time_series.data[i + 1].get_time_range();
            
            if let Some(overlap) = self.check_time_range_overlap(current_range, next_range) {
                // 計算重疊的時間差（以秒為單位）
                let overlap_duration = (overlap.1 - overlap.0).num_seconds();
                
                println!("發現時間重疊！索引 {}-{} 重疊時間 {} 秒 ({} 至 {})",
                    i, i+1, overlap_duration, overlap.0, overlap.1);
                
                // 獲取當前和下一個數據點的時間戳，用於更清晰的錯誤報告
                let current_ts = time_series.data[i].get_timestamp();
                let next_ts = time_series.data[i + 1].get_timestamp();
                
                return Err(DataValidationError::TimeSeriesError {
                    message: format!(
                        "時間序列重疊錯誤：索引 {} (時間戳 {}) 和索引 {} (時間戳 {}) 之間存在 {} 秒的時間範圍重疊 ({} 至 {})",
                        i, current_ts, i+1, next_ts, overlap_duration, overlap.0, overlap.1
                    ),
                    context: None,
                });
            }
        }
        
        println!("時間序列檢查完成，未發現重疊");
        Ok(())
    }
    
    // 檢查兩個時間範圍是否重疊，如果重疊則返回重疊範圍
    fn check_time_range_overlap(
        &self,
        time_range_a: (DateTime<Utc>, DateTime<Utc>),
        time_range_b: (DateTime<Utc>, DateTime<Utc>),
    ) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        let (start_a, end_a) = time_range_a;
        let (start_b, end_b) = time_range_b;
        
        // 兩種檢測重疊的情況
        // 1. 如果 A 的結束時間大於 B 的開始時間，則有潛在重疊
        // 2. 如果 B 的結束時間大於 A 的開始時間，則有潛在重疊 (這是為了捕捉逆序的情況)
        
        // 首先檢查時間戳是否相同，這是最明顯的重疊情況
        if start_a == start_b {
            println!("開始時間相同，視為重疊");
            return Some((start_a, if end_a < end_b { end_a } else { end_b }));
        }
        
        // 檢查範圍重疊的常規情況
        // 1. A 的範圍覆蓋了 B 的開始時間
        if end_a > start_b && start_a < start_b {
            let overlap_start = start_b;
            let overlap_end = if end_b < end_a { end_b } else { end_a };
            println!("A覆蓋B的開始，重疊範圍: {} 至 {}", overlap_start, overlap_end);
            return Some((overlap_start, overlap_end));
        }
        
        // 2. B 的範圍覆蓋了 A 的開始時間
        if end_b > start_a && start_b < start_a {
            let overlap_start = start_a;
            let overlap_end = if end_a < end_b { end_a } else { end_b };
            println!("B覆蓋A的開始，重疊範圍: {} 至 {}", overlap_start, overlap_end);
            return Some((overlap_start, overlap_end));
        }
        
        // 3. A 完全包含 B
        if start_a <= start_b && end_a >= end_b {
            println!("A完全包含B，重疊範圍: {} 至 {}", start_b, end_b);
            return Some((start_b, end_b));
        }
        
        // 4. B 完全包含 A
        if start_b <= start_a && end_b >= end_a {
            println!("B完全包含A，重疊範圍: {} 至 {}", start_a, end_a);
            return Some((start_a, end_a));
        }
        
        None
    }
}

/// 創建用於OHLCV數據的時間序列驗證器
pub fn create_ohlcv_validator() -> TimeSeriesValidator<OHLCVPoint, OHLCVValidator> {
    let validator = OHLCVValidator::new();
    TimeSeriesValidator::new(validator)
        .set_check_database_duplicates(true)
}

/// 創建用於Tick數據的時間序列驗證器
pub fn create_tick_validator() -> TimeSeriesValidator<TickPoint, TickValidator> {
    let validator = TickValidator::new();
    TimeSeriesValidator::new(validator)
        .set_check_database_duplicates(true)
} 