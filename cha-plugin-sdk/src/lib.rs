//! Cha Plugin SDK — toolkit for building Cha WASM analyzer plugins.
//!
//! # Usage
//!
//! `Cargo.toml`:
//! ```toml
//! [lib]
//! crate-type = ["cdylib"]
//!
//! [dependencies]
//! cha-plugin-sdk = { git = "https://github.com/W-Mai/Cha" }
//! wit-bindgen = "0.55"
//! ```
//!
//! `src/lib.rs`:
//! ```rust,ignore
//! cha_plugin_sdk::plugin!(MyPlugin);
//!
//! struct MyPlugin;
//! impl Guest for MyPlugin {
//!     fn name() -> String { "my-plugin".into() }
//!     fn analyze(input: AnalysisInput) -> Vec<Finding> { vec![] }
//! }
//! ```

pub use cha_plugin_sdk_macros::plugin;

#[cfg(feature = "test-utils")]
mod test_utils_impl;
#[cfg(feature = "test-utils")]
pub use test_utils_impl::test_utils;

/// Extract a string option by key from `analysis-input.options`.
#[macro_export]
macro_rules! option_str {
    ($options:expr, $key:expr) => {
        $options.iter().find_map(|(k, v)| match v {
            OptionValue::Str(s) if k == $key => Some(s.as_str()),
            _ => None,
        })
    };
}

/// Extract an integer option by key.
#[macro_export]
macro_rules! option_int {
    ($options:expr, $key:expr) => {
        $options.iter().find_map(|(k, v)| match v {
            OptionValue::Int(n) if k == $key => Some(*n),
            _ => None,
        })
    };
}

/// Extract a float option by key.
#[macro_export]
macro_rules! option_float {
    ($options:expr, $key:expr) => {
        $options.iter().find_map(|(k, v)| match v {
            OptionValue::Float(n) if k == $key => Some(*n),
            _ => None,
        })
    };
}

/// Extract a boolean option by key.
#[macro_export]
macro_rules! option_bool {
    ($options:expr, $key:expr) => {
        $options.iter().find_map(|(k, v)| match v {
            OptionValue::Boolean(b) if k == $key => Some(*b),
            _ => None,
        })
    };
}

/// Extract a string list option by key.
#[macro_export]
macro_rules! option_list_str {
    ($options:expr, $key:expr) => {
        $options.iter().find_map(|(k, v)| match v {
            OptionValue::ListStr(l) if k == $key => Some(l.as_slice()),
            _ => None,
        })
    };
}

/// Iterate all string options as `(key, value)` pairs.
#[macro_export]
macro_rules! str_options {
    ($options:expr) => {
        $options.iter().filter_map(|(k, v)| match v {
            OptionValue::Str(s) => Some((k.as_str(), s.as_str())),
            _ => None,
        })
    };
}
