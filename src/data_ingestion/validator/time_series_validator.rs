use super::error::{ValidationError, ValidationErrors, ValidationResult};
use super::traits::{ComposableValidator, ValidationConfig, Validator};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::marker::PhantomData;

/// 通用時間序列記錄特徵
pub trait TimeSeriesRecord: Send + Sync {
    /// 獲取時間戳記
    fn timestamp(&self) -> &DateTime<Utc>;

    /// 獲取記錄的唯一標識（用於檢測重複）
    fn unique_key(&self) -> String {
        self.timestamp().to_rfc3339()
    }
}

/// 時間序列驗證器
pub struct TimeSeriesValidator<T: TimeSeriesRecord> {
    config: ValidationConfig,
    /// 最小時間間隔（毫秒）
    min_gap_ms: Option<i64>,
    /// 最大時間間隔（毫秒）
    max_gap_ms: Option<i64>,
    /// 是否允許重複時間戳記
    allow_duplicates: bool,
    /// 是否要求嚴格遞增
    strict_ascending: bool,
    /// 最早允許的時間
    min_timestamp: Option<DateTime<Utc>>,
    /// 最晚允許的時間
    max_timestamp: Option<DateTime<Utc>>,
    _phantom: PhantomData<T>,
}

impl<T: TimeSeriesRecord> TimeSeriesValidator<T> {
    /// 創建新的時間序列驗證器
    pub fn new() -> Self {
        let config = ValidationConfig::default()
            .with_param("allow_duplicates", serde_json::json!(false))
            .with_param("strict_ascending", serde_json::json!(true));

        Self {
            config,
            min_gap_ms: None,
            max_gap_ms: None,
            allow_duplicates: false,
            strict_ascending: true,
            min_timestamp: None,
            max_timestamp: None,
            _phantom: PhantomData,
        }
    }

    /// 設置最小時間間隔
    pub fn with_min_gap(mut self, duration: Duration) -> Self {
        let ms = duration.num_milliseconds();
        self.min_gap_ms = Some(ms);
        self.config = self.config.with_param("min_gap_ms", serde_json::json!(ms));
        self
    }

    /// 設置最大時間間隔
    pub fn with_max_gap(mut self, duration: Duration) -> Self {
        let ms = duration.num_milliseconds();
        self.max_gap_ms = Some(ms);
        self.config = self.config.with_param("max_gap_ms", serde_json::json!(ms));
        self
    }

    /// 設置是否允許重複
    pub fn with_allow_duplicates(mut self, allow: bool) -> Self {
        self.allow_duplicates = allow;
        self.config = self
            .config
            .with_param("allow_duplicates", serde_json::json!(allow));
        self
    }

    /// 設置是否要求嚴格遞增
    pub fn with_strict_ascending(mut self, strict: bool) -> Self {
        self.strict_ascending = strict;
        self.config = self
            .config
            .with_param("strict_ascending", serde_json::json!(strict));
        self
    }

    /// 設置時間範圍
    pub fn with_time_range(
        mut self,
        min: Option<DateTime<Utc>>,
        max: Option<DateTime<Utc>>,
    ) -> Self {
        self.min_timestamp = min;
        self.max_timestamp = max;

        if let Some(min) = min {
            self.config = self
                .config
                .with_param("min_timestamp", serde_json::json!(min.to_rfc3339()));
        }
        if let Some(max) = max {
            self.config = self
                .config
                .with_param("max_timestamp", serde_json::json!(max.to_rfc3339()));
        }

        self
    }

    /// 驗證時間戳記範圍
    fn validate_timestamp_range(&self, timestamp: &DateTime<Utc>) -> ValidationResult<()> {
        if let Some(min) = self.min_timestamp {
            if timestamp < &min {
                return Err(ValidationError::InvalidTimestamp {
                    timestamp: timestamp.to_rfc3339(),
                    reason: format!("時間戳記早於最小允許時間 {}", min.to_rfc3339()),
                });
            }
        }

        if let Some(max) = self.max_timestamp {
            if timestamp > &max {
                return Err(ValidationError::InvalidTimestamp {
                    timestamp: timestamp.to_rfc3339(),
                    reason: format!("時間戳記晚於最大允許時間 {}", max.to_rfc3339()),
                });
            }
        }

        Ok(())
    }

