#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::{HashSet, HashMap};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, token, braced, Ident, Result, Token, Type};
use std::iter::repeat;

struct GenOpSingle {
    reg_type: Type,
    brace_token: token::Brace,
    rules: Punctuated<OpRule, Token![;]>,
}

impl Parse for GenOpSingle {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            reg_type: input.parse()?,
            brace_token: braced!(content in input),
            rules: content.parse_terminated(OpRule::parse)?,
        })
    }
}

struct GenOps {
    types: Punctuated<GenOpSingle, Token![,]>,
}

impl Parse for GenOps {
    fn parse(input: ParseStream) -> Result<Self> {
        let parser = Punctuated::<GenOpSingle, Token![,]>::parse_terminated;
        Ok(Self {
            types: input.call(parser)?,
        })
    }
}

struct OpRule {
    rule_type: Ident,
    defs: Punctuated<Ident, Token![,]>,
}

impl Parse for OpRule {
    fn parse(input: ParseStream) -> Result<Self> {
        let rule_type: Ident = input.parse()?;
        let _ = input.parse::<Token![:]>()?;
        let parser = Punctuated::<Ident, Token![,]>::parse_separated_nonempty;
        let defs = input.call(parser)?;

        Ok(Self { rule_type, defs })
    }
}

#[proc_macro]
pub fn gen_ops(input: TokenStream) -> TokenStream {
    let GenOps { types } = parse_macro_input!(input as GenOps);

    let mut unary = HashMap::new();
    let mut binary = HashMap::new();
    let mut custom = HashMap::new();
    let mut override_maker = HashSet::new();

    for GenOpSingle { reg_type, brace_token: _, rules } in types.into_iter() {
        for rule in rules.into_iter() {
            let ru = &rule.rule_type;
            match rule.rule_type.to_string().as_ref() {
                "unary" => {
                    unary.extend(rule.defs.into_iter().zip(repeat(reg_type.clone())));
                }
                "binary" => {
                    binary.extend(rule.defs.into_iter().zip(repeat(reg_type.clone())));
                }
                "override_maker" => {
                    override_maker.extend(rule.defs.into_iter());
                }
                "custom" => {
                    let def: Vec<_> = rule.defs.into_iter().collect();
                    if def.len() < 2 {
                        ru.span()
                            .unwrap()
                            .error("not enough operands for custom rule: <mnemonic> <op>[, <op>,...}")
                            .emit();
                        return TokenStream::new();
                    }
                    custom.insert(def, reg_type.clone());
                }
                _ => {
                    ru.span()
                        .unwrap()
                        .error("unknown rule when defining op")
                        .emit();
                    return TokenStream::new();
                }
            }
        }
    }

    let customs = custom
        .iter()
        .map(|(v, t)| {
            let mnemonic = &v[0];
            let iter = v.iter().skip(1);
            quote! {
                #mnemonic { #( #iter: #t ),* },
            }
        })
        .into_iter();
    let custom_makers = custom
        .iter()
        .filter(|(v, _)| {
            let mnemonic = &v[0];

            !override_maker.contains(mnemonic)
        })
        .map(|(v, t)| {
            let mnemonic = &v[0];
            let fn_name = format_ident!("make_{}", v[0]);
            let params = v.iter().skip(1);
            let args = params.clone();
            quote! {
                impl Op {
                    pub fn #fn_name(#( #params: #t ),*) -> Self {
                        Self::#mnemonic { #( #args ),* }
                    }
                }
            }
        })
        .into_iter();

    let unaries = unary
        .iter()
        .map(|(m, t)| quote! {
                #m { rd: #t, rs1: #t },
            }
        )
        .into_iter();
    let unary_makers = unary
        .iter()
        .filter(|(u, _)| !override_maker.contains(u))
        .map(|(m, t)| {
            let fn_name = format_ident!("make_{}", m);
            quote! {
                impl Op {
                    pub fn #fn_name(rd: #t, rs1: #t) -> Self {
                        Self::#m { rd, rs1 }
                    }
                }
            }
        })
        .into_iter();

    let binaries = binary
        .iter()
        .map(|(m, t)| quote! {
                #m { rd: #t, rs1: #t, rs2: #t },
            }
        )
        .into_iter();
    let binary_makers = binary
        .iter()
        .filter(|(b, _)| !override_maker.contains(b))
        .map(|(m, t)| {
            let fn_name = format_ident!("make_{}", m);
            quote! {
                impl Op {
                    pub fn #fn_name(rd: #t, rs1: #t, rs2: #t) -> Self {
                        Self::#m { rd, rs1, rs2 }
                    }
                }
            }
        })
        .into_iter();

    let expanded = quote! {
        pub enum Op {
            #( #unaries )*
            #( #binaries )*
            #( #customs )*
        }

        #( #unary_makers )*
        #( #binary_makers )*
        #( #custom_makers )*
    };

    TokenStream::from(expanded)
}
