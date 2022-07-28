use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Meta};

/// Print gas usage of a contract method, optionally gated by a cfg attribute.
///
/// ```ignore
/// #[debug_print_gas]
/// impl Contract {
///     #[debug_print_gas]
///     pub fn new_order() {
///         // gas usage will be logged always
///     }
///
///     #[debug_print_gas(test)]
///     pub fn new_order() {
///         // gas usage will be logged in the test profile
///     }
///
///     #[debug_print_gas(feature = "measure_performance")]
///     pub fn new_order() {
///         // gas usage will be logged when the "measure_performance" feature is enabled
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn measure_gas(_cfg_gate: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_attrs = input_fn.attrs;
    let fn_vis = input_fn.vis;
    let fn_sig = input_fn.sig;
    let fn_block = input_fn.block;
    let fn_name = fn_sig.ident.to_string();

    let fn_block_with_gas_measurement = quote! {
        let before = near_sdk::env::used_gas().0;

        let ret = { #fn_block };

        let after = near_sdk::env::used_gas().0;

        near_sdk::env::log_str(&format!("GAS USAGE({}): {}", #fn_name, after - before));

        ret
    };

    if _cfg_gate.is_empty() {
        // always measure
        return proc_macro::TokenStream::from(quote! {
            #(#fn_attrs)*
            #fn_vis #fn_sig {
                #fn_block_with_gas_measurement
            }
        });
    }
    // conditionally measure
    let cfg_meta = parse_macro_input!(_cfg_gate as Meta);
    proc_macro::TokenStream::from(quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            if cfg!(#cfg_meta) {
                #fn_block_with_gas_measurement
            } else {
                #fn_block
            }
        }
    })
}