    /// 驗證時間序列
    pub fn validate_series(&self, records: &[T]) -> Result<TimeSeriesStats, ValidationErrors> {
        if records.is_empty() {
            return Ok(TimeSeriesStats::default());
        }

        let mut errors = ValidationErrors::new();
        let mut seen_timestamps = HashSet::new();
        let mut stats = TimeSeriesStats::default();

        // 第一筆記錄
        let first = &records[0];
        stats.start_time = Some(*first.timestamp());
        stats.total_records = records.len();

        // 驗證第一筆記錄的時間範圍
        if let Err(e) = self.validate_timestamp_range(first.timestamp()) {
            errors.add(0, e);
        }

        seen_timestamps.insert(first.unique_key());

        // 驗證後續記錄
        for (i, record) in records.iter().enumerate().skip(1) {
            let timestamp = record.timestamp();
            let prev_timestamp = records[i - 1].timestamp();

            // 驗證時間範圍
            if let Err(e) = self.validate_timestamp_range(timestamp) {
                errors.add(i, e);
            }

            // 檢查時間順序
            let is_out_of_order = if self.strict_ascending {
                timestamp <= prev_timestamp
            } else {
                timestamp < prev_timestamp
            };

            if is_out_of_order {
                errors.add(
                    i,
                    ValidationError::OutOfOrder {
                        previous: prev_timestamp.to_rfc3339(),
                        current: timestamp.to_rfc3339(),
                    },
                );
            }

            // 檢查重複
            let key = record.unique_key();
            if !self.allow_duplicates && seen_timestamps.contains(&key) {
                errors.add(
                    i,
                    ValidationError::DuplicateEntry {
                        timestamp: timestamp.to_rfc3339(),
                    },
                );
                stats.duplicate_count += 1;
            }
            seen_timestamps.insert(key);

            // 計算時間間隔
            let gap_ms = (*timestamp - *prev_timestamp).num_milliseconds();

            // 更新統計
            if gap_ms > 0 {
                stats.gaps.push(gap_ms);
                stats.min_gap_ms = Some(stats.min_gap_ms.map_or(gap_ms, |min| min.min(gap_ms)));
                stats.max_gap_ms = Some(stats.max_gap_ms.map_or(gap_ms, |max| max.max(gap_ms)));
            }

            // 檢查最小間隔
            if let Some(min_gap) = self.min_gap_ms {
                if gap_ms > 0 && gap_ms < min_gap {
                    errors.add(
                        i,
                        ValidationError::InvalidTimestamp {
                            timestamp: timestamp.to_rfc3339(),
                            reason: format!(
                                "時間間隔 {} 毫秒小於最小允許值 {} 毫秒",
                                gap_ms, min_gap
                            ),
                        },
                    );
                }
            }

            // 檢查最大間隔
            if let Some(max_gap) = self.max_gap_ms {
                if gap_ms > max_gap {
                    errors.add(
                        i,
                        ValidationError::LargeGap {
                            gap_seconds: gap_ms / 1000,
                            max_gap_seconds: max_gap / 1000,
                        },
                    );
                    stats.gap_violations += 1;
                }
            }
        }

        // 設置結束時間
        if let Some(last) = records.last() {
            stats.end_time = Some(*last.timestamp());
        }

        // 計算平均間隔
        if !stats.gaps.is_empty() {
            let sum: i64 = stats.gaps.iter().sum();
            stats.avg_gap_ms = Some(sum / stats.gaps.len() as i64);
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(stats)
        }
    }
}

impl<T: TimeSeriesRecord> Default for TimeSeriesValidator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TimeSeriesRecord + 'static> Validator for TimeSeriesValidator<T> {
    type Data = T;

    fn name(&self) -> &str {
        "TimeSeriesValidator"
    }

    fn validate_record(&self, record: &Self::Data) -> ValidationResult<()> {
        self.validate_timestamp_range(record.timestamp())
    }

    fn validate_batch(&self, data: &[Self::Data]) -> Result<(), ValidationErrors> {
        self.validate_series(data).map(|_| ())
    }

    fn config(&self) -> &ValidationConfig {
        &self.config
    }
}

