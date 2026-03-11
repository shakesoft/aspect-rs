//! # aspect-macros
//!
//! Procedural macros for aspect-oriented programming in Rust.
//!
//! This crate provides the `#[aspect]` and `#[async_aspect]` attribute macros
//! that enable aspect weaving at compile time.

use proc_macro::TokenStream;
use syn::{parse_macro_input, Expr, ItemFn};

mod advice_macro;
mod aspect_attr;
mod codegen;
mod parsing;

/// Applies a synchronous aspect to a function.
#[proc_macro_attribute]
pub fn aspect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let aspect_expr = parse_macro_input!(attr as Expr);
    let func = parse_macro_input!(item as ItemFn);

    aspect_attr::transform(aspect_expr, func)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Applies an asynchronous aspect to an async function.
#[proc_macro_attribute]
pub fn async_aspect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let aspect_expr = parse_macro_input!(attr as Expr);
    let func = parse_macro_input!(item as ItemFn);

    aspect_attr::transform_async(aspect_expr, func)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Registers an aspect with a pointcut pattern for declarative aspect application.
#[proc_macro_attribute]
pub fn advice(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as advice_macro::AdviceArgs);
    let func = parse_macro_input!(item as ItemFn);

    advice_macro::transform(args, func)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
