use super::error::{ValidationError, ValidationErrors};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 驗證報告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// 驗證器名稱
    pub validator_name: String,
    /// 開始時間
    pub start_time: DateTime<Utc>,
    /// 結束時間
    pub end_time: DateTime<Utc>,
    /// 總記錄數
    pub total_records: usize,
    /// 有效記錄數
    pub valid_records: usize,
    /// 無效記錄數
    pub invalid_records: usize,
    /// 錯誤摘要
    pub error_summary: ErrorSummary,
    /// 詳細錯誤（可選）
    pub detailed_errors: Option<Vec<DetailedError>>,
    /// 統計資訊
    pub statistics: HashMap<String, serde_json::Value>,
}

impl ValidationReport {
    /// 創建新的驗證報告
    pub fn new(validator_name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            validator_name: validator_name.into(),
            start_time: now,
            end_time: now,
            total_records: 0,
            valid_records: 0,
            invalid_records: 0,
            error_summary: ErrorSummary::default(),
            detailed_errors: None,
            statistics: HashMap::new(),
        }
    }

    /// 完成報告
    pub fn finish(mut self) -> Self {
        self.end_time = Utc::now();
        self
    }

    /// 添加成功記錄
    pub fn add_success(&mut self) {
        self.total_records += 1;
        self.valid_records += 1;
    }

    /// 添加錯誤
    pub fn add_error(&mut self, line: usize, error: ValidationError) {
        self.total_records += 1;
        self.invalid_records += 1;
        self.error_summary.add_error(&error);

        if let Some(ref mut errors) = self.detailed_errors {
            errors.push(DetailedError {
                line,
                error_type: error_type_name(&error),
                message: error.to_string(),
            });
        }
    }

    /// 從錯誤集合創建報告
    pub fn from_errors(
        validator_name: impl Into<String>,
        total_records: usize,
        errors: &ValidationErrors,
    ) -> Self {
        let mut report = Self::new(validator_name);
        report.total_records = total_records;
        report.valid_records = total_records.saturating_sub(errors.error_count());
        report.invalid_records = errors.error_count();

        for (line, error) in errors.iter() {
            report.error_summary.add_error(error);

            if report.detailed_errors.is_none() {
                report.detailed_errors = Some(Vec::new());
            }

            if let Some(ref mut detailed) = report.detailed_errors {
                detailed.push(DetailedError {
                    line: *line,
                    error_type: error_type_name(error),
                    message: error.to_string(),
                });
            }
        }

        report.finish()
    }

    /// 啟用詳細錯誤記錄
    pub fn with_detailed_errors(mut self) -> Self {
        self.detailed_errors = Some(Vec::new());
        self
    }

    /// 添加統計資訊
    pub fn add_statistic(&mut self, key: impl Into<String>, value: impl Serialize) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.statistics.insert(key.into(), json_value);
        }
    }

    /// 獲取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_records == 0 {
            0.0
        } else {
            self.valid_records as f64 / self.total_records as f64
        }
    }

    /// 獲取處理時間（秒）
    pub fn processing_time(&self) -> f64 {
        (self.end_time - self.start_time).num_milliseconds() as f64 / 1000.0
    }

    /// 合併多個報告
    pub fn merge(reports: Vec<ValidationReport>) -> Option<ValidationReport> {
        if reports.is_empty() {
            return None;
        }

        let mut merged = ValidationReport::new("MergedReport");
        merged.start_time = reports.iter().map(|r| r.start_time).min().unwrap();
        merged.end_time = reports.iter().map(|r| r.end_time).max().unwrap();

        for report in reports {
            merged.total_records += report.total_records;
            merged.valid_records += report.valid_records;
            merged.invalid_records += report.invalid_records;
            merged.error_summary.merge(report.error_summary);

            if let Some(errors) = report.detailed_errors {
                if merged.detailed_errors.is_none() {
                    merged.detailed_errors = Some(Vec::new());
                }
                if let Some(ref mut merged_errors) = merged.detailed_errors {
                    merged_errors.extend(errors);
                }
            }
        }

        Some(merged)
    }
}

