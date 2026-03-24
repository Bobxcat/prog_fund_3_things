use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn trace_function(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input: ItemFn = parse_macro_input!(input as ItemFn);
    let func_name = input.sig.ident.to_string();
    let trace_name = syn::parse::<LitStr>(attr).map_or_else(
        |_| func_name.clone(),
        |x| {
            let s = x.value();
            let s = s.replace("$f", &func_name);
            s
        },
    );

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = input;

    quote! {
        #(#attrs)* #vis #sig {
            ::perf_tracer::trace_op(
                #trace_name,
                move || {
                    #block
                }
            )
        }
    }
    .into()
}
