use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// DSL 標準庫定義
pub struct StandardLibrary {
    pub functions: HashMap<String, FunctionDefinition>,
    pub indicators: HashMap<String, IndicatorDefinition>,
    pub constants: HashMap<String, ConstantDefinition>,
    pub data_accessors: HashMap<String, DataAccessorDefinition>,
    pub types: HashMap<String, TypeDefinition>,
}

/// 函數定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub category: FunctionCategory,
    pub description: String,
    pub params: Vec<ParamDefinition>,
    pub return_type: DataType,
    pub examples: Vec<String>,
}

/// 函數分類
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionCategory {
    Math,
    TimeSeries,
    Statistics,
    Array,
    String,
    DateTime,
    Logical,
}

/// 參數定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDefinition {
    pub name: String,
    pub data_type: DataType,
    pub optional: bool,
    pub default_value: Option<Value>,
    pub description: String,
}

/// 技術指標定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorDefinition {
    pub name: String,
    pub category: IndicatorCategory,
    pub description: String,
    pub params: Vec<ParamDefinition>,
    pub outputs: Vec<OutputDefinition>,
    pub calculation_type: CalculationType,
}

/// 指標分類
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndicatorCategory {
    Trend,
    Momentum,
    Volatility,
    Volume,
    MarketStructure,
    Sentiment,
    Custom,
}

/// 輸出定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDefinition {
    pub name: String,
    pub data_type: DataType,
    pub description: String,
}

/// 計算類型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalculationType {
    Simple,
    Exponential,
    Weighted,
    Adaptive,
}

/// 資料存取器定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAccessorDefinition {
    pub name: String,
    pub description: String,
    pub return_type: DataType,
    pub context_required: Vec<String>,
}

/// 常量定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantDefinition {
    pub name: String,
    pub value: Value,
    pub data_type: DataType,
    pub description: String,
}

/// 類型定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub base_type: DataType,
    pub constraints: Option<TypeConstraints>,
    pub description: String,
}

/// 類型約束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeConstraints {
    pub min: Option<Value>,
    pub max: Option<Value>,
    pub pattern: Option<String>,
    pub enum_values: Option<Vec<Value>>,
}

/// 資料類型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Numeric,
    Integer,
    Float,
    String,
    Boolean,
    DateTime,
    Array(Box<DataType>),
    Object(HashMap<String, DataType>),
    TimeSeries(Box<DataType>),
    PriceSeries,
    VolumeSereis,
    Any,
}