impl<T: TimeSeriesRecord + 'static> ComposableValidator for TimeSeriesValidator<T> {}

/// 時間序列統計資訊
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TimeSeriesStats {
    /// 總記錄數
    pub total_records: usize,
    /// 開始時間
    pub start_time: Option<DateTime<Utc>>,
    /// 結束時間
    pub end_time: Option<DateTime<Utc>>,
    /// 重複記錄數
    pub duplicate_count: usize,
    /// 時間間隔違規次數
    pub gap_violations: usize,
    /// 最小間隔（毫秒）
    pub min_gap_ms: Option<i64>,
    /// 最大間隔（毫秒）
    pub max_gap_ms: Option<i64>,
    /// 平均間隔（毫秒）
    pub avg_gap_ms: Option<i64>,
    /// 所有間隔列表
    #[serde(skip)]
    pub gaps: Vec<i64>,
}

impl TimeSeriesStats {
    /// 獲取總時長
    pub fn duration(&self) -> Option<Duration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }

    /// 獲取完整性百分比（基於預期間隔）
    pub fn completeness_percent(&self, expected_gap_ms: i64) -> Option<f64> {
        self.duration().map(|duration| {
            let expected_records = (duration.num_milliseconds() / expected_gap_ms) + 1;
            (self.total_records as f64 / expected_records as f64) * 100.0
        })
    }
}

/// 為 OHLCV 和 Tick 記錄實現 TimeSeriesRecord
impl TimeSeriesRecord for super::ohlcv_validator::OhlcvRecord {
    fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
}

impl TimeSeriesRecord for super::tick_validator::TickRecord {
    fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestRecord {
        timestamp: DateTime<Utc>,
        #[allow(dead_code)]
        value: f64,
    }

    impl TimeSeriesRecord for TestRecord {
        fn timestamp(&self) -> &DateTime<Utc> {
            &self.timestamp
        }
    }

    #[test]
    fn test_valid_series() {
        let validator = TimeSeriesValidator::<TestRecord>::new();
        let now = Utc::now();

        let records = vec![
            TestRecord {
                timestamp: now,
                value: 100.0,
            },
            TestRecord {
                timestamp: now + Duration::seconds(1),
                value: 101.0,
            },
            TestRecord {
                timestamp: now + Duration::seconds(2),
                value: 102.0,
            },
        ];

        let result = validator.validate_series(&records);
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_records, 3);
        assert_eq!(stats.duplicate_count, 0);
        assert_eq!(stats.gap_violations, 0);
    }

    #[test]
    fn test_out_of_order() {
        let validator = TimeSeriesValidator::<TestRecord>::new();
        let now = Utc::now();

        let records = vec![
            TestRecord {
                timestamp: now,
                value: 100.0,
            },
            TestRecord {
                timestamp: now + Duration::seconds(2),
                value: 102.0,
            },
            TestRecord {
                timestamp: now + Duration::seconds(1),
                value: 101.0,
            }, // out of order
        ];

        let result = validator.validate_series(&records);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicates() {
        let validator = TimeSeriesValidator::<TestRecord>::new().with_allow_duplicates(false);
        let now = Utc::now();

        let records = vec![
            TestRecord {
                timestamp: now,
                value: 100.0,
            },
            TestRecord {
                timestamp: now,
                value: 101.0,
            }, // duplicate timestamp
            TestRecord {
                timestamp: now + Duration::seconds(1),
                value: 102.0,
            },
        ];

        let result = validator.validate_series(&records);
        assert!(result.is_err());
    }

    #[test]
    fn test_gap_validation() {
        let validator = TimeSeriesValidator::<TestRecord>::new().with_max_gap(Duration::seconds(5));
        let now = Utc::now();

        let records = vec![
            TestRecord {
                timestamp: now,
                value: 100.0,
            },
            TestRecord {
                timestamp: now + Duration::seconds(10),
                value: 101.0,
            }, // gap too large
        ];

        let result = validator.validate_series(&records);
        assert!(result.is_err());
    }
}
