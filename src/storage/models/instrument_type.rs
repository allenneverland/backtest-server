use serde::{Deserialize, Serialize};
use sqlx::encode::IsNull;
use std::error::Error;
use std::fmt;

/// 金融商品類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentType {
    /// 股票
    STOCK,
    /// 期貨
    FUTURE,
    /// 選擇權
    OPTIONCONTRACT,
    /// 外匯
    FOREX,
    /// 虛擬貨幣
    CRYPTO,
}

impl fmt::Display for InstrumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstrumentType::STOCK => write!(f, "STOCK"),
            InstrumentType::FUTURE => write!(f, "FUTURE"),
            InstrumentType::OPTIONCONTRACT => write!(f, "OPTIONCONTRACT"),
            InstrumentType::FOREX => write!(f, "FOREX"),
            InstrumentType::CRYPTO => write!(f, "CRYPTO"),
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for InstrumentType {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("VARCHAR")
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for InstrumentType {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        match s.as_str() {
            "STOCK" => Ok(InstrumentType::STOCK),
            "FUTURE" => Ok(InstrumentType::FUTURE),
            "OPTIONCONTRACT" => Ok(InstrumentType::OPTIONCONTRACT),
            "FOREX" => Ok(InstrumentType::FOREX),
            "CRYPTO" => Ok(InstrumentType::CRYPTO),
            _ => Err(format!("未知的金融商品類型: {}", s).into()),
        }
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for InstrumentType {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<IsNull, Box<dyn Error + Send + Sync>> {
        let s = self.to_string();
        <String as sqlx::Encode<sqlx::Postgres>>::encode(s, buf)
    }
}