/// 錯誤摘要
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorSummary {
    /// 各類型錯誤計數
    pub error_counts: HashMap<String, usize>,
    /// 最常見的錯誤
    pub top_errors: Vec<(String, usize)>,
}

impl ErrorSummary {
    /// 添加錯誤
    pub fn add_error(&mut self, error: &ValidationError) {
        let error_type = error_type_name(error);
        *self.error_counts.entry(error_type).or_insert(0) += 1;
        self.update_top_errors();
    }

    /// 更新最常見錯誤
    fn update_top_errors(&mut self) {
        let mut counts: Vec<(String, usize)> = self
            .error_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        self.top_errors = counts.into_iter().take(5).collect();
    }

    /// 合併另一個錯誤摘要
    pub fn merge(&mut self, other: ErrorSummary) {
        for (error_type, count) in other.error_counts {
            *self.error_counts.entry(error_type).or_insert(0) += count;
        }
        self.update_top_errors();
    }
}

/// 詳細錯誤資訊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedError {
    /// 行號
    pub line: usize,
    /// 錯誤類型
    pub error_type: String,
    /// 錯誤訊息
    pub message: String,
}

/// 獲取錯誤類型名稱
fn error_type_name(error: &ValidationError) -> String {
    match error {
        ValidationError::OutOfRange { .. } => "OutOfRange".to_string(),
        ValidationError::InconsistentValue { .. } => "InconsistentValue".to_string(),
        ValidationError::MissingField { .. } => "MissingField".to_string(),
        ValidationError::InvalidTimestamp { .. } => "InvalidTimestamp".to_string(),
        ValidationError::DuplicateEntry { .. } => "DuplicateEntry".to_string(),
        ValidationError::OutOfOrder { .. } => "OutOfOrder".to_string(),
        ValidationError::LargeGap { .. } => "LargeGap".to_string(),
        ValidationError::InvalidValue { .. } => "InvalidValue".to_string(),
        ValidationError::TypeMismatch { .. } => "TypeMismatch".to_string(),
        ValidationError::BatchValidationFailed { .. } => "BatchValidationFailed".to_string(),
        ValidationError::CustomRuleFailed { .. } => "CustomRuleFailed".to_string(),
    }
}

/// 報告格式化器
pub struct ReportFormatter;

impl ReportFormatter {
    /// 格式化為人類可讀的文字
    pub fn format_text(report: &ValidationReport) -> String {
        let mut output = String::new();

        output.push_str(&format!("=== 驗證報告: {} ===\n", report.validator_name));
        output.push_str(&format!(
            "開始時間: {}\n",
            report.start_time.format("%Y-%m-%d %H:%M:%S")
        ));
        output.push_str(&format!(
            "結束時間: {}\n",
            report.end_time.format("%Y-%m-%d %H:%M:%S")
        ));
        output.push_str(&format!("處理時間: {:.2} 秒\n", report.processing_time()));
        output.push_str("\n");

        output.push_str("統計摘要:\n");
        output.push_str(&format!("  總記錄數: {}\n", report.total_records));
        output.push_str(&format!(
            "  有效記錄: {} ({:.2}%)\n",
            report.valid_records,
            report.success_rate() * 100.0
        ));
        output.push_str(&format!(
            "  無效記錄: {} ({:.2}%)\n",
            report.invalid_records,
            (1.0 - report.success_rate()) * 100.0
        ));
        output.push_str("\n");

        if !report.error_summary.top_errors.is_empty() {
            output.push_str("最常見的錯誤:\n");
            for (error_type, count) in &report.error_summary.top_errors {
                output.push_str(&format!("  {}: {} 次\n", error_type, count));
            }
            output.push_str("\n");
        }

        if !report.statistics.is_empty() {
            output.push_str("其他統計:\n");
            for (key, value) in &report.statistics {
                output.push_str(&format!("  {}: {}\n", key, value));
            }
        }

        output
    }

    /// 格式化為JSON
    pub fn format_json(report: &ValidationReport) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(report)
    }
}
