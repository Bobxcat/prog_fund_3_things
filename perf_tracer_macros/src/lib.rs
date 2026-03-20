use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn trace_function(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input: ItemFn = parse_macro_input!(input as ItemFn);

    let sig = input.sig.clone();
    let func_name = &sig.ident;
    let func_name_str = func_name.to_string();
    let args = &sig.inputs;
    let arg_names = args.iter().map(|x| match x {
        syn::FnArg::Receiver(_) => quote! { self },
        syn::FnArg::Typed(pat_type) => {
            let name = &pat_type.pat;
            quote! { #name }
        }
    });

    quote! {
        #sig {
            #input
            ::perf_tracer::trace_op(
                #func_name_str,
                move || {
                    #func_name( #(#arg_names,)* )
                }
            )
        }
    }
    .into()
}
