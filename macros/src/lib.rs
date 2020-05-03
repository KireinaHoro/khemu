#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use std::iter::repeat;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input, token, Expr, Ident, Result, Token};

struct GenOpSingle {
    reg_type: Expr,
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
    let mut convert = HashMap::new();
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
                "convert" => {
                    convert.extend(rule.defs.into_iter().zip(repeat(reg_type.clone())));
                }
                "binary" => {
                    binary.extend(rule.defs.into_iter().zip(repeat(reg_type.clone())));
                }
                "override_maker" => {
                    override_maker.extend(rule.defs.into_iter());
                }
                "custom" => {
                    let def: Vec<_> = rule.defs.into_iter().collect();
                    if def.len() < 1 {
                        ru.span()
                            .unwrap()
                            .error(
                                "not enough operands for custom rule: <mnemonic> [<op>, <op>,...]",
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
        .map(|(v, _)| {
            let mnemonic = &v[0];
            let iter = v.iter().skip(1);
            quote! {
                #mnemonic { #( #iter: ::std::rc::Rc<crate::ir::storage::KHVal<R>> ),* },
            }
        })
        .into_iter();
    let custom_makers = custom
        .iter()
        .map(|(v, t)| {
            let mnemonic = &v[0];
            let lower = mnemonic.to_string().to_lowercase();
            let fn_name = if !override_maker.contains(mnemonic) {
                format_ident!("push_{}", lower)
            } else {
                format_ident!("_push_{}", lower)
            };
            let params = v.iter().skip(1);
            let aa = params.clone();
            let bb = params.clone();
            quote! {
                impl<R: crate::ir::storage::HostStorage> Op<R> {
                    pub fn #fn_name<C: crate::guest::DisasContext<R> + crate::guest::Disassembler<R>>(
                        ctx: &mut C,
                        #( #params: &::std::rc::Rc<crate::ir::storage::KHVal<R>> ),*) {
                        // we enforce all arguments to be of the declared type
                        #( assert_eq!(#aa.ty, #t); )*
                        ctx.push_op(Self::#mnemonic { #( #bb: ::std::rc::Rc::clone(#bb) ),* })
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
            let placeholder = params.clone().map(|_| "{}").collect::<Vec<_>>().join(", ");
            quote! {
                Self::#mnemonic { #( #params ),* } => {
                    write!(f, "{}\t", #_lower)?;
                    write!(f, #placeholder, #( #args ),*)?;
                    Ok(())
                },
            }
        })
        .into_iter();

    let unaries = unary
        .iter()
        .map(|(m, _)| {
            quote! {
                #m {
                    rd: ::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                    rs1: ::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                },
            }
        })
        .into_iter();
    let unary_makers = unary
        .iter()
        .map(|(m, t)| {
            let lower = m.to_string().to_lowercase();
            let fn_name = if !override_maker.contains(m) {
                format_ident!("push_{}", lower)
            } else {
                format_ident!("_push_{}", lower)
            };
            quote! {
                impl<R: crate::ir::storage::HostStorage> Op<R> {
                    pub fn #fn_name<C: crate::guest::DisasContext<R> + crate::guest::Disassembler<R>>(
                        ctx: &mut C,
                        rd: &::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                        rs1: &::std::rc::Rc<crate::ir::storage::KHVal<R>>) {
                        // we enforce all arguments to be of the declared type
                        assert_eq!(rd.ty, #t);
                        assert_eq!(rs1.ty, #t);
                        ctx.push_op(Self::#m {
                            rd: ::std::rc::Rc::clone(rd),
                            rs1: ::std::rc::Rc::clone(rs1),
                        })
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
                    write!(f, "{}\t{}, {}", #_lower, rd, rs1)?;
                    Ok(())
                },
            }
        })
        .into_iter();

    let converts = convert
        .iter()
        .map(|(m, _)| {
            quote! {
                #m {
                    rd: ::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                    rs: ::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                },
            }
        })
        .into_iter();
    let convert_makers = convert
        .iter()
        .map(|(m, t)| {
            let lower = m.to_string().to_lowercase();
            let fn_name = if !override_maker.contains(m) {
                format_ident!("push_{}", lower)
            } else {
                format_ident!("_push_{}", lower)
            };
            quote! {
                impl<R: crate::ir::storage::HostStorage> Op<R> {
                    pub fn #fn_name<C: crate::guest::DisasContext<R> + crate::guest::Disassembler<R>>(
                        ctx: &mut C,
                        rd: &::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                        rs: &::std::rc::Rc<crate::ir::storage::KHVal<R>>) {
                        // we enforce rd to be the type declared
                        assert_eq!(rd.ty, #t);
                        ctx.push_op(Self::#m {
                            rd: ::std::rc::Rc::clone(rd),
                            rs: ::std::rc::Rc::clone(rs),
                        })
                    }
                }
            }
        })
        .into_iter();
    let convert_display = convert
        .iter()
        .map(|(m, _)| {
            let _lower = m.to_string().to_lowercase();
            quote! {
                Self::#m { rd, rs } => {
                    write!(f, "{}\t{}, {}", #_lower, rd, rs)?;
                    Ok(())
                },
            }
        })
        .into_iter();

    let binaries = binary
        .iter()
        .map(|(m, _)| {
            quote! {
                #m {
                    rd: ::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                    rs1: ::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                    rs2: ::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                },
            }
        })
        .into_iter();
    let binary_makers = binary
        .iter()
        .map(|(m, t)| {
            let lower = m.to_string().to_lowercase();
            let fn_name = if !override_maker.contains(m) {
                format_ident!("push_{}", lower)
            } else {
                format_ident!("_push_{}", lower)
            };
            quote! {
                impl<R: crate::ir::storage::HostStorage> Op<R> {
                    pub fn #fn_name<C: crate::guest::DisasContext<R> + crate::guest::Disassembler<R>>(
                        ctx: &mut C,
                        rd: &::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                        rs1: &::std::rc::Rc<crate::ir::storage::KHVal<R>>,
                        rs2: &::std::rc::Rc<crate::ir::storage::KHVal<R>>) {
                        // we enforce all arguments to be of the declared type
                        assert_eq!(rd.ty, #t);
                        assert_eq!(rs1.ty, #t);
                        assert_eq!(rs2.ty, #t);
                        ctx.push_op(Self::#m {
                            rd: ::std::rc::Rc::clone(rd),
                            rs1: ::std::rc::Rc::clone(rs1),
                            rs2: ::std::rc::Rc::clone(rs2),
                        })
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
                    write!(f, "{}\t{}, {}, {}", #_lower, rd, rs1, rs2)?;
                    Ok(())
                },
            }
        })
        .into_iter();

    let expanded = quote! {
        #[derive(Debug)]
        pub enum Op<R: crate::ir::storage::HostStorage> {
            #( #unaries )*
            #( #converts )*
            #( #binaries )*
            #( #customs )*
        }

        impl<R: crate::ir::storage::HostStorage> ::std::fmt::Display for Op<R> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                match self {
                    #( #unary_display )*
                    #( #convert_display )*
                    #( #binary_display )*
                    #( #custom_display )*
                }
            }
        }

        #( #unary_makers )*
        #( #convert_makers )*
        #( #binary_makers )*
        #( #custom_makers )*
    };

    TokenStream::from(expanded)
}
