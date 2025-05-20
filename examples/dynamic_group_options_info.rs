use polars::prelude::*;

fn main() {
    // This is just to trigger a compilation error that will show us 
    // the proper type structure of DynamicGroupOptions
    let _options = DynamicGroupOptions {
        label: "time", // We're using a string here, but error will tell us the right type
        index_column: "time", // Likewise, using string to get error with proper type
        every: Duration::parse("1d"),
        period: Duration::parse("1d"),
        offset: Duration::parse("0d"), // Using Duration instead of Option<Duration>
        include_boundaries: false,
        closed_window: ClosedWindow::Left,
        start_by: StartBy::WindowBound,
    };
    
    println!("DynamicGroupOptions structure information");
}