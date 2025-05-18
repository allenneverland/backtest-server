use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use chrono::{DateTime, Utc};

// 頂層 DSL 結構
#[derive(Debug, Serialize, Deserialize)]
pub struct TradingDSL {
    pub dsl_version: String,
    pub created_at: DateTime<Utc>,
    pub author: String,
    pub strategy: Strategy,
}

// 策略定義
#[derive(Debug, Serialize, Deserialize)]
pub struct Strategy {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub universe: Option<Universe>,
    pub execution: Option<Execution>,
    pub parameters: Option<HashMap<String, Value>>,
    pub assets: Option<Assets>,
    pub indicators: Option<Vec<Indicator>>,
    pub conditions: Option<Conditions>,
    pub actions: Option<Actions>,
    pub risk_management: Option<RiskManagement>,
    pub state_machine: Option<StateMachine>,
    pub loops: Option<Vec<Loop>>,
    pub signals: Option<HashMap<String, Signal>>,
}

// 資產配置
#[derive(Debug, Serialize, Deserialize)]
pub struct Universe {
    pub asset_classes: Vec<AssetClass>,
    pub markets: Vec<String>,
    pub exchanges: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetClass {
    Stocks,
    Futures,
    Crypto,
    Forex,
    Options,
}

// 執行環境
#[derive(Debug, Serialize, Deserialize)]
pub struct Execution {
    pub mode: ExecutionMode,
    pub frequency: Frequency,
    pub timezone: Option<String>,
    pub slippage: Option<SlippageModel>,
    pub commission: Option<Commission>,
    pub constraints: Option<ExecutionConstraints>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Backtest,
    Paper,
    Live,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    Tick,
    Second,
    Minute,
    Hour,
    Daily,
}

// 資產定義
#[derive(Debug, Serialize, Deserialize)]
pub struct Assets {
    pub stocks: Option<StockAssets>,
    pub futures: Option<FuturesAssets>,
    pub crypto: Option<CryptoAssets>,
    pub forex: Option<ForexAssets>,
    pub options: Option<OptionsAssets>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StockAssets {
    pub symbols: Vec<String>,
    pub filters: Option<HashMap<String, Filter>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Filter {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

// 指標系統
#[derive(Debug, Serialize, Deserialize)]
pub struct Indicator {
    pub name: String,
    pub params: Option<HashMap<String, Value>>,
    pub formula: Option<String>,
    pub inputs: Option<Vec<String>>,
    pub output: Option<String>,
}

// 條件系統
#[derive(Debug, Serialize, Deserialize)]
pub struct Conditions {
    pub comparisons: Option<Vec<String>>,
    pub logical: Option<LogicalConditions>,
    pub temporal: Option<TemporalConditions>,
    pub cross_asset: Option<Vec<String>>,
    pub market_state: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogicalConditions {
    pub all_of: Option<Vec<String>>,
    pub any_of: Option<Vec<String>>,
    pub none_of: Option<Vec<String>>,
}

// 動作系統
#[derive(Debug, Serialize, Deserialize)]
pub struct Actions {
    pub basic: Option<Vec<BasicAction>>,
    pub advanced: Option<Vec<AdvancedAction>>,
    pub futures_specific: Option<Vec<Action>>,
    pub options_specific: Option<Vec<Action>>,
    pub crypto_specific: Option<Vec<Action>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub quantity: Quantity,
    pub order_type: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Quantity {
    Fixed(i32),
    All(String),
    Calculated(HashMap<String, String>),
}

// 風險管理
#[derive(Debug, Serialize, Deserialize)]
pub struct RiskManagement {
    pub position_level: Option<PositionRisk>,
    pub strategy_level: Option<StrategyRisk>,
    pub portfolio_level: Option<PortfolioRisk>,
    pub dynamic_sizing: Option<DynamicSizing>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionRisk {
    pub stop_loss: Option<StopLoss>,
    pub take_profit: Option<TakeProfit>,
    pub max_holding_period: Option<String>,
}

// 狀態機
#[derive(Debug, Serialize, Deserialize)]
pub struct StateMachine {
    pub initial: String,
    pub states: HashMap<String, State>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub description: String,
    pub sub_states: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub trigger: TriggerType,
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TriggerType {
    Single(String),
    Multiple(Vec<String>),
}

// 迴圈結構
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Loop {
    #[serde(rename = "for")]
    For {
        name: String,
        iterator: String,
        values: Vec<Value>,
        condition: Option<Vec<String>>,
        action: Option<Action>,
        body: Option<Vec<Loop>>,
    },
    #[serde(rename = "for_range")]
    ForRange {
        name: String,
        variable: String,
        from: f64,
        to: f64,
        step: f64,
        action: Option<Action>,
        body: Option<Vec<Loop>>,
    },
    #[serde(rename = "foreach")]
    ForEach {
        name: String,
        collection: String,
        #[serde(rename = "as")]
        as_variable: String,
        when: Option<Vec<String>>,
        action: Option<Action>,
        body: Option<Vec<Loop>>,
    },
    #[serde(rename = "while")]
    While {
        name: String,
        condition: Vec<String>,
        max_iterations: Option<i32>,
        body: Vec<Action>,
    },
}

// 通用值類型
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

// 動作類型
#[derive(Debug, Serialize, Deserialize)]
pub struct Action {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(flatten)]
    pub params: HashMap<String, Value>,
}

// 信號
#[derive(Debug, Serialize, Deserialize)]
pub struct Signal {
    pub name: String,
    pub when: SignalCondition,
    pub actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignalCondition {
    pub all_of: Option<Vec<String>>,
    pub any_of: Option<Vec<String>>,
}

// 其他支援結構
#[derive(Debug, Serialize, Deserialize)]
pub struct FuturesAssets {
    pub contracts: Vec<String>,
    pub month_codes: Vec<String>,
    pub filters: Option<HashMap<String, Filter>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoAssets {
    pub pairs: Vec<String>,
    pub exchanges: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForexAssets {
    pub pairs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptionsAssets {
    pub underlying: Vec<String>,
    pub types: Vec<String>,
    pub expiry: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdvancedAction {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(flatten)]
    pub params: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemporalConditions {
    pub time_of_day: Option<TimeRange>,
    pub days_of_week: Option<Vec<String>>,
    pub exclude_dates: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopLoss {
    #[serde(rename = "type")]
    pub stop_type: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TakeProfit {
    #[serde(rename = "type")]
    pub profit_type: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyRisk {
    pub max_positions: i32,
    pub max_exposure: f64,
    pub max_sector_exposure: Option<f64>,
    pub max_correlation: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioRisk {
    pub max_drawdown: f64,
    pub var_limit: Option<f64>,
    pub sharpe_ratio_target: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DynamicSizing {
    pub volatility_adjusted: bool,
    pub kelly_criterion: Option<f64>,
    pub risk_parity: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlippageModel {
    pub model: String,
    pub parameter: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commission {
    pub stocks: Option<StockCommission>,
    pub futures: Option<FuturesCommission>,
    pub crypto: Option<CryptoCommission>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StockCommission {
    pub rate: f64,
    pub minimum: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FuturesCommission {
    pub per_contract: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoCommission {
    pub maker: f64,
    pub taker: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionConstraints {
    pub min_order_size: Option<i32>,
    pub max_order_size: Option<i32>,
    pub max_orders_per_minute: Option<i32>,
    pub trading_hours: Option<HashMap<String, String>>,
}

// 解析器實現
pub struct DSLParser;

impl DSLParser {
    pub fn new() -> Self {
        DSLParser
    }

    /// 從 YAML 字符串解析 DSL
    pub fn parse_from_string(&self, yaml_str: &str) -> Result<TradingDSL, Box<dyn Error>> {
        let dsl: TradingDSL = serde_yaml::from_str(yaml_str)?;
        Ok(dsl)
    }

    /// 從文件路徑解析 DSL
    pub fn parse_from_file(&self, file_path: &str) -> Result<TradingDSL, Box<dyn Error>> {
        let contents = fs::read_to_string(file_path)?;
        self.parse_from_string(&contents)
    }

    /// 將 DSL 結構序列化為 YAML 字符串
    pub fn serialize_to_string(&self, dsl: &TradingDSL) -> Result<String, Box<dyn Error>> {
        let yaml = serde_yaml::to_string(dsl)?;
        Ok(yaml)
    }

    /// 將 DSL 結構序列化並寫入文件
    pub fn serialize_to_file(&self, dsl: &TradingDSL, file_path: &str) -> Result<(), Box<dyn Error>> {
        let yaml = self.serialize_to_string(dsl)?;
        fs::write(file_path, yaml)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_strategy() {
        let yaml = r#"
dsl_version: "2.0"
created_at: "2024-01-01T00:00:00Z"
author: "test_author"
strategy:
  name: "Simple Strategy"
  version: "1.0"
  description: "A simple test strategy"
  tags: ["test", "simple"]
  universe:
    asset_classes: ["stocks", "futures"]
    markets: ["US", "TW"]
"#;

        let parser = DSLParser::new();
        let result = parser.parse_from_string(yaml);
        assert!(result.is_ok());
        
        let dsl = result.unwrap();
        assert_eq!(dsl.strategy.name, "Simple Strategy");
        assert_eq!(dsl.strategy.version, "1.0");
    }

    #[test]
    fn test_parse_with_indicators() {
        let yaml = r#"
dsl_version: "2.0"
created_at: "2024-01-01T00:00:00Z"
author: "test_author"
strategy:
  name: "Indicator Strategy"
  version: "1.0"
  indicators:
    - name: ma
      params:
        period: 20
        type: simple
    - name: rsi
      params:
        period: 14
"#;

        let parser = DSLParser::new();
        let result = parser.parse_from_string(yaml);
        assert!(result.is_ok());
        
        let dsl = result.unwrap();
        assert!(dsl.strategy.indicators.is_some());
        let indicators = dsl.strategy.indicators.unwrap();
        assert_eq!(indicators.len(), 2);
        assert_eq!(indicators[0].name, "ma");
    }

    #[test]
    fn test_serialize_strategy() {
        let strategy = Strategy {
            name: "Test Strategy".to_string(),
            version: "1.0".to_string(),
            description: Some("Test description".to_string()),
            tags: Some(vec!["test".to_string()]),
            universe: None,
            execution: None,
            parameters: None,
            assets: None,
            indicators: None,
            conditions: None,
            actions: None,
            risk_management: None,
            state_machine: None,
            loops: None,
            signals: None,
        };

        let dsl = TradingDSL {
            dsl_version: "2.0".to_string(),
            created_at: Utc::now(),
            author: "test".to_string(),
            strategy,
        };

        let parser = DSLParser::new();
        let result = parser.serialize_to_string(&dsl);
        assert!(result.is_ok());
    }
}