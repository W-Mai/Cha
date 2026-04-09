use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, parse_macro_input};

/// The WIT content embedded at proc-macro compile time.
const WIT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../wit/plugin.wit"));

/// Set up bindings and export a plugin implementation.
///
/// Expands to `wit_bindgen::generate!` with the embedded WIT and `export!`.
/// No local WIT file needed in the plugin project.
///
/// # Example
/// ```rust,ignore
/// cha_plugin_sdk::plugin!(MyPlugin);
///
/// struct MyPlugin;
/// impl Guest for MyPlugin {
///     fn name() -> String { "my-plugin".into() }
///     fn analyze(input: AnalysisInput) -> Vec<Finding> { vec![] }
/// }
/// ```
#[proc_macro]
pub fn plugin(input: TokenStream) -> TokenStream {
    let ty = parse_macro_input!(input as Ident);
    let wit = WIT;

    quote! {
        wit_bindgen::generate!({
            inline: #wit,
            world: "analyzer",
        });
        // Bring remaining types into scope (AnalysisInput/Finding are already at root).
        #[allow(unused_imports)]
        use cha::plugin::types::{
            ClassInfo, FunctionInfo, ImportInfo, Location, OptionValue, Severity, SmellCategory,
        };
        export!(#ty);
    }
    .into()
}
