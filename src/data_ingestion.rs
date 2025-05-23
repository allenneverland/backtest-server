//! 資料攝取模組

pub mod processor;
pub mod validator;

pub use processor::{CsvReader, CsvParser, CsvError, CsvResult};