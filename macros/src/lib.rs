use proc_macro2::Ident;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitInt, Token, parse_macro_input};

struct RepeatInput {
    count: LitInt,
    _as: Option<(Token![as], Ident)>,
    _comma: Token![,],
    expr: Expr,
}

impl Parse for RepeatInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let count = input.parse()?;
        let _as: Option<(Token![as], Ident)> = match input.parse()? {
            Some(t) => Some((t, input.parse()?)),
            None => None,
        };
        let _comma = input.parse()?;
        let expr = input.parse()?;

        Ok(Self {
            count,
            _as,
            _comma,
            expr,
        })
    }
}

#[proc_macro]
pub fn repeat(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let RepeatInput {
        count, expr, _as, ..
    } = parse_macro_input!(input as RepeatInput);

    let count = match count.base10_parse::<usize>() {
        Ok(v) => v,
        Err(e) => return proc_macro::TokenStream::from(e.to_compile_error()),
    };

    let repeated = if let Some(_as) = _as {
        (0..count)
            .map(move |i| {
                let ident = _as.1.clone();
                quote! {
                    {
                        let #ident = #i;
                        #expr;
                    }
                }
            })
            .collect::<Vec<_>>()
    } else {
        (0..count)
            .map(|_| {
                quote! {
                    {
                        #expr;
                    }
                }
            })
            .collect::<Vec<_>>()
    };

    quote! { #(#repeated)* }.into()
}
