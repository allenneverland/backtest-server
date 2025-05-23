use backtest_server::data_ingestion::validator::{
    create_default_ohlcv_chain, create_default_tick_chain, validate_data,
    OhlcvRecord, OhlcvValidator, TickRecord, TickValidator,
    TimeSeriesValidator, ValidationConfig, ValidationReport, Validator,
};
use chrono::{Duration, Utc};

#[tokio::test]
async fn test_ohlcv_validation_success() {
    let now = Utc::now();
    let records = vec![
        OhlcvRecord {
            timestamp: now,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 102.0,
            volume: 1000.0,
        },
        OhlcvRecord {
            timestamp: now + Duration::minutes(1),
            open: 102.0,
            high: 103.0,
            low: 101.0,
            close: 102.5,
            volume: 500.0,
        },
        OhlcvRecord {
            timestamp: now + Duration::minutes(2),
            open: 102.5,
            high: 104.0,
            low: 102.0,
            close: 103.0,
            volume: 800.0,
        },
    ];

    let chain = create_default_ohlcv_chain();
    let result = validate_data(&chain, &records, "OHLCV Test").await;
    
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.total_records, 3);
    assert_eq!(report.valid_records, 3);
    assert_eq!(report.invalid_records, 0);
}

#[tokio::test]
async fn test_ohlcv_validation_price_inconsistency() {
    let validator = OhlcvValidator::new();
    let record = OhlcvRecord {
        timestamp: Utc::now(),
        open: 100.0,
        high: 99.0,  // high < low，違反規則
        low: 105.0,
        close: 102.0,
        volume: 1000.0,
    };
    
    let result = validator.validate_record(&record);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ohlcv_validation_negative_volume() {
    let validator = OhlcvValidator::new();
    let record = OhlcvRecord {
        timestamp: Utc::now(),
        open: 100.0,
        high: 105.0,
        low: 99.0,
        close: 102.0,
        volume: -100.0,  // 負數成交量
    };
    
    let result = validator.validate_record(&record);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tick_validation_success() {
    let now = Utc::now();
    let records = vec![
        TickRecord {
            timestamp: now,
            price: 100.0,
            volume: 100.0,
            bid: Some(99.5),
            ask: Some(100.5),
            bid_volume: Some(50.0),
            ask_volume: Some(50.0),
        },
        TickRecord {
            timestamp: now + Duration::seconds(1),
            price: 100.1,
            volume: 150.0,
            bid: Some(99.6),
            ask: Some(100.6),
            bid_volume: Some(75.0),
            ask_volume: Some(75.0),
        },
    ];

    let chain = create_default_tick_chain();
    let result = validate_data(&chain, &records, "Tick Test").await;
    
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.total_records, 2);
    assert_eq!(report.valid_records, 2);
}

#[tokio::test]
async fn test_tick_validation_invalid_spread() {
    let validator = TickValidator::new().with_max_spread_percent(1.0);  // 1% 最大價差
    let record = TickRecord {
        timestamp: Utc::now(),
        price: 100.0,
        volume: 100.0,
        bid: Some(95.0),   // 5.26% 價差，超過限制
        ask: Some(100.0),
        bid_volume: None,
        ask_volume: None,
    };
    
    let result = validator.validate_record(&record);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tick_validation_price_outside_spread() {
    let validator = TickValidator::new();
    let record = TickRecord {
        timestamp: Utc::now(),
        price: 105.0,  // 價格在買賣價範圍外
        volume: 100.0,
        bid: Some(99.0),
        ask: Some(101.0),
        bid_volume: None,
        ask_volume: None,
    };
    
    let result = validator.validate_record(&record);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_time_series_validation_out_of_order() {
    let validator = TimeSeriesValidator::<OhlcvRecord>::new();
    let now = Utc::now();
    
    let records = vec![
        OhlcvRecord {
            timestamp: now,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 102.0,
            volume: 1000.0,
        },
        OhlcvRecord {
            timestamp: now + Duration::minutes(2),
            open: 102.0,
            high: 103.0,
            low: 101.0,
            close: 102.5,
            volume: 500.0,
        },
        OhlcvRecord {
            timestamp: now + Duration::minutes(1),  // 時間順序錯誤
            open: 101.0,
            high: 102.0,
            low: 100.0,
            close: 101.5,
            volume: 300.0,
        },
    ];
    
    let result = validator.validate_series(&records);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_time_series_validation_large_gap() {
    let validator = TimeSeriesValidator::<OhlcvRecord>::new()
        .with_max_gap(Duration::minutes(5));
    let now = Utc::now();
    
    let records = vec![
        OhlcvRecord {
            timestamp: now,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 102.0,
            volume: 1000.0,
        },
        OhlcvRecord {
            timestamp: now + Duration::minutes(10),  // 間隔過大
            open: 102.0,
            high: 103.0,
            low: 101.0,
            close: 102.5,
            volume: 500.0,
        },
    ];
    
    let result = validator.validate_series(&records);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_time_series_validation_duplicates() {
    let validator = TimeSeriesValidator::<OhlcvRecord>::new()
        .with_allow_duplicates(false);
    let now = Utc::now();
    
    let records = vec![
        OhlcvRecord {
            timestamp: now,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 102.0,
            volume: 1000.0,
        },
        OhlcvRecord {
            timestamp: now,  // 重複時間戳記
            open: 101.0,
            high: 106.0,
            low: 100.0,
            close: 103.0,
            volume: 1100.0,
        },
    ];
    
    let result = validator.validate_series(&records);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validation_config() {
    let config = ValidationConfig::new()
        .with_enabled(true)
        .with_fail_on_error(true)
        .with_max_errors(Some(100))
        .with_param("custom_param", serde_json::json!(42));
    
    assert!(config.enabled);
    assert!(config.fail_on_error);
    assert_eq!(config.max_errors, Some(100));
    
    let param: Option<i32> = config.get_param("custom_param");
    assert_eq!(param, Some(42));
}

#[tokio::test]
async fn test_validation_report_formatting() {
    use backtest_server::data_ingestion::validator::ReportFormatter;
    
    let mut report = ValidationReport::new("Test Validator");
    report.total_records = 1000;
    report.valid_records = 950;
    report.invalid_records = 50;
    report.add_statistic("avg_price", 100.5);
    report.add_statistic("max_volume", 10000);
    
    let finished_report = report.finish();
    
    let text = ReportFormatter::format_text(&finished_report);
    assert!(text.contains("Test Validator"));
    assert!(text.contains("1000"));
    assert!(text.contains("950"));
    assert!(text.contains("50"));
    
    let json = ReportFormatter::format_json(&finished_report).unwrap();
    // Pretty-printed JSON includes spaces after colons
    assert!(json.contains("\"validator_name\": \"Test Validator\""));
}