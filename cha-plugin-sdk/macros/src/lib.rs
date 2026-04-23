use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, parse_macro_input};

/// The WIT content embedded at proc-macro compile time.
const WIT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/wit/plugin.wit"));

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
        #[allow(unused_imports)]
        use cha::plugin::types::{
            ClassInfo, FunctionInfo, ImportInfo, Location, OptionValue, Severity, SmellCategory,
        };

        /// Implement this trait in your plugin struct.
        /// `version`, `description`, `authors`, `smells` all have default impls.
        pub trait PluginImpl {
            fn name() -> String;
            fn analyze(input: AnalysisInput) -> Vec<Finding>;
            /// Smell names this plugin can produce. Default: empty.
            /// Declaring them lets the host filter by smell_name and show
            /// accurate docs in `cha plugin list`.
            fn smells() -> Vec<String> { vec![] }
        }

        struct __ChaPluginWrapper(std::marker::PhantomData<#ty>);

        impl Guest for __ChaPluginWrapper {
            fn name() -> String { <#ty as PluginImpl>::name() }
            fn version() -> String { env!("CARGO_PKG_VERSION").to_string() }
            fn description() -> String {
                let d = env!("CARGO_PKG_DESCRIPTION");
                if d.is_empty() { <#ty as PluginImpl>::name() } else { d.to_string() }
            }
            fn authors() -> Vec<String> {
                let a = env!("CARGO_PKG_AUTHORS");
                if a.is_empty() { vec![] } else { a.split(':').map(str::to_string).collect() }
            }
            fn smells() -> Vec<String> { <#ty as PluginImpl>::smells() }
            fn analyze(input: AnalysisInput) -> Vec<Finding> { <#ty as PluginImpl>::analyze(input) }
        }

        export!(__ChaPluginWrapper);
    }
    .into()
}
