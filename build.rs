use std::env;
use std::fs;
use std::path::Path;

// Note: We need to duplicate these types in build.rs because we can't import from the crate being built
// These definitions must match those in src/domain_types/frequency.rs

#[derive(Debug, serde::Deserialize)]
struct FrequencyConfig {
    frequency: Vec<FrequencyDef>,
}

#[derive(Debug, serde::Deserialize)]
struct FrequencyDef {
    #[allow(dead_code)]
    name: String,
    enum_name: String,
    #[allow(dead_code)]
    struct_name: String,
    seconds: u64,
    milliseconds: u64,
    polars_string: String,
    display_name: String,
    alias_suffix: String,
    is_ohlcv: bool,
}

fn main() {
    println!("cargo:rerun-if-changed=config/frequencies.toml");
    
    // 讀取 frequencies.toml
    let toml_content = fs::read_to_string("config/frequencies.toml")
        .expect("Failed to read config/frequencies.toml");
    
    let config: FrequencyConfig = toml::from_str(&toml_content)
        .expect("Failed to parse config/frequencies.toml");
    
    // 生成頻率宏定義
    let frequencies_macro = generate_frequencies_macro(&config.frequency);
    
    // 寫入到輸出目錄
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("frequencies_generated.rs");
    
    fs::write(&dest_path, frequencies_macro)
        .expect("Failed to write generated frequency code");
}

fn generate_frequencies_macro(frequencies: &[FrequencyDef]) -> String {
    let mut output = String::new();
    
    // 生成主頻率定義宏
    output.push_str("/// 主頻率定義宏 - 包含所有頻率的元數據\n");
    output.push_str("/// 這是所有其他宏的數據源\n");
    output.push_str("macro_rules! frequencies {\n");
    output.push_str("    ($call:ident) => {\n");
    output.push_str("        $call! {\n");
    
    for (i, freq) in frequencies.iter().enumerate() {
        let comma = if i < frequencies.len() - 1 { "," } else { "" };
        output.push_str(&format!(
            "            ({}, {:?}, {}, {}, {}u64, {:?}, {:?}){}\n",
            freq.enum_name,
            freq.alias_suffix,
            freq.is_ohlcv,
            freq.seconds,
            freq.milliseconds,
            freq.polars_string,
            freq.display_name,
            comma
        ));
    }
    
    output.push_str("        }\n");
    output.push_str("    };\n");
    output.push_str("}\n");
    
    output
}