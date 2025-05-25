use proc_macro2::TokenStream;
use quote::quote;
use std::env;
use std::fs;
use std::path::Path;

// Note: These types are only used in build.rs for parsing the TOML file
// The actual types used in the crate are generated from this data

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

    let config: FrequencyConfig =
        toml::from_str(&toml_content).expect("Failed to parse config/frequencies.toml");

    // 生成頻率宏定義
    let frequencies_macro = generate_frequencies_macro(&config.frequency);

    // 寫入到輸出目錄
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("frequencies_generated.rs");

    fs::write(&dest_path, frequencies_macro.to_string())
        .expect("Failed to write generated frequency code");
}

fn generate_frequencies_macro(frequencies: &[FrequencyDef]) -> TokenStream {
    // 為每個頻率生成 token
    let frequency_entries: Vec<TokenStream> = frequencies
        .iter()
        .map(|freq| {
            let enum_name = syn::Ident::new(&freq.enum_name, proc_macro2::Span::call_site());
            let alias_suffix = &freq.alias_suffix;
            let is_ohlcv = freq.is_ohlcv;
            let seconds = freq.seconds;
            let milliseconds = freq.milliseconds;
            let polars_string = &freq.polars_string;
            let display_name = &freq.display_name;

            quote! {
                (#enum_name, #alias_suffix, #is_ohlcv, #seconds, #milliseconds, #polars_string, #display_name)
            }
        })
        .collect();

    // 生成完整的宏定義
    quote! {
        /// 主頻率定義宏 - 包含所有頻率的元數據
        /// 這是所有其他宏的數據源
        macro_rules! frequencies {
            ($call:ident) => {
                $call! {
                    #(#frequency_entries),*
                }
            };
        }
    }
}
