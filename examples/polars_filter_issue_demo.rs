//! Demo to reproduce and test the Polars Int128 type coercion issue
//! when filtering with large i64 timestamp values in lazy frames

use polars::prelude::*;

fn main() {
    println!("=== Polars Filter Int128 Issue Demo ===\n");

    // Create test data with large timestamp values (milliseconds since epoch)
    let timestamps = vec![
        1704067200000i64,     // 2024-01-01 00:00:00 UTC
        1704153600000i64,     // 2024-01-02 00:00:00 UTC
        1704240000000i64,     // 2024-01-03 00:00:00 UTC
        1704326400000i64,     // 2024-01-04 00:00:00 UTC
        1704412800000i64,     // 2024-01-05 00:00:00 UTC
    ];

    let values = vec![100.0, 101.0, 102.0, 103.0, 104.0];

    // Create DataFrame
    let df = DataFrame::new(vec![
        Series::new("time".into(), &timestamps).into(),
        Series::new("value".into(), &values).into(),
    ])
    .unwrap();

    println!("Original DataFrame:");
    println!("{}\n", df);

    // Test 1: Direct DataFrame filtering (this should work)
    println!("Test 1: Direct DataFrame filtering");
    let start_time = 1704153600000i64; // 2024-01-02
    let end_time = 1704326400000i64;   // 2024-01-04

    let time_col = df.column("time").unwrap();
    let time_ca = time_col.i64().unwrap();
    let mask = time_ca.gt_eq(start_time) & time_ca.lt_eq(end_time);
    
    let filtered_df = df.filter(&mask).unwrap();
    println!("Filtered result (should have 3 rows):");
    println!("{}\n", filtered_df);

    // Test 2: LazyFrame filtering with lit() - this causes Int128 error
    println!("Test 2: LazyFrame filtering with lit()");
    let lazy_df = df.clone().lazy();
    
    // This is where the Int128 error occurs
    let filtered_lazy = lazy_df.filter(
        col("time")
            .gt_eq(lit(start_time))
            .and(col("time").lt_eq(lit(end_time))),
    );

    match filtered_lazy.collect() {
        Ok(result) => {
            println!("Success! Filtered result:");
            println!("{}\n", result);
        }
        Err(e) => {
            println!("Error (expected): {}", e);
            println!("This is the Int128 type coercion issue!\n");
        }
    }

    // Test 3: LazyFrame filtering with explicit cast
    println!("Test 3: LazyFrame filtering with explicit cast to Int64");
    let lazy_df2 = df.clone().lazy();
    
    let filtered_lazy2 = lazy_df2.filter(
        col("time")
            .gt_eq(lit(start_time).cast(DataType::Int64))
            .and(col("time").lt_eq(lit(end_time).cast(DataType::Int64))),
    );

    match filtered_lazy2.collect() {
        Ok(result) => {
            println!("Success! Filtered result:");
            println!("{}\n", result);
        }
        Err(e) => {
            println!("Error: {}\n", e);
        }
    }

    // Test 4: Using smaller timestamp values
    println!("Test 4: Using smaller timestamp values");
    let small_timestamps = vec![1000i64, 2000, 3000, 4000, 5000];
    let df_small = DataFrame::new(vec![
        Series::new("time".into(), &small_timestamps).into(),
        Series::new("value".into(), &values).into(),
    ])
    .unwrap();

    let lazy_df3 = df_small.lazy();
    let filtered_lazy3 = lazy_df3.filter(
        col("time")
            .gt_eq(lit(2000i64))
            .and(col("time").lt_eq(lit(4000i64))),
    );

    match filtered_lazy3.collect() {
        Ok(result) => {
            println!("Success with small values! Filtered result:");
            println!("{}\n", result);
        }
        Err(e) => {
            println!("Error: {}\n", e);
        }
    }

    // Test 5: Alternative approach - collect first, then filter
    println!("Test 5: Alternative approach - collect first, then filter");
    let alternative_filter = |df: DataFrame, start: i64, end: i64| -> PolarsResult<DataFrame> {
        let time_col = df.column("time")?;
        let time_ca = time_col.i64()?;
        let mask = time_ca.gt_eq(start) & time_ca.lt_eq(end);
        df.filter(&mask)
    };

    match alternative_filter(df.clone(), start_time, end_time) {
        Ok(result) => {
            println!("Success with alternative approach!");
            println!("{}\n", result);
        }
        Err(e) => {
            println!("Error: {}\n", e);
        }
    }

    // Test 6: Working solution - use smaller divisor to avoid Int128
    println!("Test 6: Working solution - scale down timestamps");
    
    // Scale down the timestamps to avoid Int128 inference
    let scaled_df = df.clone().lazy()
        .with_column((col("time") / lit(1000000)).alias("time_scaled"))
        .collect()
        .unwrap();
    
    let scaled_start = start_time / 1000000;
    let scaled_end = end_time / 1000000;
    
    let filtered_scaled = scaled_df.lazy().filter(
        col("time_scaled")
            .gt_eq(lit(scaled_start))
            .and(col("time_scaled").lt_eq(lit(scaled_end))),
    );
    
    match filtered_scaled.collect() {
        Ok(result) => {
            println!("Success with scaled timestamps!");
            println!("{}\n", result);
        }
        Err(e) => {
            println!("Error: {}\n", e);
        }
    }

    println!("=== Demo Complete ===");
}