/// 值類型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl StandardLibrary {
    pub fn new() -> Self {
        let mut lib = StandardLibrary {
            functions: HashMap::new(),
            indicators: HashMap::new(),
            constants: HashMap::new(),
            data_accessors: HashMap::new(),
            types: HashMap::new(),
        };

        lib.init_math_functions();
        lib.init_timeseries_functions();
        lib.init_statistics_functions();
        lib.init_indicators();
        lib.init_data_accessors();
        lib.init_constants();
        lib.init_types();

        lib
    }

    /// 初始化數學函數
    fn init_math_functions(&mut self) {
        // 基本數學函數
        self.add_function(FunctionDefinition {
            name: "abs".to_string(),
            category: FunctionCategory::Math,
            description: "絕對值".to_string(),
            params: vec![
                ParamDefinition {
                    name: "x".to_string(),
                    data_type: DataType::Numeric,
                    optional: false,
                    default_value: None,
                    description: "輸入值".to_string(),
                }
            ],
            return_type: DataType::Numeric,
            examples: vec!["abs(-5) // returns 5".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "max".to_string(),
            category: FunctionCategory::Math,
            description: "最大值".to_string(),
            params: vec![
                ParamDefinition {
                    name: "values".to_string(),
                    data_type: DataType::Array(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "數值數組".to_string(),
                }
            ],
            return_type: DataType::Numeric,
            examples: vec!["max([1, 5, 3]) // returns 5".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "min".to_string(),
            category: FunctionCategory::Math,
            description: "最小值".to_string(),
            params: vec![
                ParamDefinition {
                    name: "values".to_string(),
                    data_type: DataType::Array(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "數值數組".to_string(),
                }
            ],
            return_type: DataType::Numeric,
            examples: vec!["min([1, 5, 3]) // returns 1".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "sum".to_string(),
            category: FunctionCategory::Math,
            description: "總和".to_string(),
            params: vec![
                ParamDefinition {
                    name: "array".to_string(),
                    data_type: DataType::Array(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "數值數組".to_string(),
                }
            ],
            return_type: DataType::Numeric,
            examples: vec!["sum([1, 2, 3]) // returns 6".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "mean".to_string(),
            category: FunctionCategory::Math,
            description: "平均值".to_string(),
            params: vec![
                ParamDefinition {
                    name: "array".to_string(),
                    data_type: DataType::Array(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "數值數組".to_string(),
                }
            ],
            return_type: DataType::Float,
            examples: vec!["mean([1, 2, 3]) // returns 2.0".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "sqrt".to_string(),
            category: FunctionCategory::Math,
            description: "平方根".to_string(),
            params: vec![
                ParamDefinition {
                    name: "x".to_string(),
                    data_type: DataType::Numeric,
                    optional: false,
                    default_value: None,
                    description: "輸入值".to_string(),
                }
            ],
            return_type: DataType::Float,
            examples: vec!["sqrt(4) // returns 2.0".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "log".to_string(),
            category: FunctionCategory::Math,
            description: "自然對數".to_string(),
            params: vec![
                ParamDefinition {
                    name: "x".to_string(),
                    data_type: DataType::Numeric,
                    optional: false,
                    default_value: None,
                    description: "輸入值".to_string(),
                }
            ],
            return_type: DataType::Float,
            examples: vec!["log(2.718) // returns 1.0".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "exp".to_string(),
            category: FunctionCategory::Math,
            description: "指數函數".to_string(),
            params: vec![
                ParamDefinition {
                    name: "x".to_string(),
                    data_type: DataType::Numeric,
                    optional: false,
                    default_value: None,
                    description: "指數".to_string(),
                }
            ],
            return_type: DataType::Float,
            examples: vec!["exp(1) // returns 2.718".to_string()],
        });
    }

    /// 初始化時間序列函數
    fn init_timeseries_functions(&mut self) {
        self.add_function(FunctionDefinition {
            name: "shift".to_string(),
            category: FunctionCategory::TimeSeries,
            description: "位移n個週期".to_string(),
            params: vec![
                ParamDefinition {
                    name: "series".to_string(),
                    data_type: DataType::TimeSeries(Box::new(DataType::Any)),
                    optional: false,
                    default_value: None,
                    description: "時間序列".to_string(),
                },
                ParamDefinition {
                    name: "n".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "位移數量".to_string(),
                }
            ],
            return_type: DataType::TimeSeries(Box::new(DataType::Any)),
            examples: vec!["shift(close, -1) // 前一期收盤價".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "rolling".to_string(),
            category: FunctionCategory::TimeSeries,
            description: "滾動窗口".to_string(),
            params: vec![
                ParamDefinition {
                    name: "series".to_string(),
                    data_type: DataType::TimeSeries(Box::new(DataType::Any)),
                    optional: false,
                    default_value: None,
                    description: "時間序列".to_string(),
                },
                ParamDefinition {
                    name: "window".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "窗口大小".to_string(),
                }
            ],
            return_type: DataType::Array(Box::new(DataType::Any)),
            examples: vec!["rolling(close, 20) // 20期滾動窗口".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "resample".to_string(),
            category: FunctionCategory::TimeSeries,
            description: "重新採樣".to_string(),
            params: vec![
                ParamDefinition {
                    name: "series".to_string(),
                    data_type: DataType::TimeSeries(Box::new(DataType::Any)),
                    optional: false,
                    default_value: None,
                    description: "時間序列".to_string(),
                },
                ParamDefinition {
                    name: "freq".to_string(),
                    data_type: DataType::String,
                    optional: false,
                    default_value: None,
                    description: "頻率".to_string(),
                }
            ],
            return_type: DataType::TimeSeries(Box::new(DataType::Any)),
            examples: vec!["resample(close, '1h') // 按小時重採樣".to_string()],
        });
    }

    /// 初始化統計函數
    fn init_statistics_functions(&mut self) {
        self.add_function(FunctionDefinition {
            name: "corr".to_string(),
            category: FunctionCategory::Statistics,
            description: "相關係數".to_string(),
            params: vec![
                ParamDefinition {
                    name: "x".to_string(),
                    data_type: DataType::TimeSeries(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "序列1".to_string(),
                },
                ParamDefinition {
                    name: "y".to_string(),
                    data_type: DataType::TimeSeries(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "序列2".to_string(),
                },
                ParamDefinition {
                    name: "period".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "計算週期".to_string(),
                }
            ],
            return_type: DataType::Float,
            examples: vec!["corr(BTC, ETH, 20) // 20期相關係數".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "std".to_string(),
            category: FunctionCategory::Statistics,
            description: "標準差".to_string(),
            params: vec![
                ParamDefinition {
                    name: "array".to_string(),
                    data_type: DataType::Array(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "數值數組".to_string(),
                }
            ],
            return_type: DataType::Float,
            examples: vec!["std([1, 2, 3]) // returns 0.816".to_string()],
        });

        self.add_function(FunctionDefinition {
            name: "percentile".to_string(),
            category: FunctionCategory::Statistics,
            description: "百分位數".to_string(),
            params: vec![
                ParamDefinition {
                    name: "array".to_string(),
                    data_type: DataType::Array(Box::new(DataType::Numeric)),
                    optional: false,
                    default_value: None,
                    description: "數值數組".to_string(),
                },
                ParamDefinition {
                    name: "p".to_string(),
                    data_type: DataType::Float,
                    optional: false,
                    default_value: None,
                    description: "百分位（0-100）".to_string(),
                }
            ],
            return_type: DataType::Float,
            examples: vec!["percentile(prices, 90) // 90百分位".to_string()],
        });
    }

    /// 初始化技術指標
    fn init_indicators(&mut self) {
        // 移動平均線
        self.add_indicator(IndicatorDefinition {
            name: "ma".to_string(),
            category: IndicatorCategory::Trend,
            description: "移動平均線".to_string(),
            params: vec![
                ParamDefinition {
                    name: "series".to_string(),
                    data_type: DataType::PriceSeries,
                    optional: false,
                    default_value: None,
                    description: "價格序列".to_string(),
                },
                ParamDefinition {
                    name: "period".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "週期".to_string(),
                },
                ParamDefinition {
                    name: "type".to_string(),
                    data_type: DataType::String,
                    optional: true,
                    default_value: Some(Value::String("simple".to_string())),
                    description: "類型: simple, exponential, weighted".to_string(),
                }
            ],
            outputs: vec![
                OutputDefinition {
                    name: "ma".to_string(),
                    data_type: DataType::Float,
                    description: "移動平均值".to_string(),
                }
            ],
            calculation_type: CalculationType::Simple,
        });

        // RSI
        self.add_indicator(IndicatorDefinition {
            name: "rsi".to_string(),
            category: IndicatorCategory::Momentum,
            description: "相對強弱指標".to_string(),
            params: vec![
                ParamDefinition {
                    name: "series".to_string(),
                    data_type: DataType::PriceSeries,
                    optional: false,
                    default_value: None,
                    description: "價格序列".to_string(),
                },
                ParamDefinition {
                    name: "period".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "週期".to_string(),
                }
            ],
            outputs: vec![
                OutputDefinition {
                    name: "rsi".to_string(),
                    data_type: DataType::Float,
                    description: "RSI值 (0-100)".to_string(),
                }
            ],
            calculation_type: CalculationType::Exponential,
        });

        // MACD
        self.add_indicator(IndicatorDefinition {
            name: "macd".to_string(),
            category: IndicatorCategory::Trend,
            description: "移動平均收斂發散指標".to_string(),
            params: vec![
                ParamDefinition {
                    name: "series".to_string(),
                    data_type: DataType::PriceSeries,
                    optional: false,
                    default_value: None,
                    description: "價格序列".to_string(),
                },
                ParamDefinition {
                    name: "fast".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "快線週期".to_string(),
                },
                ParamDefinition {
                    name: "slow".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "慢線週期".to_string(),
                },
                ParamDefinition {
                    name: "signal".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "信號線週期".to_string(),
                }
            ],
            outputs: vec![
                OutputDefinition {
                    name: "macd_line".to_string(),
                    data_type: DataType::Float,
                    description: "MACD線".to_string(),
                },
                OutputDefinition {
                    name: "signal_line".to_string(),
                    data_type: DataType::Float,
                    description: "信號線".to_string(),
                },
                OutputDefinition {
                    name: "histogram".to_string(),
                    data_type: DataType::Float,
                    description: "柱狀圖".to_string(),
                }
            ],
            calculation_type: CalculationType::Exponential,
        });

        // 布林通道
        self.add_indicator(IndicatorDefinition {
            name: "bollinger_bands".to_string(),
            category: IndicatorCategory::Volatility,
            description: "布林通道".to_string(),
            params: vec![
                ParamDefinition {
                    name: "series".to_string(),
                    data_type: DataType::PriceSeries,
                    optional: false,
                    default_value: None,
                    description: "價格序列".to_string(),
                },
                ParamDefinition {
                    name: "period".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "週期".to_string(),
                },
                ParamDefinition {
                    name: "std_dev".to_string(),
                    data_type: DataType::Float,
                    optional: false,
                    default_value: None,
                    description: "標準差倍數".to_string(),
                }
            ],
            outputs: vec![
                OutputDefinition {
                    name: "upper".to_string(),
                    data_type: DataType::Float,
                    description: "上軌".to_string(),
                },
                OutputDefinition {
                    name: "middle".to_string(),
                    data_type: DataType::Float,
                    description: "中軌".to_string(),
                },
                OutputDefinition {
                    name: "lower".to_string(),
                    data_type: DataType::Float,
                    description: "下軌".to_string(),
                }
            ],
            calculation_type: CalculationType::Simple,
        });

        // ATR
        self.add_indicator(IndicatorDefinition {
            name: "atr".to_string(),
            category: IndicatorCategory::Volatility,
            description: "平均真實範圍".to_string(),
            params: vec![
                ParamDefinition {
                    name: "high".to_string(),
                    data_type: DataType::PriceSeries,
                    optional: false,
                    default_value: None,
                    description: "最高價序列".to_string(),
                },
                ParamDefinition {
                    name: "low".to_string(),
                    data_type: DataType::PriceSeries,
                    optional: false,
                    default_value: None,
                    description: "最低價序列".to_string(),
                },
                ParamDefinition {
                    name: "close".to_string(),
                    data_type: DataType::PriceSeries,
                    optional: false,
                    default_value: None,
                    description: "收盤價序列".to_string(),
                },
                ParamDefinition {
                    name: "period".to_string(),
                    data_type: DataType::Integer,
                    optional: false,
                    default_value: None,
                    description: "週期".to_string(),
                }
            ],
            outputs: vec![
                OutputDefinition {
                    name: "atr".to_string(),
                    data_type: DataType::Float,
                    description: "ATR值".to_string(),
                }
            ],
            calculation_type: CalculationType::Exponential,
        });
    }

    /// 初始化數據存取器
    fn init_data_accessors(&mut self) {
        // 價格數據
        self.add_data_accessor(DataAccessorDefinition {
            name: "open".to_string(),
            description: "開盤價".to_string(),
            return_type: DataType::Float,
            context_required: vec!["symbol".to_string()],
        });

        self.add_data_accessor(DataAccessorDefinition {
            name: "high".to_string(),
            description: "最高價".to_string(),
            return_type: DataType::Float,
            context_required: vec!["symbol".to_string()],
        });

        self.add_data_accessor(DataAccessorDefinition {
            name: "low".to_string(),
            description: "最低價".to_string(),
            return_type: DataType::Float,
            context_required: vec!["symbol".to_string()],
        });

        self.add_data_accessor(DataAccessorDefinition {
            name: "close".to_string(),
            description: "收盤價".to_string(),
            return_type: DataType::Float,
            context_required: vec!["symbol".to_string()],
        });

        self.add_data_accessor(DataAccessorDefinition {
            name: "volume".to_string(),
            description: "成交量".to_string(),
            return_type: DataType::Integer,
            context_required: vec!["symbol".to_string()],
        });

        self.add_data_accessor(DataAccessorDefinition {
            name: "vwap".to_string(),
            description: "成交量加權平均價".to_string(),
            return_type: DataType::Float,
            context_required: vec!["symbol".to_string()],
        });

        // 參考數據
        self.add_data_accessor(DataAccessorDefinition {
            name: "prev".to_string(),
            description: "前n個值".to_string(),
            return_type: DataType::Any,
            context_required: vec!["series".to_string(), "n".to_string()],
        });

        self.add_data_accessor(DataAccessorDefinition {
            name: "at".to_string(),
            description: "特定時間的值".to_string(),
            return_type: DataType::Any,
            context_required: vec!["series".to_string(), "datetime".to_string()],
        });

        self.add_data_accessor(DataAccessorDefinition {
            name: "between".to_string(),
            description: "時間區間的值".to_string(),
            return_type: DataType::Array(Box::new(DataType::Any)),
            context_required: vec!["series".to_string(), "start".to_string(), "end".to_string()],
        });
    }

    /// 初始化常量
    fn init_constants(&mut self) {
        // 時間框架
        self.add_constant(ConstantDefinition {
            name: "TIMEFRAME_1M".to_string(),
            value: Value::String("1m".to_string()),
            data_type: DataType::String,
            description: "1分鐘時間框架".to_string(),
        });

        self.add_constant(ConstantDefinition {
            name: "TIMEFRAME_5M".to_string(),
            value: Value::String("5m".to_string()),
            data_type: DataType::String,
            description: "5分鐘時間框架".to_string(),
        });

        self.add_constant(ConstantDefinition {
            name: "TIMEFRAME_1H".to_string(),
            value: Value::String("1h".to_string()),
            data_type: DataType::String,
            description: "1小時時間框架".to_string(),
        });

        self.add_constant(ConstantDefinition {
            name: "TIMEFRAME_1D".to_string(),
            value: Value::String("1d".to_string()),
            data_type: DataType::String,
            description: "1日時間框架".to_string(),
        });

        // 訂單類型
        self.add_constant(ConstantDefinition {
            name: "ORDER_MARKET".to_string(),
            value: Value::String("market".to_string()),
            data_type: DataType::String,
            description: "市價單".to_string(),
        });

        self.add_constant(ConstantDefinition {
            name: "ORDER_LIMIT".to_string(),
            value: Value::String("limit".to_string()),
            data_type: DataType::String,
            description: "限價單".to_string(),
        });

        self.add_constant(ConstantDefinition {
            name: "ORDER_STOP".to_string(),
            value: Value::String("stop".to_string()),
            data_type: DataType::String,
            description: "止損單".to_string(),
        });

        // 市場交易時段
        self.add_constant(ConstantDefinition {
            name: "SESSION_PREMARKET".to_string(),
            value: Value::String("04:00-09:30".to_string()),
            data_type: DataType::String,
            description: "盤前交易時段".to_string(),
        });

        self.add_constant(ConstantDefinition {
            name: "SESSION_REGULAR".to_string(),
            value: Value::String("09:30-16:00".to_string()),
            data_type: DataType::String,
            description: "常規交易時段".to_string(),
        });

        self.add_constant(ConstantDefinition {
            name: "SESSION_AFTERHOURS".to_string(),
            value: Value::String("16:00-20:00".to_string()),
            data_type: DataType::String,
            description: "盤後交易時段".to_string(),
        });
    }

    /// 初始化類型系統
    fn init_types(&mut self) {
        // 價格類型
        self.add_type(TypeDefinition {
            name: "Price".to_string(),
            base_type: DataType::Float,
            constraints: Some(TypeConstraints {
                min: Some(Value::Float(0.0)),
                max: None,
                pattern: None,
                enum_values: None,
            }),
            description: "價格類型，必須為正數".to_string(),
        });

        // 百分比類型
        self.add_type(TypeDefinition {
            name: "Percentage".to_string(),
            base_type: DataType::Float,
            constraints: Some(TypeConstraints {
                min: Some(Value::Float(-100.0)),
                max: Some(Value::Float(100.0)),
                pattern: None,
                enum_values: None,
            }),
            description: "百分比類型，範圍-100到100".to_string(),
        });

        // 交易代碼類型
        self.add_type(TypeDefinition {
            name: "Symbol".to_string(),
            base_type: DataType::String,
            constraints: Some(TypeConstraints {
                min: None,
                max: None,
                pattern: Some("^[A-Z0-9]{1,10}(\\.[A-Z]{2})?$".to_string()),
                enum_values: None,
            }),
            description: "交易代碼，如AAPL或2330.TW".to_string(),
        });

        // 時間框架類型
        self.add_type(TypeDefinition {
            name: "TimeFrame".to_string(),
            base_type: DataType::String,
            constraints: Some(TypeConstraints {
                min: None,
                max: None,
                pattern: None,
                enum_values: Some(vec![
                    Value::String("1m".to_string()),
                    Value::String("5m".to_string()),
                    Value::String("15m".to_string()),
                    Value::String("1h".to_string()),
                    Value::String("4h".to_string()),
                    Value::String("1d".to_string()),
                ]),
            }),
            description: "時間框架枚舉".to_string(),
        });
    }

    // 輔助方法
    fn add_function(&mut self, function: FunctionDefinition) {
        self.functions.insert(function.name.clone(), function);
    }

    fn add_indicator(&mut self, indicator: IndicatorDefinition) {
        self.indicators.insert(indicator.name.clone(), indicator);
    }

    fn add_data_accessor(&mut self, accessor: DataAccessorDefinition) {
        self.data_accessors.insert(accessor.name.clone(), accessor);
    }

    fn add_constant(&mut self, constant: ConstantDefinition) {
        self.constants.insert(constant.name.clone(), constant);
    }

    fn add_type(&mut self, type_def: TypeDefinition) {
        self.types.insert(type_def.name.clone(), type_def);
    }

    /// 獲取函數定義
    pub fn get_function(&self, name: &str) -> Option<&FunctionDefinition> {
        self.functions.get(name)
    }

    /// 獲取指標定義
    pub fn get_indicator(&self, name: &str) -> Option<&IndicatorDefinition> {
        self.indicators.get(name)
    }

    /// 獲取常量定義
    pub fn get_constant(&self, name: &str) -> Option<&ConstantDefinition> {
        self.constants.get(name)
    }

    /// 獲取數據存取器定義
    pub fn get_data_accessor(&self, name: &str) -> Option<&DataAccessorDefinition> {
        self.data_accessors.get(name)
    }

    /// 獲取類型定義
    pub fn get_type(&self, name: &str) -> Option<&TypeDefinition> {
        self.types.get(name)
    }

    /// 驗證函數調用
    pub fn validate_function_call(&self, name: &str, args: &[Value]) -> Result<(), String> {
        match self.get_function(name) {
            Some(func) => {
                // 檢查參數數量
                let required_params = func.params.iter().filter(|p| !p.optional).count();
                if args.len() < required_params {
                    return Err(format!(
                        "Function '{}' requires at least {} arguments, got {}",
                        name, required_params, args.len()
                    ));
                }
                Ok(())
            }
            None => Err(format!("Unknown function: {}", name)),
        }
    }

    /// 列出所有可用的函數
    pub fn list_functions(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }

    /// 列出所有可用的指標
    pub fn list_indicators(&self) -> Vec<&str> {
        self.indicators.keys().map(|s| s.as_str()).collect()
    }

    /// 導出為 JSON 格式（用於文檔生成）
    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

// 默認實現
impl Default for StandardLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_initialization() {
        let stdlib = StandardLibrary::new();
        
        // 測試函數是否被正確初始化
        assert!(stdlib.get_function("abs").is_some());
        assert!(stdlib.get_function("max").is_some());
        assert!(stdlib.get_function("corr").is_some());
        
        // 測試指標是否被正確初始化
        assert!(stdlib.get_indicator("ma").is_some());
        assert!(stdlib.get_indicator("rsi").is_some());
        assert!(stdlib.get_indicator("macd").is_some());
        
        // 測試常量是否被正確初始化
        assert!(stdlib.get_constant("TIMEFRAME_1D").is_some());
        assert!(stdlib.get_constant("ORDER_MARKET").is_some());
    }

    #[test]
    fn test_function_validation() {
        let stdlib = StandardLibrary::new();
        
        // 測試有效的函數調用
        let args = vec![Value::Float(-5.0)];
        assert!(stdlib.validate_function_call("abs", &args).is_ok());
        
        // 測試無效的函數名
        assert!(stdlib.validate_function_call("unknown_func", &args).is_err());
        
        // 測試參數數量不足
        let empty_args: Vec<Value> = vec![];
        assert!(stdlib.validate_function_call("abs", &empty_args).is_err());
    }

    #[test]
    fn test_indicator_definition() {
        let stdlib = StandardLibrary::new();
        
        let ma = stdlib.get_indicator("ma").unwrap();
        assert_eq!(ma.name, "ma");
        assert_eq!(ma.params.len(), 3);
        assert_eq!(ma.outputs.len(), 1);
        
        let macd = stdlib.get_indicator("macd").unwrap();
        assert_eq!(macd.outputs.len(), 3); // macd_line, signal_line, histogram
    }

    #[test]
    fn test_type_constraints() {
        let stdlib = StandardLibrary::new();
        
        let price_type = stdlib.get_type("Price").unwrap();
        assert!(price_type.constraints.is_some());
        
        let constraints = price_type.constraints.as_ref().unwrap();
        assert_eq!(constraints.min, Some(Value::Float(0.0)));
        
        let timeframe_type = stdlib.get_type("TimeFrame").unwrap();
        let constraints = timeframe_type.constraints.as_ref().unwrap();
        assert!(constraints.enum_values.is_some());
    }
}