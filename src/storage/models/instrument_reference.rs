use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 金融商品參考模型（回測資料庫中的輕量級版本）
/// 此模型對應 instrument_reference 表，用於回測系統
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InstrumentReference {
    pub instrument_id: i32,
    pub symbol: String,
    pub exchange_code: String,
    pub instrument_type: String,
    pub name: String,
    pub currency: String,
    pub is_active: bool,
    /// 最後一次成功同步的時間
    pub last_sync_at: DateTime<Utc>,
    /// 同步版本號，用於增量更新
    pub sync_version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InstrumentReference {
    /// 創建新的金融商品參考
    pub fn new(
        instrument_id: i32,
        symbol: String,
        exchange_code: String,
        instrument_type: String,
        name: String,
        currency: String,
    ) -> Self {
        let now = Utc::now();

        Self {
            instrument_id,
            symbol,
            exchange_code,
            instrument_type,
            name,
            currency,
            is_active: true,
            last_sync_at: now,
            sync_version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    /// 檢查是否為股票
    pub fn is_stock(&self) -> bool {
        self.instrument_type == "STOCK"
    }

    /// 檢查是否為期貨
    pub fn is_future(&self) -> bool {
        self.instrument_type == "FUTURE"
    }

    /// 檢查是否為選擇權
    pub fn is_option(&self) -> bool {
        self.instrument_type == "OPTIONCONTRACT"
    }

    /// 檢查是否為外匯
    pub fn is_forex(&self) -> bool {
        self.instrument_type == "FOREX"
    }

    /// 檢查是否為加密貨幣
    pub fn is_crypto(&self) -> bool {
        self.instrument_type == "CRYPTO"
    }

    /// 獲取市場識別符（符號 + 交易所）
    pub fn market_identifier(&self) -> String {
        format!("{}:{}", self.exchange_code, self.symbol)
    }

    /// 更新同步版本和時間
    pub fn update_sync(&mut self, sync_version: i64) {
        self.sync_version = sync_version;
        self.last_sync_at = Utc::now();
        self.updated_at = Utc::now();
    }

    /// 停用商品
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }

    /// 啟用商品
    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_reference_creation() {
        let instrument = InstrumentReference::new(
            1,
            "AAPL".to_string(),
            "NASDAQ".to_string(),
            "STOCK".to_string(),
            "Apple Inc.".to_string(),
            "USD".to_string(),
        );

        assert_eq!(instrument.instrument_id, 1);
        assert_eq!(instrument.symbol, "AAPL");
        assert_eq!(instrument.exchange_code, "NASDAQ");
        assert_eq!(instrument.instrument_type, "STOCK");
        assert_eq!(instrument.name, "Apple Inc.");
        assert_eq!(instrument.currency, "USD");
        assert!(instrument.is_active);
        assert_eq!(instrument.sync_version, 1);
    }

    #[test]
    fn test_instrument_type_checks() {
        let stock = InstrumentReference::new(
            1,
            "AAPL".to_string(),
            "NASDAQ".to_string(),
            "STOCK".to_string(),
            "Apple".to_string(),
            "USD".to_string(),
        );
        assert!(stock.is_stock());
        assert!(!stock.is_future());

        let future = InstrumentReference::new(
            2,
            "ES".to_string(),
            "CME".to_string(),
            "FUTURE".to_string(),
            "E-mini S&P 500".to_string(),
            "USD".to_string(),
        );
        assert!(future.is_future());
        assert!(!future.is_stock());

        let option = InstrumentReference::new(
            3,
            "AAPL240315C00150000".to_string(),
            "NASDAQ".to_string(),
            "OPTIONCONTRACT".to_string(),
            "AAPL Call".to_string(),
            "USD".to_string(),
        );
        assert!(option.is_option());

        let forex = InstrumentReference::new(
            4,
            "EURUSD".to_string(),
            "FOREX".to_string(),
            "FOREX".to_string(),
            "Euro/US Dollar".to_string(),
            "USD".to_string(),
        );
        assert!(forex.is_forex());

        let crypto = InstrumentReference::new(
            5,
            "BTCUSD".to_string(),
            "BINANCE".to_string(),
            "CRYPTO".to_string(),
            "Bitcoin".to_string(),
            "USD".to_string(),
        );
        assert!(crypto.is_crypto());
    }

    #[test]
    fn test_market_identifier() {
        let instrument = InstrumentReference::new(
            1,
            "AAPL".to_string(),
            "NASDAQ".to_string(),
            "STOCK".to_string(),
            "Apple".to_string(),
            "USD".to_string(),
        );
        assert_eq!(instrument.market_identifier(), "NASDAQ:AAPL");
    }

    #[test]
    fn test_sync_update() {
        let mut instrument = InstrumentReference::new(
            1,
            "AAPL".to_string(),
            "NASDAQ".to_string(),
            "STOCK".to_string(),
            "Apple".to_string(),
            "USD".to_string(),
        );

        let original_sync_time = instrument.last_sync_at;
        let original_update_time = instrument.updated_at;

        // 模擬時間流逝
        std::thread::sleep(std::time::Duration::from_millis(1));

        instrument.update_sync(2);

        assert_eq!(instrument.sync_version, 2);
        assert!(instrument.last_sync_at > original_sync_time);
        assert!(instrument.updated_at > original_update_time);
    }

    #[test]
    fn test_activation_deactivation() {
        let mut instrument = InstrumentReference::new(
            1,
            "AAPL".to_string(),
            "NASDAQ".to_string(),
            "STOCK".to_string(),
            "Apple".to_string(),
            "USD".to_string(),
        );

        assert!(instrument.is_active);

        let original_update_time = instrument.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));

        instrument.deactivate();
        assert!(!instrument.is_active);
        assert!(instrument.updated_at > original_update_time);

        let deactivate_time = instrument.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));

        instrument.activate();
        assert!(instrument.is_active);
        assert!(instrument.updated_at > deactivate_time);
    }
}
