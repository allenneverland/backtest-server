//! 金融商品結構定義

use super::types::AssetType;
use serde::{Serialize, Deserialize};

/// 金融商品定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub instrument_id: String,
    pub symbol: String,
    pub exchange: String, 
    pub asset_type: AssetType,
    pub name: String,
    pub is_active: bool,
    pub currency: String,
    pub attributes: serde_json::Value, // 存儲特定資產類型的附加屬性
}

impl Instrument {
    pub fn new(
        instrument_id: impl Into<String>,
        symbol: impl Into<String>,
        exchange: impl Into<String>,
        asset_type: AssetType,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            symbol: symbol.into(),
            exchange: exchange.into(),
            asset_type,
            name: String::new(),
            is_active: true,
            currency: "USD".to_string(),
            attributes: serde_json::Value::Null,
        }
    }
    
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }
    
    pub fn with_attributes(mut self, attributes: serde_json::Value) -> Self {
        self.attributes = attributes;
        self
    }
}