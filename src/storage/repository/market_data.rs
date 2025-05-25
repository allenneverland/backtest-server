use anyhow::Result;
use sqlx::PgPool;

use crate::storage::{
    models::market_data::{
        DailyBar, FundamentalIndicator, InstrumentDailyIndicator, MinuteBar, MinuteBarInsert,
        TechnicalIndicator, Tick, TickInsert,
    },
    repository::{DbExecutor, TimeRange},
};

/// 市場數據存取接口
#[async_trait::async_trait]
pub trait MarketDataRepository: Send + Sync + DbExecutor {
    // 分鐘K線數據
    async fn get_minute_bars(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>>;
    async fn get_latest_minute_bar(&self, instrument_id: i32) -> Result<Option<MinuteBar>>;
    async fn insert_minute_bars(&self, bars: &[MinuteBarInsert]) -> Result<()>;
    async fn batch_insert_minute_bars(
        &self,
        bars: &[MinuteBarInsert],
        batch_size: usize,
    ) -> Result<()>;

    // Tick數據
    async fn get_ticks(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<Tick>>;
    async fn get_latest_tick(&self, instrument_id: i32) -> Result<Option<Tick>>;
    async fn insert_ticks(&self, ticks: &[TickInsert]) -> Result<()>;
    async fn batch_insert_ticks(&self, ticks: &[TickInsert], batch_size: usize) -> Result<()>;

    // 日K線數據（連續聚合）
    async fn get_daily_bars(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<DailyBar>>;

    // 技術指標
    async fn get_technical_indicators(&self) -> Result<Vec<TechnicalIndicator>>;
    async fn get_technical_indicator_by_id(
        &self,
        indicator_id: i32,
    ) -> Result<Option<TechnicalIndicator>>;
    async fn get_instrument_daily_indicators(
        &self,
        instrument_id: i32,
        indicator_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<InstrumentDailyIndicator>>;
    async fn insert_instrument_daily_indicator(
        &self,
        indicator: &InstrumentDailyIndicator,
    ) -> Result<()>;

    // 基本面指標
    async fn get_fundamental_indicators(
        &self,
        instrument_id: i32,
        indicator_type: &str,
        time_range: TimeRange,
    ) -> Result<Vec<FundamentalIndicator>>;
    async fn insert_fundamental_indicator(&self, indicator: &FundamentalIndicator) -> Result<()>;

    /// 獲取分鐘K線數據 (從頭部開始取)
    async fn get_minute_bars_head(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>>;

    /// 獲取分鐘K線數據 (從尾部開始取)
    async fn get_minute_bars_tail(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>>;
}

/// PostgreSQL市場數據存取實現
pub struct PgMarketDataRepository {
    pool: PgPool,
}

impl PgMarketDataRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 獲取分鐘K線數據，從頭部開始取
    pub async fn get_minute_bars_head(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>> {
        let mut query = sqlx::query_as::<_, MinuteBar>(
            r#"
            SELECT instrument_id, time, open, high, low, close, volume, amount, open_interest
            FROM minute_bar
            WHERE instrument_id = $1 AND time >= $2 AND time <= $3
            ORDER BY time ASC
            "#,
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end);

        if let Some(limit_val) = limit {
            query = sqlx::query_as::<_, MinuteBar>(
                r#"
                SELECT instrument_id, time, open, high, low, close, volume, amount, open_interest
                FROM minute_bar
                WHERE instrument_id = $1 AND time >= $2 AND time <= $3
                ORDER BY time ASC
                LIMIT $4
                "#,
            )
            .bind(instrument_id)
            .bind(time_range.start)
            .bind(time_range.end)
            .bind(limit_val);
        }

        let result = query.fetch_all(&self.pool).await?;
        Ok(result)
    }

    /// 獲取分鐘K線數據，從尾部開始取
    pub async fn get_minute_bars_tail(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>> {
        let mut query = sqlx::query_as::<_, MinuteBar>(
            r#"
            SELECT instrument_id, time, open, high, low, close, volume, amount, open_interest
            FROM minute_bar
            WHERE instrument_id = $1 AND time >= $2 AND time <= $3
            ORDER BY time DESC
            "#,
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end);

        if let Some(limit_val) = limit {
            query = sqlx::query_as::<_, MinuteBar>(
                r#"
                SELECT instrument_id, time, open, high, low, close, volume, amount, open_interest
                FROM minute_bar
                WHERE instrument_id = $1 AND time >= $2 AND time <= $3
                ORDER BY time DESC
                LIMIT $4
                "#,
            )
            .bind(instrument_id)
            .bind(time_range.start)
            .bind(time_range.end)
            .bind(limit_val);
        }

        let mut result = query.fetch_all(&self.pool).await?;
        // 反轉結果，讓時間順序正確
        result.reverse();
        Ok(result)
    }

    /// 獲取股票分鐘K線數據的時間範圍
    pub async fn get_ohlcv_time_range_for_stock(
        &self,
        instrument_id: i32,
    ) -> Result<Option<TimeRange>> {
        let result = sqlx::query!(
            r#"
            SELECT MIN(time) as min_time, MAX(time) as max_time
            FROM minute_bar
            WHERE instrument_id = $1
            "#,
            instrument_id
        )
        .fetch_one(&self.pool)
        .await?;

        match (result.min_time, result.max_time) {
            (Some(min_time), Some(max_time)) => Ok(Some(TimeRange::new(min_time, max_time))),
            _ => Ok(None),
        }
    }

    /// 獲取Tick數據，從頭部開始取
    pub async fn get_ticks_head(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<Tick>> {
        let mut query = sqlx::query_as::<_, Tick>(
            r#"
            SELECT * FROM tick
            WHERE instrument_id = $1 AND time >= $2 AND time <= $3
            ORDER BY time ASC
            "#,
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end);

        if let Some(limit_val) = limit {
            query = sqlx::query_as::<_, Tick>(
                r#"
                SELECT * FROM tick
                WHERE instrument_id = $1 AND time >= $2 AND time <= $3
                ORDER BY time ASC
                LIMIT $4
                "#,
            )
            .bind(instrument_id)
            .bind(time_range.start)
            .bind(time_range.end)
            .bind(limit_val);
        }

        let result = query.fetch_all(&self.pool).await?;
        Ok(result)
    }

    /// 獲取Tick數據，從尾部開始取
    pub async fn get_ticks_tail(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<Tick>> {
        let mut query = sqlx::query_as::<_, Tick>(
            r#"
            SELECT * FROM tick
            WHERE instrument_id = $1 AND time >= $2 AND time <= $3
            ORDER BY time DESC
            "#,
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end);

        if let Some(limit_val) = limit {
            query = sqlx::query_as::<_, Tick>(
                r#"
                SELECT * FROM tick
                WHERE instrument_id = $1 AND time >= $2 AND time <= $3
                ORDER BY time DESC
                LIMIT $4
                "#,
            )
            .bind(instrument_id)
            .bind(time_range.start)
            .bind(time_range.end)
            .bind(limit_val);
        }

        let mut result = query.fetch_all(&self.pool).await?;
        // 反轉結果，讓時間順序正確
        result.reverse();
        Ok(result)
    }

    /// 獲取股票Tick數據的時間範圍
    pub async fn get_tick_time_range_for_stock(
        &self,
        instrument_id: i32,
    ) -> Result<Option<TimeRange>> {
        let result = sqlx::query!(
            r#"
            SELECT MIN(time) as min_time, MAX(time) as max_time
            FROM tick
            WHERE instrument_id = $1
            "#,
            instrument_id
        )
        .fetch_one(&self.pool)
        .await?;

        match (result.min_time, result.max_time) {
            (Some(min_time), Some(max_time)) => Ok(Some(TimeRange::new(min_time, max_time))),
            _ => Ok(None),
        }
    }
}

impl DbExecutor for PgMarketDataRepository {
    fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait::async_trait]
impl MarketDataRepository for PgMarketDataRepository {
    async fn get_minute_bars(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>> {
        let limit = limit.unwrap_or(1000);
        let bars = sqlx::query_as::<_, MinuteBar>(
            "SELECT * FROM minute_bar
             WHERE instrument_id = $1 AND time >= $2 AND time <= $3 
             ORDER BY time DESC 
             LIMIT $4",
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .bind(limit)
        .fetch_all(self.get_pool())
        .await?;

        Ok(bars)
    }

    async fn get_latest_minute_bar(&self, instrument_id: i32) -> Result<Option<MinuteBar>> {
        let bar = sqlx::query_as::<_, MinuteBar>(
            "SELECT * FROM minute_bar
             WHERE instrument_id = $1
             ORDER BY time DESC
             LIMIT 1",
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(bar)
    }

    async fn insert_minute_bars(&self, bars: &[MinuteBarInsert]) -> Result<()> {
        if bars.is_empty() {
            return Ok(());
        }

        let mut tx = self.get_pool().begin().await?;

        for bar in bars {
            sqlx::query(
                "INSERT INTO minute_bar (time, instrument_id, open, high, low, close, volume, amount, open_interest) 
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (time, instrument_id) DO UPDATE 
                 SET open = $3, high = $4, low = $5, close = $6, volume = $7, amount = $8, open_interest = $9"
            )
            .bind(bar.time)
            .bind(bar.instrument_id)
            .bind(bar.open)
            .bind(bar.high)
            .bind(bar.low)
            .bind(bar.close)
            .bind(bar.volume)
            .bind(bar.amount)
            .bind(bar.open_interest)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn batch_insert_minute_bars(
        &self,
        bars: &[MinuteBarInsert],
        batch_size: usize,
    ) -> Result<()> {
        if bars.is_empty() {
            return Ok(());
        }

        let mut tx = self.get_pool().begin().await?;

        for chunk in bars.chunks(batch_size) {
            let mut copy = sqlx::postgres::PgConnection::copy_in_raw(
                &mut tx,
                "COPY minute_bar (time, instrument_id, open, high, low, close, volume, amount, open_interest) FROM STDIN"
            ).await?;

            for bar in chunk {
                let line = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    bar.time.to_rfc3339(),
                    bar.instrument_id,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume,
                    bar.amount.unwrap_or_default(),
                    bar.open_interest.unwrap_or_default()
                );
                copy.send(line.as_bytes()).await?;
            }

            copy.finish().await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_ticks(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<Tick>> {
        let limit = limit.unwrap_or(10000);
        let ticks = sqlx::query_as::<_, Tick>(
            "SELECT * FROM tick
             WHERE instrument_id = $1 AND time >= $2 AND time <= $3 
             ORDER BY time DESC 
             LIMIT $4",
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .bind(limit)
        .fetch_all(self.get_pool())
        .await?;

        Ok(ticks)
    }

    async fn get_latest_tick(&self, instrument_id: i32) -> Result<Option<Tick>> {
        let tick = sqlx::query_as::<_, Tick>(
            "SELECT * FROM tick
             WHERE instrument_id = $1
             ORDER BY time DESC
             LIMIT 1",
        )
        .bind(instrument_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(tick)
    }

    async fn insert_ticks(&self, ticks: &[TickInsert]) -> Result<()> {
        if ticks.is_empty() {
            return Ok(());
        }

        let mut tx = self.get_pool().begin().await?;

        for tick in ticks {
            let bid_prices = match &tick.bid_prices {
                Some(prices) => serde_json::to_value(prices).unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };

            let bid_volumes = match &tick.bid_volumes {
                Some(volumes) => serde_json::to_value(volumes).unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };

            let ask_prices = match &tick.ask_prices {
                Some(prices) => serde_json::to_value(prices).unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };

            let ask_volumes = match &tick.ask_volumes {
                Some(volumes) => serde_json::to_value(volumes).unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };

            sqlx::query(
                "INSERT INTO tick (
                    time, instrument_id, price, volume, trade_type, 
                    bid_price_1, bid_volume_1, ask_price_1, ask_volume_1,
                    bid_prices, bid_volumes, ask_prices, ask_volumes,
                    open_interest, spread, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                 ON CONFLICT (time, instrument_id) DO UPDATE 
                 SET price = $3, volume = $4, trade_type = $5, 
                     bid_price_1 = $6, bid_volume_1 = $7, ask_price_1 = $8, ask_volume_1 = $9,
                     bid_prices = $10, bid_volumes = $11, ask_prices = $12, ask_volumes = $13,
                     open_interest = $14, spread = $15, metadata = $16",
            )
            .bind(tick.time)
            .bind(tick.instrument_id)
            .bind(tick.price)
            .bind(tick.volume)
            .bind(tick.trade_type)
            .bind(tick.bid_price_1)
            .bind(tick.bid_volume_1)
            .bind(tick.ask_price_1)
            .bind(tick.ask_volume_1)
            .bind(bid_prices)
            .bind(bid_volumes)
            .bind(ask_prices)
            .bind(ask_volumes)
            .bind(tick.open_interest)
            .bind(tick.spread)
            .bind(&tick.metadata)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn batch_insert_ticks(&self, ticks: &[TickInsert], batch_size: usize) -> Result<()> {
        if ticks.is_empty() {
            return Ok(());
        }

        let mut tx = self.get_pool().begin().await?;

        for chunk in ticks.chunks(batch_size) {
            // For large data volumes, we'll use simplified COPY format
            let mut copy = sqlx::postgres::PgConnection::copy_in_raw(
                &mut tx,
                "COPY tick (time, instrument_id, price, volume, trade_type) FROM STDIN",
            )
            .await?;

            for tick in chunk {
                let line = format!(
                    "{}\t{}\t{}\t{}\t{}\n",
                    tick.time.to_rfc3339(),
                    tick.instrument_id,
                    tick.price,
                    tick.volume,
                    tick.trade_type.unwrap_or_default()
                );
                copy.send(line.as_bytes()).await?;
            }

            copy.finish().await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_daily_bars(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<DailyBar>> {
        let bars = sqlx::query_as::<_, DailyBar>(
            "SELECT bucket::date as date, instrument_id, 
                    open, high, low, close, 
                    total_volume as volume, total_amount as amount
             FROM daily_volume_by_instrument
             WHERE instrument_id = $1 AND bucket >= $2 AND bucket <= $3
             ORDER BY bucket DESC",
        )
        .bind(instrument_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(self.get_pool())
        .await?;

        Ok(bars)
    }

    async fn get_technical_indicators(&self) -> Result<Vec<TechnicalIndicator>> {
        let indicators = sqlx::query_as::<_, TechnicalIndicator>(
            "SELECT * FROM technical_indicator ORDER BY code",
        )
        .fetch_all(self.get_pool())
        .await?;

        Ok(indicators)
    }

    async fn get_technical_indicator_by_id(
        &self,
        indicator_id: i32,
    ) -> Result<Option<TechnicalIndicator>> {
        let indicator = sqlx::query_as::<_, TechnicalIndicator>(
            "SELECT * FROM technical_indicator WHERE indicator_id = $1",
        )
        .bind(indicator_id)
        .fetch_optional(self.get_pool())
        .await?;

        Ok(indicator)
    }

    async fn get_instrument_daily_indicators(
        &self,
        instrument_id: i32,
        indicator_id: i32,
        time_range: TimeRange,
    ) -> Result<Vec<InstrumentDailyIndicator>> {
        let indicators = sqlx::query_as::<_, InstrumentDailyIndicator>(
            "SELECT * FROM instrument_daily_indicator
             WHERE instrument_id = $1 AND indicator_id = $2 AND time >= $3 AND time <= $4
             ORDER BY time DESC",
        )
        .bind(instrument_id)
        .bind(indicator_id)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(self.get_pool())
        .await?;

        Ok(indicators)
    }

    async fn insert_instrument_daily_indicator(
        &self,
        indicator: &InstrumentDailyIndicator,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO instrument_daily_indicator 
             (time, instrument_id, indicator_id, parameters, values) 
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (time, instrument_id, indicator_id) DO UPDATE 
             SET parameters = $4, values = $5",
        )
        .bind(indicator.time)
        .bind(indicator.instrument_id)
        .bind(indicator.indicator_id)
        .bind(&indicator.parameters)
        .bind(&indicator.values)
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn get_fundamental_indicators(
        &self,
        instrument_id: i32,
        indicator_type: &str,
        time_range: TimeRange,
    ) -> Result<Vec<FundamentalIndicator>> {
        let indicators = sqlx::query_as::<_, FundamentalIndicator>(
            "SELECT * FROM fundamental_indicator
             WHERE instrument_id = $1 AND indicator_type = $2 AND time >= $3 AND time <= $4
             ORDER BY time DESC",
        )
        .bind(instrument_id)
        .bind(indicator_type)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(self.get_pool())
        .await?;

        Ok(indicators)
    }

    async fn insert_fundamental_indicator(&self, indicator: &FundamentalIndicator) -> Result<()> {
        sqlx::query(
            "INSERT INTO fundamental_indicator 
             (time, instrument_id, indicator_type, values, source) 
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (time, instrument_id, indicator_type) DO UPDATE 
             SET values = $4, source = $5",
        )
        .bind(indicator.time)
        .bind(indicator.instrument_id)
        .bind(&indicator.indicator_type)
        .bind(&indicator.values)
        .bind(&indicator.source)
        .execute(self.get_pool())
        .await?;

        Ok(())
    }

    async fn get_minute_bars_head(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>> {
        self.get_minute_bars_head(instrument_id, time_range, limit)
            .await
    }

    async fn get_minute_bars_tail(
        &self,
        instrument_id: i32,
        time_range: TimeRange,
        limit: Option<i64>,
    ) -> Result<Vec<MinuteBar>> {
        self.get_minute_bars_tail(instrument_id, time_range, limit)
            .await
    }
}
