#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use std::iter::repeat;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input, token, Ident, Result, Token, Type};

struct GenOpSingle {
    reg_type: Type,
    _brace: token::Brace,
    rules: Punctuated<OpRule, Token![;]>,
}

impl Parse for GenOpSingle {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            reg_type: input.parse()?,
            _brace: braced!(content in input),
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

    for GenOpSingle {
        reg_type,
        _brace: _,
        rules,
    } in types.into_iter()
    {
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
                            .error(
                                "not enough operands for custom rule: <mnemonic> <op>[, <op>,...}",
                            )
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
                #mnemonic { #( #iter: &'a #t ),* },
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
            let lower = mnemonic.to_string().to_lowercase();
            let fn_name = format_ident!("make_{}", lower);
            let params = v.iter().skip(1);
            let args = params.clone();
            quote! {
                impl<'a> Op<'a> {
                    pub fn #fn_name(#( #params: &'a #t ),*) -> Self {
                        Self::#mnemonic { #( #args ),* }
                    }
                }
            }
        })
        .into_iter();
    let custom_display = custom
        .iter()
        .map(|(v, _)| {
            let mnemonic = &v[0];
            let _lower = mnemonic.to_string().to_lowercase();
            let params = v.iter().skip(1);
            let args = params.clone();
            quote! {
                Self::#mnemonic { #( #params ),* } => {
                    write!(f, "{}\t", #_lower);
                    #( write!(f, " {}", #args); )*
                    Ok(())
                },
            }
        })
        .into_iter();

    let unaries = unary
        .iter()
        .map(|(m, t)| {
            quote! {
                #m { rd: &'a #t, rs1: &'a #t },
            }
        })
        .into_iter();
    let unary_makers = unary
        .iter()
        .filter(|(u, _)| !override_maker.contains(u))
        .map(|(m, t)| {
            let lower = m.to_string().to_lowercase();
            let fn_name = format_ident!("make_{}", lower);
            quote! {
                impl<'a> Op<'a> {
                    pub fn #fn_name(rd: &'a #t, rs1: &'a #t) -> Self {
                        Self::#m { rd, rs1 }
                    }
                }
            }
        })
        .into_iter();
    let unary_display = unary
        .iter()
        .map(|(m, _)| {
            let _lower = m.to_string().to_lowercase();
            quote! {
                Self::#m { rd, rs1 } => {
                    write!(f, "{}\t {} {}", #_lower, rd, rs1);
                    Ok(())
                },
            }
        })
        .into_iter();

    let binaries = binary
        .iter()
        .map(|(m, t)| {
            quote! {
                #m { rd: &'a #t, rs1: &'a #t, rs2: &'a #t },
            }
        })
        .into_iter();
    let binary_makers = binary
        .iter()
        .filter(|(b, _)| !override_maker.contains(b))
        .map(|(m, t)| {
            let lower = m.to_string().to_lowercase();
            let fn_name = format_ident!("make_{}", lower);
            quote! {
                impl<'a> Op<'a> {
                    pub fn #fn_name(rd: &'a #t, rs1: &'a #t, rs2: &'a #t) -> Self {
                        Self::#m { rd, rs1, rs2 }
                    }
                }
            }
        })
        .into_iter();
    let binary_display = binary
        .iter()
        .map(|(m, _)| {
            let _lower = m.to_string().to_lowercase();
            quote! {
                Self::#m { rd, rs1, rs2 } => {
                    write!(f, "{}\t {} {} {}", #_lower, rd, rs1, rs2);
                    Ok(())
                },
            }
        })
        .into_iter();

    let expanded = quote! {
        #[derive(Debug)]
        pub enum Op<'a> {
            #( #unaries )*
            #( #binaries )*
            #( #customs )*
        }

        impl<'a> ::std::fmt::Display for Op<'a> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                match self {
                    #( #unary_display )*
                    #( #binary_display )*
                    #( #custom_display )*
                }
            }
        }

        #( #unary_makers )*
        #( #binary_makers )*
        #( #custom_makers )*
    };

    TokenStream::from(expanded)
}
