//! Main transformation logic for the #[aspect] and #[async_aspect] attribute macros.

use proc_macro2::TokenStream;
use syn::{spanned::Spanned, Expr, ItemFn, Result};

use crate::codegen::{generate_aspect_wrapper, generate_async_aspect_wrapper};
use crate::parsing::AspectInfo;

/// Transforms a function by applying synchronous aspect weaving.
pub fn transform(aspect_expr: Expr, func: ItemFn) -> Result<TokenStream> {
    let aspect_info = AspectInfo::parse(aspect_expr)?;
    Ok(generate_aspect_wrapper(&aspect_info, &func))
}

/// Transforms an async function by applying asynchronous aspect weaving.
pub fn transform_async(aspect_expr: Expr, func: ItemFn) -> Result<TokenStream> {
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            func.sig.fn_token.span(),
            "#[async_aspect] can only be applied to async fn",
        ));
    }

    let aspect_info = AspectInfo::parse(aspect_expr)?;
    Ok(generate_async_aspect_wrapper(&aspect_info, &func))
}
