// utils/serde_helpers.rs - 序列化與反序列化輔助函數
use serde::{Deserialize, Deserializer};

/// 將空字符串反序列化為None
///
/// 在配置文件中，經常需要將空字符串解析為None以表示不存在的值。
/// 這個函數可用於serde的自定義反序列化器。
///
/// # 參數
///
/// * `deserializer` - 反序列化器
///
/// # 返回值
///
/// * `Result<Option<String>, D::Error>` - 解析結果，空字符串解析為None
///
/// # 使用範例
///
/// ```
/// use serde::Deserialize;
/// use crate::utils::serde_helpers::empty_string_as_none;
///
/// #[derive(Deserialize)]
/// struct Config {
///     #[serde(deserialize_with = "empty_string_as_none")]
///     optional_value: Option<String>,
/// }
/// ```
pub fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
} 