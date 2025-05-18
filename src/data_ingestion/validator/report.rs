use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use super::error::DataValidationError;
use crate::domain_types::data_point::OHLCVPoint;
use crate::domain_types::TimeSeries;
use std::collections::HashSet;

/// 數據驗證報告
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataValidationReport {
    pub symbol: String,
    pub data_type: String, // e.g., "OHLCV", "Tick"
    pub validation_time: DateTime<Utc>,
    pub total_records_processed: usize, // Total records before filtering
    pub valid_records: usize, // Records remaining after validation and cleaning
    pub invalid_records_count: usize, // Number of records deemed invalid
    pub validation_issues: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationWarning>,
    pub statistics: Option<DataStatistics>, // Statistics might not be available for all data types or if data is empty
}

/// 驗證問題
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationIssue {
    pub error_type: String,
    pub message: String,
    pub record_index: Option<usize>, // Index in the original raw data, if applicable
    pub field: Option<String>,
    pub value: Option<String>, // The problematic value
    pub timestamp: Option<DateTime<Utc>>,
}

/// 驗證警告（不會導致數據被過濾，但需要注意）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationWarning {
    pub warning_type: String,
    pub message: String,
    pub record_index: Option<usize>,
    pub field: Option<String>,
    pub value: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
}

/// 數據統計信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataStatistics {
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub avg_price: Option<f64>,
    pub min_volume: Option<f64>,
    pub max_volume: Option<f64>,
    pub avg_volume: Option<f64>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub time_span_days: Option<f64>,
    pub price_volatility: Option<f64>,
    pub missing_data_points_estimated: usize, // Estimated based on expected frequency vs. actual
    pub gaps_count: usize,
    pub largest_gap_duration_secs: Option<i64>,
    pub average_gap_duration_secs: Option<f64>,
}

impl DataValidationReport {
    pub fn new(symbol: String, data_type: String, total_records_processed: usize) -> Self {
        Self {
            symbol,
            data_type,
            validation_time: Utc::now(),
            total_records_processed,
            valid_records: 0, // Will be updated after processing
            invalid_records_count: 0,
            validation_issues: Vec::new(),
            warnings: Vec::new(),
            statistics: None, 
        }
    }

    pub fn add_issue(&mut self, error: DataValidationError, record_index: Option<usize>, timestamp: Option<DateTime<Utc>>) {
        let (error_type, message, field, value) = match error {
            DataValidationError::RangeError { field, value, message, .. } => 
                ("RangeError".to_string(), message, Some(field), Some(value)),
            DataValidationError::FormatError { field, message, .. } => 
                ("FormatError".to_string(), message, Some(field), None),
            DataValidationError::LogicError { message, .. } => 
                ("LogicError".to_string(), message, None, None),
            DataValidationError::MissingData { field, .. } => 
                ("MissingData".to_string(), format!("缺失欄位 {}", field), Some(field), None),
            DataValidationError::ConsistencyError { message, .. } => 
                ("ConsistencyError".to_string(), message, None, None),
            DataValidationError::TimeSeriesError { message, .. } => 
                ("TimeSeriesError".to_string(), message, None, None),
            DataValidationError::DuplicateDataError { message, timestamp: ts, .. } => 
                ("DuplicateDataError".to_string(), message, None, ts.map(|t| t.to_rfc3339())),
            DataValidationError::SystemError { message, .. } => 
                ("SystemError".to_string(), message, None, None),
        };
        
        self.validation_issues.push(ValidationIssue {
            error_type,
            message,
            record_index,
            field,
            value,
            timestamp,
        });
        self.invalid_records_count += 1;
    }
    
    pub fn add_warning(&mut self, warning_type: &str, message: &str, record_index: Option<usize>, field: Option<String>, value: Option<String>, timestamp: Option<DateTime<Utc>>) {
        self.warnings.push(ValidationWarning {
            warning_type: warning_type.to_string(),
            message: message.to_string(),
            record_index,
            field,
            value,
            timestamp,
        });
    }

    pub fn set_valid_records_count(&mut self, count: usize) {
        self.valid_records = count;
    }

