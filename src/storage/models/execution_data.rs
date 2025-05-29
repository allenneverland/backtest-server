use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::{Decimal, Json};

/// 執行交易記錄模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionTrade {
    pub time: DateTime<Utc>,
    pub run_id: i32,
    pub instrument_id: i32,
    pub direction: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
    pub slippage: Option<Decimal>,
    pub trade_id: Option<String>,
    pub position_effect: Option<String>,
    pub order_type: Option<String>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 執行交易記錄插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTradeInsert {
    pub time: DateTime<Utc>,
    pub run_id: i32,
    pub instrument_id: i32,
    pub direction: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
    pub slippage: Option<Decimal>,
    pub trade_id: Option<String>,
    pub position_effect: Option<String>,
    pub order_type: Option<String>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 執行倉位快照模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionPosition {
    pub time: DateTime<Utc>,
    pub run_id: i32,
    pub instrument_id: i32,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub unrealized_pl: Decimal,
    pub realized_pl: Decimal,
    pub margin_used: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

/// 執行倉位快照插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPositionInsert {
    pub time: DateTime<Utc>,
    pub run_id: i32,
    pub instrument_id: i32,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_value: Decimal,
    pub unrealized_pl: Decimal,
    pub realized_pl: Decimal,
    pub margin_used: Option<Decimal>,
}

/// 執行投資組合快照模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionPortfolio {
    pub time: DateTime<Utc>,
    pub run_id: i32,
    pub total_value: Decimal,
    pub cash: Decimal,
    pub equity: Decimal,
    pub margin: Option<Decimal>,
    pub daily_pnl: Option<Decimal>,
    pub total_pnl: Option<Decimal>,
    pub daily_return: Option<Decimal>,
    pub total_return: Option<Decimal>,
    pub metadata: Option<Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// 執行投資組合快照插入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPortfolioInsert {
    pub time: DateTime<Utc>,
    pub run_id: i32,
    pub total_value: Decimal,
    pub cash: Decimal,
    pub equity: Decimal,
    pub margin: Option<Decimal>,
    pub daily_pnl: Option<Decimal>,
    pub total_pnl: Option<Decimal>,
    pub daily_return: Option<Decimal>,
    pub total_return: Option<Decimal>,
    pub metadata: Option<Json<serde_json::Value>>,
}

/// 執行日收益率聚合模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionDailyReturns {
    pub bucket: DateTime<Utc>,
    pub run_id: i32,
    pub daily_return: Decimal,
    pub end_of_day_value: Decimal,
    pub end_of_day_equity: Decimal,
}

/// 交易方向枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeDirection {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

impl TradeDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeDirection::Buy => "BUY",
            TradeDirection::Sell => "SELL",
        }
    }
}

impl From<String> for TradeDirection {
    fn from(s: String) -> Self {
        match s.to_uppercase().as_str() {
            "BUY" => TradeDirection::Buy,
            "SELL" => TradeDirection::Sell,
            _ => TradeDirection::Buy,
        }
    }
}

/// 倉位效果枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionEffect {
    #[serde(rename = "OPEN")]
    Open,
    #[serde(rename = "CLOSE")]
    Close,
    #[serde(rename = "ADJUST")]
    Adjust,
}

impl PositionEffect {
    pub fn as_str(&self) -> &'static str {
        match self {
            PositionEffect::Open => "OPEN",
            PositionEffect::Close => "CLOSE",
            PositionEffect::Adjust => "ADJUST",
        }
    }
}

/// 訂單類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "MARKET")]
    Market,
    #[serde(rename = "LIMIT")]
    Limit,
    #[serde(rename = "STOP")]
    Stop,
}

impl OrderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderType::Market => "MARKET",
            OrderType::Limit => "LIMIT",
            OrderType::Stop => "STOP",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_direction_conversion() {
        assert_eq!(TradeDirection::Buy.as_str(), "BUY");
        assert_eq!(TradeDirection::Sell.as_str(), "SELL");

        assert_eq!(TradeDirection::from("BUY".to_string()), TradeDirection::Buy);
        assert_eq!(TradeDirection::from("sell".to_string()), TradeDirection::Sell);
    }

    #[test]
    fn test_position_effect_conversion() {
        assert_eq!(PositionEffect::Open.as_str(), "OPEN");
        assert_eq!(PositionEffect::Close.as_str(), "CLOSE");
        assert_eq!(PositionEffect::Adjust.as_str(), "ADJUST");
    }

    #[test]
    fn test_order_type_conversion() {
        assert_eq!(OrderType::Market.as_str(), "MARKET");
        assert_eq!(OrderType::Limit.as_str(), "LIMIT");
        assert_eq!(OrderType::Stop.as_str(), "STOP");
    }
}