    pub fn calculate_and_set_ohlcv_stats(&mut self, time_series: &TimeSeries<OHLCVPoint>) {
        if time_series.is_empty() {
            self.statistics = Some(DataStatistics {
                min_price: None,
                max_price: None,
                avg_price: None,
                min_volume: None,
                max_volume: None,
                avg_volume: None,
                start_date: None,
                end_date: None,
                time_span_days: None,
                price_volatility: None,
                missing_data_points_estimated: 0,
                gaps_count: 0,
                largest_gap_duration_secs: None,
                average_gap_duration_secs: None,
            });
            return;
        }
        
        let mut min_p = f64::MAX;
        let mut max_p = f64::MIN;
        let mut sum_price = 0.0;
        let mut min_v = f64::MAX;
        let mut max_v = f64::MIN;
        let mut sum_volume = 0.0;
        
        for point in &time_series.data {
            min_p = min_p.min(point.low);
            max_p = max_p.max(point.high);
            sum_price += point.close;
            min_v = min_v.min(point.volume);
            max_v = max_v.max(point.volume);
            sum_volume += point.volume;
        }
        
        let count = time_series.data.len() as f64;
        let avg_p = if count > 0.0 { Some(sum_price / count) } else { None };
        let avg_v = if count > 0.0 { Some(sum_volume / count) } else { None };
        
        let time_span_days = match (time_series.start_time, time_series.end_time) {
            (Some(start), Some(end)) if end > start => {
                Some(end.signed_duration_since(start).num_milliseconds() as f64 / (24.0 * 60.0 * 60.0 * 1000.0))
            }
            _ => None,
        };
        
        let price_volatility = if let Some(avg_price_val) = avg_p {
            if count > 0.0 {
                let variance = time_series.data.iter()
                    .map(|point| {
                        let diff = point.close - avg_price_val;
                        diff * diff
                    })
                    .sum::<f64>() / count;
                Some(variance.sqrt())
            } else { None }
        } else { None };
        
        // Basic gap calculation (can be refined)
        let mut gaps_count = 0;
        let mut total_gap_duration_secs: i64 = 0;
        let mut largest_gap_duration_secs: i64 = 0;
        
        if time_series.data.len() > 1 {
            for i in 0..(time_series.data.len() - 1) {
                let duration_since_last = time_series.data[i+1].timestamp.signed_duration_since(time_series.data[i].timestamp);
                // Define what constitutes a significant gap (e.g., more than 2x typical interval for the data's frequency)
                // This is a placeholder; actual expected interval would depend on time_series.frequency if available
                let expected_interval_approx_secs = match time_series.frequency {
                    Some(freq) => {
                        let duration = freq.to_duration();
                        duration.as_secs() as i64
                    },
                    None => 60, // Default to 1 minute if no frequency
                };

                if duration_since_last.num_seconds() > expected_interval_approx_secs * 2 { // Example: gap is > 2x expected interval
                    gaps_count += 1;
                    let gap_secs = duration_since_last.num_seconds();
                    total_gap_duration_secs += gap_secs;
                    if gap_secs > largest_gap_duration_secs {
                        largest_gap_duration_secs = gap_secs;
                    }
                }
            }
        }

        let avg_gap_duration_secs = if gaps_count > 0 {
            Some(total_gap_duration_secs as f64 / gaps_count as f64)
        } else { None };
        

        self.statistics = Some(DataStatistics {
            min_price: Some(min_p),
            max_price: Some(max_p),
            avg_price: avg_p,
            min_volume: Some(min_v),
            max_volume: Some(max_v),
            avg_volume: avg_v,
            start_date: time_series.start_time,
            end_date: time_series.end_time,
            time_span_days,
            price_volatility,
            missing_data_points_estimated: 0, // Placeholder, needs better estimation logic
            gaps_count,
            largest_gap_duration_secs: if largest_gap_duration_secs > 0 { Some(largest_gap_duration_secs) } else { None },
            average_gap_duration_secs: avg_gap_duration_secs,
        });
    }
    
    // TODO: Implement calculate_and_set_tick_stats for TickPoint data

    pub fn find_issues_by_timestamp(&self, timestamp: DateTime<Utc>) -> Vec<&ValidationIssue> {
        self.validation_issues.iter()
            .filter(|issue| issue.timestamp.map_or(false, |ts| ts == timestamp))
            .collect()
    }
    
    pub fn find_issues_by_type(&self, error_type: &str) -> Vec<&ValidationIssue> {
        self.validation_issues.iter()
            .filter(|issue| issue.error_type == error_type)
            .collect()
    }
    
    pub fn get_problem_timestamps(&self) -> Vec<DateTime<Utc>> {
        self.validation_issues.iter()
            .filter_map(|issue| issue.timestamp)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn generate_summary(&self) -> String {
        let mut summary = format!(
            "Validation Report for Symbol: {}\nData Type: {}\nValidation Time: {}\nTotal Records Processed: {}\nValid Records: {}\nInvalid Records: {}\n",
            self.symbol,
            self.data_type,
            self.validation_time.to_rfc3339(),
            self.total_records_processed,
            self.valid_records,
            self.invalid_records_count
        );

        if let Some(stats) = &self.statistics {
            summary.push_str(&format!("\n--- Statistics ---\n"));
            stats.start_date.map(|sd| summary.push_str(&format!("Start Date: {}\n", sd.to_rfc3339())));
            stats.end_date.map(|ed| summary.push_str(&format!("End Date: {}\n", ed.to_rfc3339())));
            stats.time_span_days.map(|tsd| summary.push_str(&format!("Time Span (days): {:.2}\n", tsd)));
            stats.min_price.map(|mp| summary.push_str(&format!("Min Price: {:.2}\n", mp)));
            stats.max_price.map(|mp| summary.push_str(&format!("Max Price: {:.2}\n", mp)));
            stats.avg_price.map(|ap| summary.push_str(&format!("Avg Price: {:.2}\n", ap)));
            stats.min_volume.map(|mv| summary.push_str(&format!("Min Volume: {:.2}\n", mv)));
            stats.max_volume.map(|mv| summary.push_str(&format!("Max Volume: {:.2}\n", mv)));
            stats.avg_volume.map(|av| summary.push_str(&format!("Avg Volume: {:.2}\n", av)));
            stats.price_volatility.map(|pv| summary.push_str(&format!("Price Volatility (std dev): {:.4}\n", pv)));
            summary.push_str(&format!("Gaps Count: {}\n", stats.gaps_count));
            stats.largest_gap_duration_secs.map(|lgd| summary.push_str(&format!("Largest Gap (s): {}\n", lgd)));
            stats.average_gap_duration_secs.map(|agd| summary.push_str(&format!("Average Gap (s): {:.2}\n", agd)));
        }

        if !self.validation_issues.is_empty() {
            summary.push_str(&format!("\n--- Validation Issues ({}) ---\n", self.validation_issues.len()));
            for issue in self.validation_issues.iter().take(10) { // Show first 10 issues
                summary.push_str(&format!("- Type: {}, Msg: {}, Idx: {:?}, Field: {:?}, Val: {:?}, TS: {:?}\n", 
                    issue.error_type, issue.message, issue.record_index, issue.field, issue.value, issue.timestamp.map(|t| t.to_rfc3339())
                ));
            }
            if self.validation_issues.len() > 10 {
                summary.push_str(&format!("... and {} more issues.\n", self.validation_issues.len() - 10));
            }
        }

        if !self.warnings.is_empty() {
            summary.push_str(&format!("\n--- Warnings ({}) ---\n", self.warnings.len()));
            for warning in self.warnings.iter().take(10) { // Show first 10 warnings
                summary.push_str(&format!("- Type: {}, Msg: {}, Idx: {:?}, Field: {:?}, Val: {:?}, TS: {:?}\n", 
                    warning.warning_type, warning.message, warning.record_index, warning.field, warning.value, warning.timestamp.map(|t| t.to_rfc3339())
                ));
            }
             if self.warnings.len() > 10 {
                summary.push_str(&format!("... and {} more warnings.\n", self.warnings.len() - 10));
            }
        }
        summary
    }
} 